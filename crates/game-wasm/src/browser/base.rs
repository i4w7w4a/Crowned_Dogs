//! Base and fitting-room navigation, catalogue interaction and asynchronous viewer lifecycle.
use super::{
    base_region::{self, Region},
    capsule, creation,
    dom_support::*,
    platform,
    scroll::BrowserScroll,
};
use js_sys::{Array, Promise};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use wasm_bindgen::{closure::Closure, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};

/// Home buttons that stay still while the camera travels.
const HOME_BUTTONS: [&str; 6] = [".cd-enter", ".cd-to-fitting", ".cd-to-sync", ".cd-to-persona", ".cd-to-region", ".cd-operator"];
struct Drift {
    home: JsValue,
    image: JsValue,
    rings: Vec<JsValue>,
    ratio: f64,
    flat: bool,
    still: bool,
    lean: JsValue,
    want: JsValue,
}
struct Base {
    host: JsValue,
    root: JsValue,
    home: JsValue,
    fitting: JsValue,
    canvas: JsValue,
    /// The base of the saved map's region, and the listeners on the Persona canvas, which a
    /// region switch replaces.
    region: &'static Region,
    canvas_listeners: Vec<Listener>,
    dialog: JsValue,
    catalog: JsValue,
    status: JsValue,
    loadout: JsValue,
    view: String,
    active: bool,
    selected: String,
    weapon_return: Option<(String, f64)>,
    origin: JsValue,
    viewer: JsValue,
    item_viewer: JsValue,
    workshop: JsValue,
    depth: JsValue,
    depth_ready: bool,
    home_stage: JsValue,
    /// The oblast map while the region screen is open.
    region_map: JsValue,
    home_ready: bool,
    home_motion: Option<(String, f64)>,
    load_attempted: bool,
    model_ready: bool,
    scroll: BrowserScroll,
    /// The creation options' own glass scroll.
    create_scroll: BrowserScroll,
    observer: JsValue,
    listeners: Vec<Listener>,
    callbacks: Vec<Closure<dyn FnMut(JsValue)>>,
    changed: Option<Closure<dyn FnMut(JsValue) -> JsValue>>,
    back: Option<Closure<dyn FnMut()>>,
    frame: Option<Closure<dyn FnMut(f64)>>,
    frame_id: JsValue,
    drifts: Vec<Drift>,
    /// The base's moving pictures need no more than 60 frames a second, even on a 120 Hz phone.
    pace: super::arena::FramePace,
    on_battle: JsValue,
    /// Touch long-press on a slot button: pending timer, and whether it fired.
    hold_timer: JsValue,
    held: bool,
    /// Floating description card for gear under the cursor on the figure, and the last pointer spot.
    tip: JsValue,
    pointer: (f64, f64, bool),
    /// The look being edited on the creation panel, and the one saved on this device (null before the first).
    look: JsValue,
    saved: JsValue,
    /// Briefing line shown before the first Persona, the creation step, and the camera framing.
    brief: usize,
    step: usize,
    focus: String,
    disposed: bool,
}
impl Base {
    fn slot_name(&self, slot: &str) -> String {
        text(&get(&get(&self.host, "GEAR_SLOT_NAMES"), slot))
    }
    fn item(&self, id: &JsValue) -> JsValue {
        if exists(id) && !text(id).is_empty() {
            call(&get(&self.host, "ITEM_BY_ID"), "get", &[id.clone()])
        } else {
            JsValue::NULL
        }
    }
    fn equipped(&self, slot: &str) -> JsValue {
        rust("client.equippedItemId", &[self.loadout.clone(), js(slot)])
    }
    fn navigate(&mut self, view: &str, weak: Weak<RefCell<Self>>) {
        call(
            &get(&global(), "history"),
            "replaceState",
            &[JsValue::NULL, js(""), js(&format!("#{view}"))],
        );
        self.show(view, weak);
    }
    fn show(&mut self, view: &str, weak: Weak<RefCell<Self>>) {
        // No Persona yet: the contract and the creation panel come before anything else.
        let view = if exists(&self.saved) { view } else { "persona" };
        self.reset_home();
        call(&get(&self.host, "gameAudio"), "screen", &[js(view)]);
        if boolean(&self.dialog, "open") {
            call(&self.dialog, "close", &[]);
        }
        let was = std::mem::replace(&mut self.view, view.into());
        hidden(&self.home, view != "base");
        hidden(&query(&self.root, ".cd-sync"), view != "sync");
        if view == "sync" {
            self.render_sync(None);
        }
        hidden(&self.fitting, !self.staged());
        self.region_map(view == "region");
        toggle(&self.fitting, "cd-creating", view == "persona");
        if view != "persona" {
            // The first creation hides «Назад»; the fitting room needs it back.
            toggle(&self.fitting, "cd-first", false);
        }
        if view == "persona" {
            if was != "persona" {
                self.step = 0;
            }
            self.open_create();
        } else {
            hidden(&query(&self.fitting, ".cd-briefing"), true);
            hidden(&query(&self.fitting, ".cd-create"), true);
            call_optional(&get(&self.host, "gameAudio"), "stopVoice", &[]);
        }
        if view != "persona" && was == "persona" {
            // Leaving without «Готово» drops the unsaved choices.
            self.look = self.saved.clone();
            call_optional(&self.viewer, "setLook", &[self.look.clone()]);
        }
        call_optional(&self.viewer, "setCreating", &[flag(view == "persona")]);
        call_optional(&self.viewer, "setBare", &[flag(view == "persona")]);
        if view == "weapon" {
            call(&self.workshop, "show", &[self.loadout.clone()]);
        } else {
            call(&self.workshop, "hide", &[]);
        }
        if self.staged() {
            self.ensure_viewer(weak);
            self.resize();
        }
        call_optional(
            &self.viewer,
            "setActive",
            &[flag(self.active && self.staged())],
        );
    }
    /// Fills the tablet. Without `before` the tablet is raised and its screen boots; after a spend only
    /// the changed stat moves.
    fn render_sync(&self, before: Option<&serde_json::Value>) {
        let body = query(&self.root, ".cd-sync-body");
        html(&body, &capsule::markup(before));
        if before.is_none() {
            set(&body, "scrollTop", &number(0.0));
            let tablet = query(&self.root, ".cd-tab");
            toggle(&tablet, "cd-tab-off", false);
            toggle(&tablet, "cd-tab-up", false);
            get(&tablet, "offsetWidth");
            toggle(&tablet, "cd-tab-up", true);
            // Escape needs focus inside the tablet; the body also scrolls from the keyboard.
            focus(&body);
        }
    }
    /// The tablet's power key: the screen shuts off and the tablet goes down, then the base comes back.
    /// Without motion, or on a second press while it shuts off, the base comes back at once.
    fn close_sync(&mut self, weak: Weak<RefCell<Self>>) {
        if self.view != "sync" {
            return;
        }
        let tablet = query(&self.root, ".cd-tab");
        let off = call(&get(&tablet, "classList"), "contains", &[js("cd-tab-off")]).as_bool()
            == Some(true);
        let reduce = boolean(
            &call(
                &global(),
                "matchMedia",
                &[js("(prefers-reduced-motion: reduce)")],
            ),
            "matches",
        );
        if off || reduce {
            self.leave_sync(weak);
        } else {
            toggle(&tablet, "cd-tab-off", true);
        }
    }
    fn leave_sync(&mut self, weak: Weak<RefCell<Self>>) {
        if self.view != "sync" {
            return;
        }
        self.navigate("base", weak);
        toggle(&query(&self.root, ".cd-tab"), "cd-tab-off", false);
        focus(&query(&self.home, ".cd-to-sync"));
    }
    /// Views that show the Persona on its stand.
    fn staged(&self) -> bool {
        self.view == "fitting" || self.view == "persona"
    }
    /// The instructor's briefing until the contract is signed, then the creation steps.
    fn open_create(&mut self) {
        let first = !exists(&self.saved);
        toggle(&self.fitting, "cd-first", first);
        if !first {
            self.brief = creation::BRIEFING.len();
        }
        if first && text(&get(&self.look, "callsign")).is_empty() {
            set(&self.look, "callsign", &js(creation::DEFAULT_CALLSIGN));
        }
        let briefing = self.brief < creation::BRIEFING.len();
        hidden(&query(&self.fitting, ".cd-briefing"), !briefing);
        hidden(&query(&self.fitting, ".cd-create"), briefing);
        if briefing {
            self.render_briefing();
        } else {
            call_optional(&get(&self.host, "gameAudio"), "stopVoice", &[]);
            attr(&query(&self.fitting, ".cd-create-options"), "data-step", "");
            let face = creation::STEPS[self.step].2;
            self.set_focus(if face { "face" } else { "body" });
            self.render_create();
        }
    }
    fn render_briefing(&self) {
        let (line, button) = creation::BRIEFING[self.brief];
        let tag = text(&get(&self.look, "tag"));
        html(
            &query(&self.fitting, ".cd-briefing-line"),
            &creation::spoken(&line.replace("{tag}", &tag)),
        );
        // Sedoy reads the line aloud: voice.sedoy.1..6, see tools/audio/build_voice.py.
        call_optional(
            &get(&self.host, "gameAudio"),
            "voice",
            &[js(&format!("voice.sedoy.{}", self.brief + 1))],
        );
        content(&query(&self.fitting, ".cd-briefing-next"), button);
        let contract = query(&self.fitting, ".cd-contract");
        hidden(
            &contract,
            self.brief < creation::SIGN_LINE || self.brief > creation::SIGN_LINE + 1,
        );
        toggle(&contract, "cd-signed", !tag.is_empty());
        content(
            &query(&contract, ".cd-contract-tag b"),
            if tag.is_empty() { "K-···-···" } else { &tag },
        );
        let callup = tag.get(2..5).and_then(|n| n.parse::<u32>().ok());
        content(
            &query(&contract, ".cd-contract-tag span"),
            &callup.map_or("Жетон".into(), |n| format!("Жетон · призыв {n}")),
        );
        hidden(
            &query(&self.fitting, ".cd-briefing-skip"),
            self.brief >= creation::SIGN_LINE,
        );
        let dots: String = (0..creation::BRIEFING.len())
            .map(|i| format!("<i class=\"{}\"></i>", if i <= self.brief { "on" } else { "" }))
            .collect();
        html(&query(&self.fitting, ".cd-briefing-dots"), &dots);
        call(&self.host, "liquidGlass", &[query(&self.fitting, ".cd-briefing")]);
    }
    fn brief_next(&mut self) {
        if self.brief == creation::SIGN_LINE && text(&get(&self.look, "tag")).is_empty() {
            set(&self.look, "tag", &creation::issue_tag());
        }
        self.brief += 1;
        if self.brief >= creation::BRIEFING.len() {
            self.step = 0;
            self.open_create();
        } else {
            self.render_briefing();
        }
    }
    fn brief_skip(&mut self) {
        self.brief = self.brief.max(creation::SIGN_LINE);
        self.render_briefing();
    }
    fn render_create(&self) {
        let (id, _, _) = creation::STEPS[self.step];
        let tabs = query(&self.fitting, ".cd-create-steps");
        html(&tabs, &creation::tabs(self.step, text(&get(&self.look, "sex")) == "male"));
        let options = query(&self.fitting, ".cd-create-options");
        let beards_ready = get(&self.viewer, "getBeardReady").is_function()
            && call(&self.viewer, "getBeardReady", &[]).as_bool().unwrap_or(false);
        html(&options, &creation::step_markup_with_beards(&self.look, self.step, beards_ready));
        // A new step starts at the top of its choices; a pick keeps the place.
        if data(&options, "step") != id {
            attr(&options, "data-step", id);
            set(&options, "scrollTop", &number(0.0));
        }
        let naming = id == "callsign";
        hidden(&query(&self.fitting, ".cd-create-name"), !naming);
        // The options sit in the game's own scroll frame; the frame goes with them.
        hidden(&get(&options, "parentNode"), naming);
        hidden(&query(&self.fitting, ".cd-create-error"), true);
        set(
            &query(&self.fitting, ".cd-callsign-input"),
            "value",
            &get(&self.look, "callsign"),
        );
        content(
            &query(&self.fitting, ".cd-create-tag"),
            &text(&get(&self.look, "tag")),
        );
        let (label, icon) = if exists(&self.saved) {
            ("Сохранить", "save")
        } else if self.step + 1 == creation::STEPS.len() {
            ("Готово", "save")
        } else {
            ("Дальше", "next")
        };
        html(
            &query(&self.fitting, ".cd-create-next"),
            &format!("<span>{label}</span>{}", creation::icon(icon)),
        );
        call(&self.host, "liquidGlass", &[query(&self.fitting, ".cd-create")]);
        call(&self.host, "liquidGlass", &[tabs]);
    }
    /// The typed callsign survives re-renders and step changes.
    fn keep_typed(&self) {
        set(
            &self.look,
            "callsign",
            &get(&query(&self.fitting, ".cd-callsign-input"), "value"),
        );
    }
    fn go_step(&mut self, step: usize) {
        self.keep_typed();
        self.step = step.min(creation::STEPS.len() - 1);
        let face = creation::STEPS[self.step].2;
        self.set_focus(if face { "face" } else { "body" });
        self.render_create();
        if creation::STEPS[self.step].0 == "callsign" {
            focus(&query(&self.fitting, ".cd-callsign-input"));
        }
    }
    fn set_focus(&mut self, focus: &str) {
        self.focus = focus.into();
        toggle(&self.fitting, "cd-face", focus == "face");
        for button in all(&self.fitting, "[data-focus]") {
            let on = data(&button, "focus") == focus;
            attr(&button, "aria-pressed", if on { "true" } else { "false" });
        }
        call_optional(&self.viewer, "setFocus", &[js(focus)]);
    }
    fn pick_look(&mut self, key: &str, value: &str) {
        self.keep_typed();
        self.look = creation::pick(&self.look, key, value);
        self.render_create();
        call_optional(&self.viewer, "setLook", &[self.look.clone()]);
    }
    fn create_next(&mut self, weak: Weak<RefCell<Self>>) {
        if exists(&self.saved) || self.step + 1 >= creation::STEPS.len() {
            self.finish_create(weak);
        } else {
            self.go_step(self.step + 1);
        }
    }
    fn finish_create(&mut self, weak: Weak<RefCell<Self>>) {
        self.keep_typed();
        let name = rust("client.look.callsign", &[get(&self.look, "callsign")]);
        if !boolean(&name, "ok") {
            let last = creation::STEPS.len() - 1;
            if self.step != last {
                self.go_step(last);
            }
            let error = query(&self.fitting, ".cd-create-error");
            content(&error, &text(&get(&name, "error")));
            hidden(&error, false);
            focus(&query(&self.fitting, ".cd-callsign-input"));
            return;
        }
        set(&self.look, "callsign", &get(&name, "callsign"));
        if !platform::save_local_look(self.look.clone()) {
            let error = query(&self.fitting, ".cd-create-error");
            content(&error, "Не удалось сохранить Персону на этом устройстве.");
            hidden(&error, false);
            return;
        }
        self.saved = platform::load_local_look();
        self.look = self.saved.clone();
        self.refresh_operator();
        self.navigate("base", weak);
    }
    fn refresh_operator(&self) {
        content(
            &query(&self.home, ".cd-callsign"),
            &text(&get(&self.saved, "callsign")),
        );
        content(&query(&self.home, ".cd-tag"), &text(&get(&self.saved, "tag")));
    }
    fn active(&mut self, active: bool) {
        if !active {
            self.home_motion = None;
        }
        self.active = active;
        hidden(&self.root, !active);
        if !active && boolean(&self.dialog, "open") {
            call(&self.dialog, "close", &[]);
        }
        if self.view == "weapon" {
            if active {
                call(&self.workshop, "show", &[self.loadout.clone()]);
            } else {
                call(&self.workshop, "hide", &[]);
            }
        }
        call_optional(
            &self.viewer,
            "setActive",
            &[flag(active && self.staged())],
        );
        if active {
            self.resize();
        }
    }
    fn resize(&self) {
        // The canvas, not its stage: while a look is made it fills the whole screen.
        let w = num(&self.canvas, "clientWidth");
        let h = num(&self.canvas, "clientHeight");
        if w > 0.0 && h > 0.0 {
            call_optional(&self.viewer, "resize", &[number(w), number(h)]);
        }
    }
    fn ensure_viewer(&mut self, weak: Weak<RefCell<Self>>) {
        if self.load_attempted {
            return;
        }
        self.load_attempted = true;
        let result =
            construct(&self.host, "PersonaViewer", &[self.canvas.clone()]).and_then(|viewer| {
                self.viewer = viewer;
                call(&self.viewer, "setLook", &[self.look.clone()]);
                call(&self.viewer, "setBare", &[flag(self.view == "persona")]);
                call(&self.viewer, "setCreating", &[flag(self.view == "persona")]);
                call(&self.viewer, "setFocus", &[js(&self.focus)]);
                call(&self.viewer, "updateLoadout", &[self.loadout.clone()]);
                self.resize();
                invoke(&self.viewer, "load", &[])
            });
        let viewer = self.viewer.clone();
        match result {
            Ok(promise) => spawn_local(async move {
                let result = JsFuture::from(Promise::resolve(&promise)).await;
                let Some(state) = weak.upgrade() else {
                    return;
                };
                let mut state = state.borrow_mut();
                // A region switch may have replaced the viewer during the load.
                if state.disposed || state.viewer != viewer {
                    return;
                }
                if result.is_ok() {
                    state.model_ready = true;
                    hidden(&state.status, true);
                    state.refresh();
                    state.resize();
                    call_optional(
                        &state.viewer,
                        "setActive",
                        &[flag(state.active && state.staged())],
                    );
                } else {
                    content(
                        &state.status,
                        "3D-модель не загрузилась. Каталог снаряжения доступен.",
                    );
                    call_optional(&state.viewer, "setActive", &[flag(false)]);
                }
            }),
            Err(_) => content(
                &self.status,
                "На этом устройстве 3D недоступно. Можно выбрать снаряжение в каталоге.",
            ),
        }
    }
    fn refresh(&self) {
        let main = self.item(&get(&get(&self.loadout, "weapon1"), "item"));
        let name = if exists(&main) {
            text(&get(&main, "name"))
        } else {
            "Оружие".into()
        };
        content(&query(&self.root, "[data-kit-name]"), &name);
        for button in all(&self.root, ".cd-slot[data-slot]") {
            let slot = data(&button, "slot");
            let id = self.equipped(&slot);
            let item = self.item(&id);
            let name = if exists(&item) {
                text(&get(&item, "name"))
            } else {
                "пусто".into()
            };
            html(
                &query(&button, ".cd-slot-art"),
                &text(&call(&self.host, "equipmentIcon", &[js(&slot), id.clone()])),
            );
            attr(
                &button,
                "aria-label",
                &format!("{}: {name}. Выбрать предмет", self.slot_name(&slot)),
            );
            toggle(&button, "cd-slot-empty", id.is_null());
            let details = if exists(&item) {
                let effects: Vec<String> = elements(&rust("describeItem", &[item.clone()]))
                    .into_iter()
                    .filter_map(|v| v.as_string())
                    .collect();
                if effects.is_empty() {
                    text(&get(&item, "note"))
                } else {
                    effects.join(" · ")
                }
            } else {
                "Не экипировано".into()
            };
            content(&query(&button, ".cd-slot-info strong"), &name);
            content(&query(&button, ".cd-slot-info small"), &details);
        }
        let kit = call(&self.host, "compileKit", &[self.loadout.clone()]);
        let armor = get(&kit, "armor");
        let stats = format!(
            "<span>Голова <b>{}</b></span><span>Корпус <b>{}</b></span><span>Ноги <b>{}</b></span>",
            num(&armor, "head"),
            num(&armor, "body"),
            num(&armor, "legs")
        );
        html(&query(&self.root, ".cd-stats"), &stats);
        call_optional(&self.viewer, "updateLoadout", &[self.loadout.clone()]);
    }
    /// Shows the worn item's name, note and effects beside the pointer ("" hides the card).
    fn show_tip(&self, slot: &str) {
        let open = invoke(&self.tip, "matches", &[js(":popover-open")]).is_ok_and(|v| v.is_truthy());
        let item = if slot.is_empty() {
            JsValue::NULL
        } else {
            self.item(&self.equipped(slot))
        };
        if !exists(&item) {
            if open {
                let _ = invoke(&self.tip, "hidePopover", &[]);
            }
            return;
        }
        // The card keeps its slot as data-tip-slot: refresh() reads every
        // .cd-slot[data-slot] under the root as a gear button.
        if data(&self.tip, "tipSlot") != slot || !open {
            set(&get(&self.tip, "dataset"), "tipSlot", &js(slot));
            html(&self.tip, "");
            let add = |tag: &str, value: String| {
                if !value.is_empty() {
                    let el = create(tag, "");
                    content(&el, &value);
                    call(&self.tip, "append", &[el]);
                }
            };
            add("b", text(&get(&item, "name")));
            add("span", text(&get(&item, "note")));
            for line in elements(&rust("describeItem", &[item.clone()])) {
                add("em", line.as_string().unwrap_or_default());
            }
        }
        if !open {
            let _ = invoke(&self.tip, "showPopover", &[]);
        }
        self.place_tip();
    }
    /// Mouse: below-right of the cursor; finger: above it, so the hand does not cover the card.
    fn place_tip(&self) {
        if !invoke(&self.tip, "matches", &[js(":popover-open")]).is_ok_and(|v| v.is_truthy()) {
            return;
        }
        let (x, y, mouse) = self.pointer;
        let vw = num(&global(), "innerWidth");
        let vh = num(&global(), "innerHeight");
        let w = num(&self.tip, "offsetWidth");
        let h = num(&self.tip, "offsetHeight");
        let (mut left, mut top) = if mouse {
            (x + 18.0, y + 18.0)
        } else {
            (x - w / 2.0, y - h - 36.0)
        };
        if left + w > vw - 8.0 {
            left = x - w - 18.0;
        }
        if top + h > vh - 8.0 {
            top = y - h - 18.0;
        }
        style(&self.tip, "left", &format!("{}px", left.clamp(8.0, (vw - w - 8.0).max(8.0))));
        style(&self.tip, "top", &format!("{}px", top.clamp(8.0, (vh - h - 8.0).max(8.0))));
    }
    /// Unfolds one slot card ("" folds them all); a hover on the button also glows the figure.
    fn hot(&self, slot: &str, from_button: bool) {
        for button in all(&self.root, ".cd-slot[data-slot]") {
            toggle(&button, "cd-slot-hot", data(&button, "slot") == slot);
        }
        if from_button {
            call_optional(&self.viewer, "highlight", &[js(slot)]);
        }
    }
    fn cancel_hold(&mut self) {
        if exists(&self.hold_timer) {
            call(&global(), "clearTimeout", &[self.hold_timer.clone()]);
            self.hold_timer = JsValue::NULL;
        }
    }
    fn open_catalog(&mut self, slot: &str, weak: Weak<RefCell<Self>>) {
        if slot == "weapon1" {
            self.weapon_return = None;
            self.navigate("weapon", weak);
            return;
        }
        self.selected = slot.into();
        self.origin = get(&document(), "activeElement");
        self.render_catalog(None);
        call(&self.dialog, "showModal", &[]);
        call_optional(&self.item_viewer, "setActive", &[flag(true)]);
        set(
            &get(&self.catalog, "parentElement"),
            "scrollTop",
            &number(0.0),
        );
        self.scroll.refresh();
        focus(&query(&self.root, ".cd-dialog-close"));
    }
    fn render_catalog(&mut self, preview: Option<JsValue>) {
        let name = self.slot_name(&self.selected);
        content(&query(&self.root, "#cd-catalog-title"), &name);
        content(&query(&self.root, ".cd-select-trigger"), &name);
        for option in all(&self.root, "[data-category]") {
            let selected = data(&option, "category") == self.selected;
            attr(&option, "aria-selected", &selected.to_string());
            toggle(&option, "on", selected);
            set(&option, "tabIndex", &number(-1.0));
        }
        hidden(&query(&self.root, ".cd-catalog-error"), true);
        let selected = self.equipped(&self.selected);
        let available: Vec<_> = elements(&rust("client.itemsForSlot", &[js(&self.selected)]))
            .into_iter()
            .filter(|item| {
                call(&self.host, "availableInFitting", &[item.clone()]).as_bool() == Some(true)
            })
            .collect();
        let choices = Array::new();
        for item in &available {
            let choice = js_sys::Object::new();
            set(&choice, "item", item);
            set(
                &choice,
                "icon",
                &call(&self.host, "equipmentItemIcon", &[get(item, "id")]),
            );
            choices.push(&choice);
        }
        let paint = self.paint_row(&selected);
        html(
            &self.catalog,
            &(paint
                + &text(&rust(
                    "ui.catalog",
                    &[js(&self.selected), selected.clone(), choices.into()],
                ))),
        );
        set(
            &get(&self.catalog, "parentElement"),
            "scrollTop",
            &number(0.0),
        );
        call(&self.host, "liquidGlass", &[self.catalog.clone()]);
        let models = get(&self.host, "ITEM_MODELS");
        let id = preview.unwrap_or_else(|| {
            if exists(&get(&models, &text(&selected))) {
                selected
            } else {
                available
                    .iter()
                    .find(|item| exists(&get(&models, &text(&get(item, "id")))))
                    .map(|item| get(item, "id"))
                    .unwrap_or(JsValue::NULL)
            }
        });
        let preview = query(&self.root, ".cd-item-preview");
        let item = self.item(&id);
        let visible = exists(&id) && exists(&get(&models, &text(&id)));
        hidden(&preview, !visible);
        if exists(&item) && visible {
            content(
                &query(&self.root, ".cd-preview-name"),
                &text(&get(&item, "name")),
            );
            if !exists(&self.item_viewer) {
                self.item_viewer = construct(
                    &self.host,
                    "ItemViewer",
                    &[query(&self.root, ".cd-item-viewport")],
                )
                .unwrap_throw();
            }
            call(
                &self.item_viewer,
                "show",
                &[get(&item, "id"), get(&item, "name"), self.loadout.clone()],
            );
        } else {
            call_optional(&self.item_viewer, "hide", &[]);
        }
        self.scroll.refresh();
    }
    fn picker(&self, open: bool) {
        let options = query(&self.root, ".cd-select-options");
        hidden(&options, !open);
        attr(
            &query(&self.root, ".cd-select-trigger"),
            "aria-expanded",
            &open.to_string(),
        );
        if open {
            call(&self.host, "liquidGlass", &[self.dialog.clone()]);
            focus(&query(
                &options,
                &format!("[data-category=\"{}\"]", self.selected),
            ));
        }
    }
    /// Opens or closes the list of five prepared kits; each option names its main weapon.
    fn kits(&self, open: bool) {
        let options = query(&self.root, ".cd-kit-options");
        hidden(&options, !open);
        attr(
            &query(&self.root, ".cd-kit-trigger"),
            "aria-expanded",
            &open.to_string(),
        );
        let active = platform::active_kit();
        content(
            &query(&self.root, ".cd-kit-current"),
            &(active + 1).to_string(),
        );
        if !open {
            return;
        }
        for option in all(&options, "[data-kit]") {
            let index = data(&option, "kit").parse().unwrap_or(0);
            attr(&option, "aria-selected", &(index == active).to_string());
            let loadout = if index == active {
                self.loadout.clone()
            } else {
                platform::kit_loadout(index)
            };
            let weapon = self.item(&rust("client.equippedItemId", &[loadout, js("weapon1")]));
            content(
                &query(&option, "small"),
                &if exists(&weapon) {
                    text(&get(&weapon, "name"))
                } else {
                    "Без оружия".into()
                },
            );
        }
        call(&self.host, "liquidGlass", &[options.clone()]);
        focus(&query(&options, &format!("[data-kit=\"{active}\"]")));
    }
    fn pick_kit(&mut self, index: usize) {
        if index != platform::active_kit() {
            self.loadout = platform::switch_kit(index);
            hidden(&query(&self.root, ".cd-save-status"), true);
            self.refresh();
        }
        self.kits(false);
        focus(&query(&self.root, ".cd-kit-trigger"));
    }
    /// Opens the oblast map from the comms table, or lets it go with its WebGL context.
    fn region_map(&mut self, open: bool) {
        let section = query(&self.root, ".cd-region");
        if open == exists(&self.region_map) {
            return;
        }
        if open {
            hidden(&section, false);
            match construct(&self.host, "RegionMap", &[section.clone()]) {
                Ok(map) => self.region_map = map,
                Err(err) => {
                    call(&get(&global(), "console"), "error", &[js("crowned-dogs: the region map failed"), err]);
                    hidden(&section, true);
                }
            }
        } else {
            call_optional(&self.region_map, "dispose", &[]);
            self.region_map = JsValue::NULL;
            fresh_canvas(&query(&section, ".cd-region-canvas"));
            hidden(&section, true);
        }
    }
    /// Names the base and puts its pictures and home rings in place.
    fn dress(&self) {
        let region = self.region;
        let base = text(&get(&self.host, "baseUrl"));
        attr(&self.home, "aria-label", &format!("База {}", region.title()));
        content(&query(&self.home, ".cd-home-title h1"), region.base);
        for (image, picture, alt) in [
            (query(&self.home, "img.cd-backdrop"), region.home_picture.to_string(), region.home_alt),
            (query(&self.root, ".cd-sync img.cd-backdrop"), region.home_picture.to_string(), ""),
            (query(&self.fitting, "img.cd-backdrop:not(.cd-create-backdrop)"), region.fitting.to_string(), region.fitting_alt),
            (query(&self.fitting, ".cd-create-backdrop"), format!("{}.webp", region.bay.photo), ""),
        ] {
            let url = format!("{base}img/{picture}");
            if call(&image, "getAttribute", &[js("src")]).as_string().as_deref() != Some(&url) {
                attr(&image, "src", &url);
            }
            attr(&image, "alt", alt);
        }
        for (selector, u, v) in region.rings {
            let ring = query(&self.home, selector);
            attr(&ring, "data-u", &u.to_string());
            attr(&ring, "data-v", &v.to_string());
        }
    }
    /// Lets the moving pictures and the Persona of the old base go: each canvas is swapped for an
    /// empty copy, so the next base's stages start on contexts of their own.
    fn unstage(&mut self) {
        self.home_motion = None;
        self.home_ready = false;
        self.depth_ready = false;
        call_optional(&self.home_stage, "dispose", &[]);
        if get(&self.depth, "dispose").is_function() {
            call(&self.depth, "dispose", &[]);
        }
        call_optional(&self.viewer, "dispose", &[]);
        self.home_stage = JsValue::NULL;
        self.depth = JsValue::NULL;
        self.viewer = JsValue::NULL;
        self.load_attempted = false;
        self.model_ready = false;
        self.show_tip("");
        for canvas in [
            query(&self.home, ".cd-home-projection"),
            query(&self.fitting, ".cd-backdrop-depth"),
        ] {
            let copy = fresh_canvas(&canvas);
            hidden(&copy, true);
            call(&copy, "removeAttribute", &[js("data-depth-base")]);
        }
        hidden(&query(&self.home, "img.cd-backdrop"), false);
        hidden(&query(&self.fitting, "img.cd-backdrop:not(.cd-create-backdrop)"), false);
        self.canvas_listeners.clear();
        call(&self.observer, "unobserve", &[self.canvas.clone()]);
        self.canvas = fresh_canvas(&self.canvas);
        call(&self.observer, "observe", &[self.canvas.clone()]);
        content(&self.status, "Загрузка Персоны…");
        hidden(&self.status, false);
    }
    fn picker_key(&self, event: JsValue) {
        let key = text(&get(&event, "key"));
        let options = query(&self.root, ".cd-select-options");
        let open = !boolean(&options, "hidden");
        let trigger = query(&self.root, ".cd-select-trigger");
        if key == "Escape" && open {
            call(&event, "preventDefault", &[]);
            call(&event, "stopPropagation", &[]);
            self.picker(false);
            focus(&trigger);
            return;
        }
        if key == "Tab" && open {
            focus(&trigger);
            self.picker(false);
            return;
        }
        if !["ArrowDown", "ArrowUp", "Home", "End"].contains(&key.as_str()) {
            return;
        }
        call(&event, "preventDefault", &[]);
        if !open {
            self.picker(true);
            return;
        }
        let items = all(&options, "[data-category]");
        let active = get(&document(), "activeElement");
        let index = items
            .iter()
            .position(|item| *item == active)
            .map(|i| i as f64)
            .unwrap_or(-1.0);
        let next = rust(
            "ui.pickerNext",
            &[js(&key), number(index), number(items.len() as f64)],
        )
        .as_f64()
        .unwrap_or(0.0) as usize;
        if let Some(item) = items.get(next) {
            focus(item);
        }
    }
    /// The worn piece's colour: as the suit, as made, or one of the suit colours.
    fn paint_row(&self, item: &JsValue) -> String {
        if !["head", "body", "suit", "boots"].contains(&self.selected.as_str()) || !exists(item) {
            return String::new();
        }
        let current = text(&rust("client.itemColor", &[self.loadout.clone(), item.clone()]));
        let colors = elements(&rust("client.gearColors", &[]));
        // a suit "as the suit" wears the colour picked for the Persona
        let name = |c: &JsValue| match text(&get(c, "id")).as_str() {
            "suit" if self.selected == "suit" => "Как в Персоне".to_string(),
            _ => text(&get(c, "name")),
        };
        let picked = colors
            .iter()
            .find(|c| text(&get(c, "id")) == current)
            .map(name)
            .unwrap_or_default();
        // the word choices on one row, the colour dots on their own row below
        let (mut words, mut swatches) = (String::new(), String::new());
        for c in &colors {
            let id = text(&get(c, "id"));
            let hex = text(&get(c, "color"));
            if hex.is_empty() {
                words.push_str(&format!(
                    "<button class=\"glass cd-paint-word\" type=\"button\" data-paint=\"{id}\" aria-pressed=\"{}\">{}</button>",
                    id == current,
                    name(c)
                ));
            } else {
                swatches.push_str(&format!(
                    "<button class=\"cd-swatch\" type=\"button\" data-paint=\"{id}\" aria-pressed=\"{on}\" aria-label=\"{label}\" title=\"{label}\" style=\"--swatch:{hex}\"></button>",
                    on = id == current,
                    label = name(c),
                ));
            }
        }
        format!(
            "<div class=\"cd-paint\" role=\"group\" aria-label=\"Цвет\"><span class=\"cd-overline\">Цвет</span><span class=\"cd-create-picked\">{picked}</span><div class=\"cd-paint-row\">{words}</div><div class=\"cd-paint-row\">{swatches}</div></div>"
        )
    }
    fn paint_click(&mut self, button: JsValue) {
        let item = self.equipped(&self.selected);
        let Ok(loadout) = try_rust(
            "client.paintItem",
            &[self.loadout.clone(), item, js(&data(&button, "paint"))],
        ) else {
            return;
        };
        self.loadout = loadout;
        let saved = call(&self.host, "saveLocalLoadout", &[self.loadout.clone()]).as_bool() == Some(true);
        hidden(&query(&self.root, ".cd-save-status"), saved);
        self.refresh();
        self.render_catalog(None);
    }
    fn catalog_click(&mut self, event: JsValue) {
        let paint = closest(&event, "[data-paint]");
        if exists(&paint) {
            self.paint_click(paint);
            return;
        }
        let button = closest(&event, "[data-item]");
        if !exists(&button) {
            return;
        }
        let id = data(&button, "item");
        let id_value = if id.is_empty() {
            JsValue::NULL
        } else {
            js(&id)
        };
        let top = num(&get(&self.catalog, "parentElement"), "scrollTop");
        match try_rust(
            "client.equipItem",
            &[self.loadout.clone(), js(&self.selected), id_value.clone()],
        ) {
            Ok(loadout) => {
                self.loadout = if self.selected == "weapon1" {
                    super::platform::swap_gun(&self.loadout, loadout)
                } else {
                    loadout
                };
                let saved = call(&self.host, "saveLocalLoadout", &[self.loadout.clone()]).as_bool()
                    == Some(true);
                self.refresh();
                self.render_catalog(Some(id_value));
                set(
                    &get(&self.catalog, "parentElement"),
                    "scrollTop",
                    &number(top),
                );
                for element in all(&self.catalog, "[data-item]") {
                    if data(&element, "item") == id {
                        focus(&element);
                        break;
                    }
                }
                let status = query(&self.root, ".cd-save-status");
                hidden(&status, saved);
                content(
                    &status,
                    if saved {
                        ""
                    } else {
                        "Не удалось сохранить комплект. Он будет доступен до закрытия страницы."
                    },
                );
            }
            Err(err) => {
                let message = query(&self.root, ".cd-catalog-error");
                hidden(&message, false);
                content(&message, &error(&err, "Не удалось выбрать предмет"));
            }
        }
    }
    fn reset_home(&mut self) {
        self.home_motion = None;
        toggle(&self.home, "cd-travelling", false);
        attr(&self.home, "aria-busy", "false");
        style(&self.home, "--cd-travel-fade", "0");
        for selector in HOME_BUTTONS {
            set(&query(&self.home, selector), "disabled", &flag(false));
        }
        if self.home_ready {
            call_optional(&self.home_stage, "draw", &[js("base"), number(0.)]);
        }
    }
    fn tick(&mut self, now: f64) -> Option<String> {
        if self.disposed {
            return None;
        }
        if let Some(frame) = &self.frame {
            self.frame_id = call(
                &global(),
                "requestAnimationFrame",
                &[frame.as_ref().clone()],
            );
        }
        if !self.active || !self.pace.due(now) {
            return None;
        }
        let mut completed = None;
        if self.view == "base" && self.home_ready {
            let (target, progress) = if let Some((target, start)) = &self.home_motion {
                let duration = 1200.;
                (target.clone(), ((now - start) / duration).clamp(0., 1.))
            } else {
                ("base".into(), 0.)
            };
            if invoke(&self.home_stage, "draw", &[js(&target), number(progress)]).is_err() {
                self.home_ready = false;
                hidden(&query(&self.home, ".cd-home-projection"), true);
                hidden(&query(&self.home, "img.cd-backdrop"), false);
                completed = self.home_motion.take().map(|x| x.0);
            } else if self.home_motion.is_some() {
                style(
                    &self.home,
                    "--cd-travel-fade",
                    &format!("{}", ((progress - 0.92) / 0.08).clamp(0., 1.)),
                );
                if progress >= 1. {
                    completed = self.home_motion.take().map(|x| x.0);
                }
            }
        }
        if self.view == "region" && exists(&self.region_map) {
            // The map hands back the region once the player moved the base there.
            if let Some(id) = invoke(&self.region_map, "frame", &[number(now)]).ok().and_then(|id| id.as_string()) {
                completed = Some(format!("region:{id}"));
            }
        }
        for drift in &mut self.drifts {
            let w = num(&drift.home, "clientWidth");
            let h = num(&drift.home, "clientHeight");
            if w <= 0.0 {
                continue;
            }
            if drift.flat {
                let iw = (h * drift.ratio).max(w);
                let ih = (w / drift.ratio).max(h);
                for ring in &drift.rings {
                    let u: f64 = data(ring, "u").parse().unwrap_or(0.5);
                    let v: f64 = data(ring, "v").parse().unwrap_or(0.5);
                    style(ring, "left", &format!("{}px", (w - iw) * 0.5 + u * iw));
                    style(ring, "top", &format!("{}px", (h - ih) * 0.5 + v * ih));
                }
                continue;
            }
            let points = Array::new();
            for ring in &drift.rings {
                points.push(&args(&[
                    number(data(ring, "u").parse().unwrap_or(0.0)),
                    number(data(ring, "v").parse().unwrap_or(0.0)),
                ]));
            }
            let result = rust(
                "ui.backdrop",
                &[
                    number(w),
                    number(h),
                    number(drift.ratio),
                    flag(drift.still),
                    number(now),
                    drift.lean.clone(),
                    drift.want.clone(),
                    points.into(),
                ],
            );
            drift.lean = get(&result, "lean");
            let tx = num(&result, "tx");
            let ty = num(&result, "ty");
            let scale = num(&result, "scale");
            if drift.flat {
                style(
                    &drift.image,
                    "transform",
                    &format!("translate({tx:.2}px, {ty:.2}px) scale({scale})"),
                );
            }
            for (i, ring) in drift.rings.iter().enumerate() {
                let point = Array::from(&get(&result, "points")).get(i as u32);
                let point = Array::from(&point);
                style(
                    ring,
                    "left",
                    &format!("{:.1}px", point.get(0).as_f64().unwrap_or(0.0)),
                );
                style(
                    ring,
                    "top",
                    &format!("{:.1}px", point.get(1).as_f64().unwrap_or(0.0)),
                );
            }
            if !drift.flat && self.depth_ready && self.view != "persona" {
                call(&self.depth, "draw", &[number(tx * 2.0), number(ty * 2.0)]);
                call_optional(&self.viewer, "shift", &[number(tx * 0.6), number(ty * 0.6)]);
            }
        }
        completed
    }
    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        // The application's callback captures its route state, which holds this
        // screen's JS wrapper. Break that cross-heap cycle on explicit teardown.
        self.on_battle = JsValue::NULL;
        if exists(&self.frame_id) {
            call(&global(), "cancelAnimationFrame", &[self.frame_id.clone()]);
        }
        call_optional(&self.observer, "disconnect", &[]);
        self.scroll.dispose();
        self.create_scroll.dispose();
        call_optional(&self.viewer, "dispose", &[]);
        call_optional(&self.item_viewer, "dispose", &[]);
        call_optional(&self.workshop, "dispose", &[]);
        self.home_motion = None;
        call_optional(&self.home_stage, "dispose", &[]);
        call_optional(&self.region_map, "dispose", &[]);
        if get(&self.depth, "dispose").is_function() {
            call(&self.depth, "dispose", &[]);
        }
        self.listeners.clear();
        call(&self.root, "remove", &[]);
    }
}
impl Drop for Base {
    fn drop(&mut self) {
        self.dispose();
    }
}

fn initial(host: &JsValue) -> String {
    let names = get(host, "GEAR_SLOT_NAMES");
    let mut columns = String::new();
    for (index, slots) in [
        ["head", "body", "weapon1", "knife"],
        ["suit", "boots", "weapon2", "grenade"],
    ]
    .iter()
    .enumerate()
    {
        columns.push_str(&format!(
            "<div class=\"cd-slot-column cd-slot-column-{index}\" aria-label=\"{}\">",
            if index == 0 {
                "Защита и оружие"
            } else {
                "Костюм и снаряжение"
            }
        ));
        for slot in slots {
            columns.push_str(&format!("<button class=\"glass cd-slot\" type=\"button\" data-slot=\"{slot}\"><span class=\"cd-slot-face\"><span class=\"cd-slot-art\"></span><span class=\"cd-slot-label\">{}</span></span><span class=\"cd-slot-info\" aria-hidden=\"true\"><strong></strong><small></small></span></button>",text(&get(&names,slot))));
        }
        columns.push_str("</div>");
    }
    let mut options = String::new();
    for slot in elements(&get(host, "GEAR_SLOTS")) {
        let slot = text(&slot);
        options.push_str(&format!("<button class=\"glass\" type=\"button\" role=\"option\" data-category=\"{slot}\" aria-selected=\"false\">{}</button>",text(&get(&names,&slot))));
    }
    let mut kits = String::new();
    for index in 0..platform::KITS {
        kits.push_str(&format!("<button class=\"glass\" type=\"button\" role=\"option\" data-kit=\"{index}\" aria-selected=\"false\"><strong>Набор {}</strong><small></small></button>", index + 1));
    }
    let region = base_region::current();
    include_str!("base.html")
        .replace("{{HOME_PICTURE}}", region.home_picture)
        .replace("{{FITTING_PICTURE}}", region.fitting)
        .replace("{{BAY_PICTURE}}", region.bay.photo)
        .replace("{{KIT}}", &(platform::active_kit() + 1).to_string())
        .replace("{{KITS}}", &kits)
        .replace("{{BASE}}", &text(&get(host, "baseUrl")))
        .replace("{{BACK}}", &text(&get(host, "backArrow")))
        .replace("{{FACE}}", &creation::icon("face"))
        .replace("{{BODY}}", &creation::icon("body"))
        .replace("{{COLUMNS}}", &columns)
        .replace("{{OPTIONS}}", &options)
}
#[wasm_bindgen]
pub struct BrowserBase {
    state: Rc<RefCell<Base>>,
}
impl Drop for BrowserBase {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.dispose();
        }
        // Base::drop also handles freeing the exported object from inside an active event.
    }
}
#[wasm_bindgen]
impl BrowserBase {
    #[wasm_bindgen(constructor)]
    pub fn new(root: JsValue, on_battle: JsValue, host: JsValue) -> Result<Self, JsValue> {
        set(&root, "className", &js("cd-shell"));
        html(&root, &initial(&host));
        let home = query(&root, ".cd-home");
        let fitting = query(&root, ".cd-fitting");
        let canvas = query(&root, ".cd-persona-canvas");
        let dialog = query(&root, "dialog");
        let catalog = query(&root, ".cd-catalog-items");
        let status = query(&root, ".cd-model-status");
        let scroll = BrowserScroll::new(query(&root, ".cd-dialog-content"))?;
        let create_scroll = BrowserScroll::new(query(&root, ".cd-create-options"))?;
        let loadout = call(&host, "loadLocalLoadout", &[]);
        let saved = platform::load_local_look();
        let look = if exists(&saved) {
            saved.clone()
        } else {
            platform::current_look()
        };
        let tip = create("div", "cd-tip glass");
        attr(&tip, "role", "tooltip");
        // Top layer: screen transitions transform the root, which would trap a fixed card.
        attr(&tip, "popover", "manual");
        call(&root, "append", &[tip.clone()]);
        let state = Rc::new(RefCell::new(Base {
            host,
            root: root.clone(),
            home: home.clone(),
            fitting: fitting.clone(),
            canvas,
            region: base_region::current(),
            canvas_listeners: vec![],
            dialog: dialog.clone(),
            catalog: catalog.clone(),
            status,
            loadout,
            view: "base".into(),
            active: true,
            selected: "body".into(),
            weapon_return: None,
            origin: JsValue::NULL,
            viewer: JsValue::NULL,
            item_viewer: JsValue::NULL,
            workshop: JsValue::NULL,
            depth: JsValue::NULL,
            depth_ready: false,
            home_stage: JsValue::NULL,
            region_map: JsValue::NULL,
            home_ready: false,
            home_motion: None,
            load_attempted: false,
            model_ready: false,
            scroll,
            create_scroll,
            observer: JsValue::NULL,
            listeners: vec![],
            callbacks: vec![],
            changed: None,
            back: None,
            frame: None,
            frame_id: JsValue::NULL,
            drifts: vec![],
            pace: super::arena::FramePace::new(13.0),
            on_battle,
            hold_timer: JsValue::NULL,
            held: false,
            tip: tip.clone(),
            pointer: (0.0, 0.0, true),
            look,
            saved,
            brief: 0,
            step: 0,
            focus: "body".into(),
            disposed: false,
        }));
        state.borrow().dress();
        bind(&state);
        bind_canvas(&state);
        setup_workshop(&state)?;
        setup_drift(&state, &home, 1882.0 / 3344.0, true);
        setup_drift(&state, &fitting, 941.0 / 1672.0, false);
        setup_home_stage(&state);
        setup_depth(&state);
        let weak = Rc::downgrade(&state);
        let callback = Closure::wrap(Box::new(move |_: JsValue| {
            if let Some(state) = weak.upgrade() {
                if let Ok(state) = state.try_borrow() {
                    state.resize();
                }
            }
        }) as Box<dyn FnMut(JsValue)>);
        let observer = construct(&global(), "ResizeObserver", &[callback.as_ref().clone()])?;
        call(&observer, "observe", &[state.borrow().canvas.clone()]);
        state.borrow_mut().observer = observer;
        state.borrow_mut().callbacks.push(callback);
        let weak = Rc::downgrade(&state);
        let frame = Closure::wrap(Box::new(move |now: f64| {
            if let Some(state) = weak.upgrade() {
                let complete = state.try_borrow_mut().ok().and_then(|mut s| s.tick(now));
                if let Some(target) = complete {
                    finish_home_transition(&state, &target);
                }
            }
        }) as Box<dyn FnMut(f64)>);
        let id = call(
            &global(),
            "requestAnimationFrame",
            &[frame.as_ref().clone()],
        );
        state.borrow_mut().frame = Some(frame);
        state.borrow_mut().frame_id = id;
        state.borrow().refresh();
        state.borrow().refresh_operator();
        call(&state.borrow().host, "liquidGlass", &[root]);
        Ok(Self { state })
    }
    pub fn show(&self, view: &str) {
        sync_region(&self.state);
        self.state
            .borrow_mut()
            .show(view, Rc::downgrade(&self.state));
    }
    #[wasm_bindgen(js_name=setActive)]
    pub fn set_active(&self, active: bool) {
        if active {
            sync_region(&self.state);
        }
        self.state.borrow_mut().active(active);
    }
    pub fn dispose(&self) {
        self.state.borrow_mut().dispose();
    }
}

fn bind(state: &Rc<RefCell<Base>>) {
    let root = state.borrow().root.clone();
    let home = state.borrow().home.clone();
    let dialog = state.borrow().dialog.clone();
    let picker = query(&root, ".cd-slot-picker");
    let mut actions = vec![
        (query(&root, ".cd-to-fitting"), "click", "fitting"),
        (query(&root, ".cd-operator"), "click", "persona"),
        (query(&root, ".cd-to-persona"), "click", "persona"),
        (query(&root, ".cd-to-sync"), "click", "sync"),
        (query(&root, ".cd-tab-power"), "click", "syncClose"),
        (query(&root, ".cd-tab"), "animationend", "syncOff"),
        (query(&root, ".cd-sync"), "keydown", "syncKeys"),
        (query(&root, ".cd-sync-body"), "click", "syncSpend"),
        (query(&root, ".cd-briefing-next"), "click", "briefNext"),
        (query(&root, ".cd-briefing-skip"), "click", "briefSkip"),
        (query(&root, ".cd-create-steps"), "click", "step"),
        (query(&root, ".cd-focus"), "click", "focus"),
        (query(&root, ".cd-create-options"), "click", "look"),
        (query(&root, ".cd-create-next"), "click", "createNext"),
        (query(&root, ".cd-callsign-input"), "keydown", "callsignKeys"),
        (query(&root, ".cd-fitting .cd-back"), "click", "base"),
        (query(&root, ".cd-enter"), "click", "battle"),
        (query(&root, ".cd-open-catalog"), "click", "catalog"),
        (query(&root, ".cd-dialog-close"), "click", "close"),
        (dialog.clone(), "click", "dialog"),
        (dialog, "close", "closed"),
        (query(&root, ".cd-select-trigger"), "click", "picker"),
        (query(&root, ".cd-kit-trigger"), "click", "kits"),
        (query(&root, ".cd-kit-options"), "click", "kit"),
        (query(&root, ".cd-kit-picker"), "keydown", "kitKeys"),
        (state.borrow().fitting.clone(), "click", "kitOutside"),
        (picker.clone(), "click", "category"),
        (picker, "keydown", "keys"),
        (state.borrow().catalog.clone(), "click", "item"),
        (query(&home, ".cd-to-region"), "click", "region"),
        (query(&root, ".cd-region-back"), "click", "base"),
        (query(&root, ".cd-region"), "keydown", "regionKeys"),
    ];
    for button in all(&root, ".cd-slot[data-slot]") {
        actions.push((button.clone(), "click", "slot"));
        actions.push((button.clone(), "pointerenter", "slotEnter"));
        actions.push((button.clone(), "pointerleave", "slotLeave"));
        actions.push((button.clone(), "pointerdown", "slotDown"));
        actions.push((button.clone(), "pointerup", "slotUp"));
        actions.push((button.clone(), "pointercancel", "slotUp"));
        actions.push((button, "contextmenu", "slotMenu"));
    }
    for (target, event, action) in actions {
        let listener = listen(state, &target, event, action);
        state.borrow_mut().listeners.push(listener);
    }
}
/// The Persona canvas's own listeners; a region switch replaces the canvas and binds them again.
fn bind_canvas(state: &Rc<RefCell<Base>>) {
    let canvas = state.borrow().canvas.clone();
    for (event, action) in [
        ("keydown", "rotate"),
        ("persona-preview-error", "previewError"),
        ("persona-beards-ready", "beardsReady"),
        ("persona-hover", "personaHover"),
        ("persona-part", "personaPart"),
        ("pointermove", "tipMove"),
        ("pointerdown", "tipMove"),
        ("pointerleave", "tipHide"),
        ("webglcontextrestored", "restored"),
    ] {
        let listener = listen(state, &canvas, event, action);
        state.borrow_mut().canvas_listeners.push(listener);
    }
}
fn listen(state: &Rc<RefCell<Base>>, target: &JsValue, event: &str, action: &'static str) -> Listener {
    let target_copy = target.clone();
    let weak = Rc::downgrade(state);
    Listener::new(target, event, move |event| {
        let Some(state) = weak.upgrade() else {
            return;
        };
        if action == "battle" || action == "fitting" || action == "persona" {
            begin_home_transition(&state, action);
            return;
        }
        let Ok(mut s) = state.try_borrow_mut() else {
            return;
        };
        if s.disposed {
            return;
        }
        match action {
            "base" | "fitting" | "persona" | "sync" | "region" => s.navigate(action, weak.clone()),
            "syncSpend" => {
                let plus = closest(&event, "[data-stat]");
                if let Some(before) = exists(&plus).then(|| capsule::spend(&data(&plus, "stat"))).flatten() {
                    s.render_sync(Some(&before));
                    let again = query(&s.root, &format!(".cd-sync-plus[data-stat=\"{}\"]", data(&plus, "stat")));
                    if !boolean(&again, "disabled") {
                        focus(&again);
                    }
                }
            }
            "briefNext" => s.brief_next(),
            "briefSkip" => s.brief_skip(),
            "step" => {
                let tab = closest(&event, "[data-step]");
                if exists(&tab) {
                    s.go_step(data(&tab, "step").parse().unwrap_or(0));
                }
            }
            "focus" => {
                let button = closest(&event, "[data-focus]");
                if exists(&button) {
                    s.set_focus(&data(&button, "focus"));
                }
            }
            "look" => {
                let choice = closest(&event, "[data-look]");
                if exists(&choice) && !boolean(&choice, "disabled") {
                    s.pick_look(&data(&choice, "look"), &data(&choice, "value"));
                }
            }
            "beardsReady" if s.view == "persona" => s.render_create(),
            "beardsReady" => {},
            "createNext" => s.create_next(weak.clone()),
            "callsignKeys" => {
                hidden(&query(&s.fitting, ".cd-create-error"), true);
                if text(&get(&event, "key")) == "Enter" {
                    call(&event, "preventDefault", &[]);
                    s.create_next(weak.clone());
                }
            }
            "catalog" => {
                let slot = s.selected.clone();
                s.open_catalog(&slot, weak.clone());
            }
            "slot" => {
                // The click that ends a long press only folds the card.
                if s.held {
                    s.held = false;
                    return;
                }
                s.hot("", true);
                s.open_catalog(&data(&target_copy, "slot"), weak.clone());
            }
            "tipMove" => {
                s.pointer = (
                    num(&event, "clientX"),
                    num(&event, "clientY"),
                    text(&get(&event, "pointerType")) == "mouse",
                );
                s.place_tip();
            }
            "tipHide" => s.show_tip(""),
            // A tap on the hair, the eyes or the suit opens that step of the creation.
            "personaPart" => {
                let part = get(&event, "detail").as_string().unwrap_or_default();
                let step = creation::STEPS.iter().position(|step| step.0 == part);
                if let Some(step) = step.filter(|_| s.brief >= creation::BRIEFING.len()) {
                    if step != s.step {
                        s.go_step(step);
                    }
                }
            }
            "personaHover" if s.view == "persona" => {}
            "personaHover" => {
                let slot = get(&event, "detail").as_string().unwrap_or_default();
                s.show_tip(&slot);
                s.hot(&slot, false);
            }
            "slotEnter" | "slotLeave" | "slotDown" | "slotUp" => {
                let mouse = text(&get(&event, "pointerType")) == "mouse";
                let slot = data(&target_copy, "slot");
                match (action, mouse) {
                    ("slotEnter", true) => s.hot(&slot, true),
                    ("slotLeave", true) => s.hot("", true),
                    ("slotDown", false) => {
                        s.held = false;
                        s.cancel_hold();
                        let weak = weak.clone();
                        let fire = Closure::once_into_js(move || {
                            let Some(state) = weak.upgrade() else {
                                return;
                            };
                            let Ok(mut s) = state.try_borrow_mut() else {
                                return;
                            };
                            s.hold_timer = JsValue::NULL;
                            if !s.disposed {
                                s.held = true;
                                s.hot(&slot, true);
                            }
                        });
                        s.hold_timer = call(&global(), "setTimeout", &[fire, number(380.0)]);
                    }
                    ("slotUp" | "slotLeave", false) => {
                        s.cancel_hold();
                        if s.held {
                            s.hot("", true);
                        }
                    }
                    _ => {}
                }
            }
            "slotMenu" => {
                call(&event, "preventDefault", &[]);
            }
            "close" => {
                call(&s.dialog, "close", &[]);
            }
            "dialog" => {
                if outside(&event, &s.dialog) {
                    call(&s.dialog, "close", &[]);
                }
                let picker = query(&s.root, ".cd-slot-picker");
                if call(&picker, "contains", &[get(&event, "target")]).as_bool() != Some(true) {
                    s.picker(false);
                }
            }
            "closed" => {
                s.picker(false);
                call_optional(&s.item_viewer, "setActive", &[flag(false)]);
                if s.view == "fitting" {
                    focus(&s.origin);
                }
            }
            "picker" => s.picker(boolean(&query(&s.root, ".cd-select-options"), "hidden")),
            "kits" => s.kits(boolean(&query(&s.root, ".cd-kit-options"), "hidden")),
            "kit" => {
                let option = closest(&event, "[data-kit]");
                if exists(&option) {
                    s.pick_kit(data(&option, "kit").parse().unwrap_or(0));
                }
            }
            "kitKeys" => {
                let options = query(&s.root, ".cd-kit-options");
                let key = text(&get(&event, "key"));
                let open = !boolean(&options, "hidden");
                if key == "Escape" && open {
                    call(&event, "preventDefault", &[]);
                    call(&event, "stopPropagation", &[]);
                    s.kits(false);
                    focus(&query(&s.root, ".cd-kit-trigger"));
                } else if open && (key == "ArrowDown" || key == "ArrowUp") {
                    call(&event, "preventDefault", &[]);
                    let items = all(&options, "[data-kit]");
                    let active = get(&document(), "activeElement");
                    let at = items.iter().position(|item| *item == active).unwrap_or(0);
                    let next = if key == "ArrowDown" {
                        (at + 1) % items.len()
                    } else {
                        (at + items.len() - 1) % items.len()
                    };
                    focus(&items[next]);
                }
            }
            "syncClose" => s.close_sync(weak.clone()),
            "syncOff" => {
                if text(&get(&event, "animationName")) == "cd-tab-drop" {
                    s.leave_sync(weak.clone());
                }
            }
            "syncKeys" => {
                if text(&get(&event, "key")) == "Escape" {
                    call(&event, "preventDefault", &[]);
                    s.close_sync(weak.clone());
                }
            }
            "regionKeys" => {
                if text(&get(&event, "key")) == "Escape" {
                    call(&event, "preventDefault", &[]);
                    s.navigate("base", weak.clone());
                    focus(&query(&s.home, ".cd-to-region"));
                }
            }
            "kitOutside" => {
                let picker = query(&s.root, ".cd-kit-picker");
                if !boolean(&query(&s.root, ".cd-kit-options"), "hidden")
                    && call(&picker, "contains", &[get(&event, "target")]).as_bool() != Some(true)
                {
                    s.kits(false);
                }
            }
            "category" => {
                let option = closest(&event, "[data-category]");
                if exists(&option) {
                    s.picker(false);
                    let slot = data(&option, "category");
                    if slot == "weapon1" {
                        s.weapon_return = Some((
                            s.selected.clone(),
                            num(&get(&s.catalog, "parentElement"), "scrollTop"),
                        ));
                        call(&s.dialog, "close", &[]);
                        s.navigate("weapon", weak.clone());
                    } else {
                        s.selected = slot;
                        s.render_catalog(None);
                        focus(&query(&s.root, ".cd-select-trigger"));
                    }
                }
            }
            "keys" => s.picker_key(event),
            "item" => s.catalog_click(event),
            "rotate" => {
                let key = text(&get(&event, "key"));
                if key == "ArrowLeft" || key == "ArrowRight" {
                    call(&event, "preventDefault", &[]);
                    call_optional(
                        &s.viewer,
                        "rotate",
                        &[number(if key == "ArrowLeft" { -0.2 } else { 0.2 })],
                    );
                }
            }
            "previewError" => {
                let detail = get(&event, "detail");
                content(
                    &s.status,
                    &detail.as_string().unwrap_or(
                        "3D-просмотр недоступен. Каталог снаряжения доступен.".into(),
                    ),
                );
                hidden(&s.status, false);
            }
            "restored" => {
                if s.model_ready {
                    hidden(&s.status, true);
                }
            }
            _ => {}
        }
    })
}
/// Moves the screens to the base of the saved map's region, if a battle or the table changed it.
fn sync_region(state: &Rc<RefCell<Base>>) {
    {
        let mut s = state.borrow_mut();
        let region = base_region::current();
        if s.disposed || std::ptr::eq(s.region, region) {
            return;
        }
        s.region = region;
        s.unstage();
        s.dress();
    }
    bind_canvas(state);
    setup_home_stage(state);
    setup_depth(state);
    let mut s = state.borrow_mut();
    if s.staged() {
        s.ensure_viewer(Rc::downgrade(state));
        let active = s.active;
        call_optional(&s.viewer, "setActive", &[flag(active)]);
    }
}
/// An empty copy of a canvas in its place. The old canvas's context is let go at once, not when
/// the page runs out of contexts.
fn fresh_canvas(old: &JsValue) -> JsValue {
    let copy = call(old, "cloneNode", &[flag(false)]);
    call(old, "replaceWith", &[copy.clone()]);
    let gl = call(old, "getContext", &[js("webgl2")]);
    if exists(&gl) {
        call_optional(&call(&gl, "getExtension", &[js("WEBGL_lose_context")]), "loseContext", &[]);
    }
    copy
}
fn setup_workshop(state: &Rc<RefCell<Base>>) -> Result<(), JsValue> {
    let weak = Rc::downgrade(state);
    let back = Closure::wrap(Box::new(move || {
        if let Some(state) = weak.upgrade() {
            let mut s = state.borrow_mut();
            let return_to = s.weapon_return.take();
            s.navigate("fitting", weak.clone());
            if let Some((slot, top)) = return_to {
                focus(&query(&s.root, ".cd-open-catalog"));
                s.open_catalog(&slot, weak.clone());
                set(&get(&s.catalog, "parentElement"), "scrollTop", &number(top));
            } else {
                focus(&query(&s.root, "[data-slot=\"weapon1\"]"));
            }
        }
    }) as Box<dyn FnMut()>);
    let weak = Rc::downgrade(state);
    let change = Closure::wrap(Box::new(move |loadout: JsValue| -> JsValue {
        let Some(state) = weak.upgrade() else {
            return flag(false);
        };
        let mut s = state.borrow_mut();
        s.loadout = loadout;
        let saved = call(&s.host, "saveLocalLoadout", &[s.loadout.clone()]);
        s.refresh();
        saved
    }) as Box<dyn FnMut(JsValue) -> JsValue>);
    let workshop = construct(
        &state.borrow().host,
        "WeaponScreen",
        &[
            state.borrow().root.clone(),
            back.as_ref().clone(),
            change.as_ref().clone(),
        ],
    )?;
    let mut s = state.borrow_mut();
    s.workshop = workshop;
    s.back = Some(back);
    s.changed = Some(change);
    Ok(())
}
fn begin_home_transition(state: &Rc<RefCell<Base>>, target: &str) {
    let animate = {
        let mut s = state.borrow_mut();
        if s.disposed
            || !s.active
            || s.view != "base"
            || s.home_motion.is_some()
            || boolean(&query(&s.home, ".cd-enter"), "disabled")
        {
            return;
        }
        let reduce = boolean(
            &call(
                &global(),
                "matchMedia",
                &[js("(prefers-reduced-motion: reduce)")],
            ),
            "matches",
        );
        if s.home_ready && !reduce {
            s.home_motion = Some((
                target.into(),
                call(&get(&global(), "performance"), "now", &[])
                    .as_f64()
                    .unwrap_or(0.),
            ));
            toggle(&s.home, "cd-travelling", true);
            attr(&s.home, "aria-busy", "true");
            for selector in HOME_BUTTONS {
                set(&query(&s.home, selector), "disabled", &flag(true));
            }
            // Load the Persona during the tunnel journey, while the base remains visible.
            if target == "fitting" || target == "persona" {
                s.ensure_viewer(Rc::downgrade(state));
            }
            true
        } else {
            false
        }
    };
    if !animate {
        finish_home_transition(state, target);
    }
}
fn finish_home_transition(state: &Rc<RefCell<Base>>, target: &str) {
    if state.borrow().disposed {
        return;
    }
    if let Some(id) = target.strip_prefix("region:") {
        // The map moved the base: back home, and the base follows the region's first map.
        base_region::choose(id);
        state.borrow_mut().navigate("base", Rc::downgrade(state));
        sync_region(state);
        focus(&query(&state.borrow().home, ".cd-to-region"));
    } else if target == "battle" {
        enter_battle(state);
    } else {
        state.borrow_mut().navigate(target, Rc::downgrade(state));
    }
}
fn setup_home_stage(state: &Rc<RefCell<Base>>) {
    let host = state.borrow().host.clone();
    if !get(&host, "HomeStage").is_function() {
        return;
    }
    let canvas = query(&state.borrow().home, ".cd-home-projection");
    // Hidden canvases have no layout size: reveal it only once textures are ready.
    let url = format!(
        "{}img/{}",
        text(&get(&host, "baseUrl")),
        state.borrow().region.home
    );
    let Ok(stage) = construct(&host, "HomeStage", &[canvas.clone(), js(&url)]) else {
        return;
    };
    let ready = get(&stage, "ready");
    state.borrow_mut().home_stage = stage.clone();
    let weak = Rc::downgrade(state);
    spawn_local(async move {
        if JsFuture::from(Promise::resolve(&ready)).await.is_err() {
            return;
        }
        let Some(state) = weak.upgrade() else {
            return;
        };
        let mut s = state.borrow_mut();
        if s.disposed || s.home_stage != stage {
            return;
        }
        hidden(&canvas, false);
        if invoke(&s.home_stage, "draw", &[js("base"), number(0.)]).is_err() {
            hidden(&canvas, true);
            return;
        }
        s.home_ready = true;
        hidden(&query(&s.home, "img.cd-backdrop"), true);
        attr(&canvas, "data-depth-base", "true");
    });
}
fn enter_battle(state: &Rc<RefCell<Base>>) {
    let (callback, loadout, button) = {
        let state = state.borrow();
        if state.disposed {
            return;
        }
        (
            state.on_battle.clone(),
            state.loadout.clone(),
            query(&state.root, ".cd-enter"),
        )
    };
    set(&button, "disabled", &flag(true));
    // The external entrypoint may immediately call setActive; do not hold the controller borrow.
    let result = invoke(&callback, "call", &[JsValue::UNDEFINED, loadout]);
    let weak = Rc::downgrade(state);
    spawn_local(async move {
        let result = match result {
            Ok(promise) => JsFuture::from(Promise::resolve(&promise)).await,
            Err(error) => Err(error),
        };
        let Some(state) = weak.upgrade() else {
            return;
        };
        let mut state = state.borrow_mut();
        if state.disposed {
            return;
        }
        if result.is_err() {
            state.reset_home();
            state.active(true);
            content(
                &query(&state.root, ".cd-home-message"),
                "Не удалось открыть бой. Попробуйте ещё раз.",
            );
        }
        set(&button, "disabled", &flag(false));
    });
}
fn setup_depth(state: &Rc<RefCell<Base>>) {
    let fitting = state.borrow().fitting.clone();
    let canvas = query(&fitting, ".cd-backdrop-depth");
    let image = query(&fitting, "img.cd-backdrop");
    let host = state.borrow().host.clone();
    let base = text(&get(&host, "baseUrl"));
    let region = state.borrow().region;
    let picture = if platform::sharp_pictures() {
        region.fitting_sharp
    } else {
        region.fitting
    };
    let result = construct(
        &host,
        "DepthBackdrop",
        &[
            canvas.clone(),
            js(&format!("{base}img/{picture}")),
            js(&format!("{base}img/{}", region.fitting_depth)),
            number(941.0 / 1672.0),
            number(1.1),
        ],
    );
    if let Ok(depth) = result {
        let ready = get(&depth, "ready");
        state.borrow_mut().depth = depth.clone();
        let weak = Rc::downgrade(state);
        spawn_local(async move {
            if JsFuture::from(Promise::resolve(&ready)).await.is_ok() {
                if let Some(state) = weak.upgrade() {
                    let mut s = state.borrow_mut();
                    if !s.disposed && s.depth == depth {
                        s.depth_ready = true;
                        hidden(&canvas, false);
                        hidden(&image, true);
                    }
                }
            }
        });
    }
}
fn setup_drift(state: &Rc<RefCell<Base>>, home: &JsValue, ratio: f64, flat: bool) {
    let index = state.borrow().drifts.len();
    let still = boolean(
        &call(
            &global(),
            "matchMedia",
            &[js("(prefers-reduced-motion: reduce)")],
        ),
        "matches",
    );
    state.borrow_mut().drifts.push(Drift {
        home: home.clone(),
        image: query(home, ".cd-backdrop"),
        rings: all(home, ".cd-tunnel"),
        ratio,
        flat,
        still,
        lean: args(&[number(0.0), number(0.0)]).into(),
        want: args(&[number(0.0), number(0.0)]).into(),
    });
    if flat {
        // The home photograph stays registered until a destination is chosen.
        // Do not request device orientation or introduce a second CSS movement.
        style(&query(home, ".cd-backdrop"), "transform", "none");
        return;
    }
    let weak = Rc::downgrade(state);
    let listener = Listener::new(home, "pointermove", move |event| {
        if let Some(state) = weak.upgrade() {
            let mut s = state.borrow_mut();
            let drift = &mut s.drifts[index];
            let rect = call(&drift.home, "getBoundingClientRect", &[]);
            let w = num(&rect, "width");
            let h = num(&rect, "height");
            if w > 0.0 && h > 0.0 {
                drift.want = args(&[
                    number((num(&event, "clientX") - num(&rect, "left")) / w - 0.5),
                    number((num(&event, "clientY") - num(&rect, "top")) / h - 0.5),
                ])
                .into();
            }
        }
    });
    state.borrow_mut().listeners.push(listener);
    let weak = Rc::downgrade(state);
    let listener = Listener::new(&global(), "deviceorientation", move |event| {
        if !exists(&get(&event, "gamma")) || !exists(&get(&event, "beta")) {
            return;
        }
        if let Some(state) = weak.upgrade() {
            state.borrow_mut().drifts[index].want = args(&[
                number((num(&event, "gamma") / 30.0).clamp(-1.0, 1.0) / 2.0),
                number(((num(&event, "beta") - 45.0) / 30.0).clamp(-1.0, 1.0) / 2.0),
            ])
            .into();
        }
    });
    state.borrow_mut().listeners.push(listener);
    let orientation = get(&global(), "DeviceOrientationEvent");
    if get(&orientation, "requestPermission").is_function() {
        let mut requested = false;
        let listener = Listener::new(home, "click", move |_| {
            if requested {
                return;
            }
            requested = true;
            if let Ok(promise) = invoke(&orientation, "requestPermission", &[]) {
                spawn_local(async move {
                    let _ = JsFuture::from(Promise::resolve(&promise)).await;
                });
            }
        });
        state.borrow_mut().listeners.push(listener);
    }
}
