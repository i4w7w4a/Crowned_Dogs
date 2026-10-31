//! Base and fitting-room navigation, catalogue interaction and asynchronous viewer lifecycle.
use super::{dom_support::*, platform, scroll::BrowserScroll};
use js_sys::{Array, Promise};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use wasm_bindgen::{closure::Closure, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};

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
    stage: JsValue,
    canvas: JsValue,
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
    home_ready: bool,
    home_motion: Option<(String, f64)>,
    load_attempted: bool,
    model_ready: bool,
    scroll: BrowserScroll,
    observer: JsValue,
    listeners: Vec<Listener>,
    callbacks: Vec<Closure<dyn FnMut(JsValue)>>,
    changed: Option<Closure<dyn FnMut(JsValue) -> JsValue>>,
    back: Option<Closure<dyn FnMut()>>,
    frame: Option<Closure<dyn FnMut(f64)>>,
    frame_id: JsValue,
    drifts: Vec<Drift>,
    on_battle: JsValue,
    /// Touch long-press on a slot button: pending timer, and whether it fired.
    hold_timer: JsValue,
    held: bool,
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
        self.reset_home();
        call(&get(&self.host, "gameAudio"), "screen", &[js(view)]);
        if boolean(&self.dialog, "open") {
            call(&self.dialog, "close", &[]);
        }
        self.view = view.into();
        hidden(&self.home, view != "base");
        hidden(&self.fitting, view != "fitting");
        if view == "weapon" {
            call(&self.workshop, "show", &[self.loadout.clone()]);
        } else {
            call(&self.workshop, "hide", &[]);
        }
        if view == "fitting" {
            self.ensure_viewer(weak);
            self.resize();
        }
        call_optional(
            &self.viewer,
            "setActive",
            &[flag(self.active && view == "fitting")],
        );
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
            &[flag(active && self.view == "fitting")],
        );
        if active {
            self.resize();
        }
    }
    fn resize(&self) {
        let w = num(&self.stage, "clientWidth");
        let h = num(&self.stage, "clientHeight");
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
                call(&self.viewer, "updateLoadout", &[self.loadout.clone()]);
                self.resize();
                invoke(&self.viewer, "load", &[])
            });
        match result {
            Ok(promise) => spawn_local(async move {
                let result = JsFuture::from(Promise::resolve(&promise)).await;
                let Some(state) = weak.upgrade() else {
                    return;
                };
                let mut state = state.borrow_mut();
                if state.disposed {
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
                        &[flag(state.active && state.view == "fitting")],
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
        for button in all(&self.root, "[data-slot]") {
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
                let effects: Vec<String> = Array::from(&rust("describeItem", &[item.clone()]))
                    .iter()
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
    /// Unfolds one slot card ("" folds them all); a hover on the button also glows the figure.
    fn hot(&self, slot: &str, from_button: bool) {
        for button in all(&self.root, "[data-slot]") {
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
        let available: Vec<_> = Array::from(&rust("client.itemsForSlot", &[js(&self.selected)]))
            .iter()
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
        html(
            &self.catalog,
            &text(&rust(
                "ui.catalog",
                &[js(&self.selected), selected.clone(), choices.into()],
            )),
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
    fn catalog_click(&mut self, event: JsValue) {
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
        for selector in [".cd-enter", ".cd-to-fitting"] {
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
        if !self.active {
            return None;
        }
        let mut completed = None;
        if self.view == "base" && self.home_ready {
            let (target, progress) = if let Some((target, start)) = &self.home_motion {
                let duration = if target == "battle" { 2100. } else { 2400. };
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
        for drift in &mut self.drifts {
            let w = num(&drift.home, "clientWidth");
            let h = num(&drift.home, "clientHeight");
            if w <= 0.0 {
                continue;
            }
            if drift.flat && self.home_ready {
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
            if !drift.flat && self.depth_ready {
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
        call_optional(&self.viewer, "dispose", &[]);
        call_optional(&self.item_viewer, "dispose", &[]);
        call_optional(&self.workshop, "dispose", &[]);
        self.home_motion = None;
        call_optional(&self.home_stage, "dispose", &[]);
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
    for slot in Array::from(&get(host, "GEAR_SLOTS")).iter() {
        let slot = text(&slot);
        options.push_str(&format!("<button class=\"glass\" type=\"button\" role=\"option\" data-category=\"{slot}\" aria-selected=\"false\">{}</button>",text(&get(&names,&slot))));
    }
    let mut kits = String::new();
    for index in 0..platform::KITS {
        kits.push_str(&format!("<button class=\"glass\" type=\"button\" role=\"option\" data-kit=\"{index}\" aria-selected=\"false\"><strong>Набор {}</strong><small></small></button>", index + 1));
    }
    include_str!("base.html")
        .replace("{{KIT}}", &(platform::active_kit() + 1).to_string())
        .replace("{{KITS}}", &kits)
        .replace("{{BASE}}", &text(&get(host, "baseUrl")))
        .replace("{{BACK}}", &text(&get(host, "backArrow")))
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
        let stage = query(&root, ".cd-model-stage");
        let canvas = query(&root, ".cd-persona-canvas");
        let dialog = query(&root, "dialog");
        let catalog = query(&root, ".cd-catalog-items");
        let status = query(&root, ".cd-model-status");
        let scroll = BrowserScroll::new(query(&root, ".cd-dialog-content"))?;
        let loadout = call(&host, "loadLocalLoadout", &[]);
        let state = Rc::new(RefCell::new(Base {
            host,
            root: root.clone(),
            home: home.clone(),
            fitting: fitting.clone(),
            stage: stage.clone(),
            canvas: canvas.clone(),
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
            home_ready: false,
            home_motion: None,
            load_attempted: false,
            model_ready: false,
            scroll,
            observer: JsValue::NULL,
            listeners: vec![],
            callbacks: vec![],
            changed: None,
            back: None,
            frame: None,
            frame_id: JsValue::NULL,
            drifts: vec![],
            on_battle,
            hold_timer: JsValue::NULL,
            held: false,
            disposed: false,
        }));
        bind(&state);
        setup_workshop(&state)?;
        setup_drift(&state, &home, 1882.0 / 3344.0, true);
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
        call(&observer, "observe", &[stage]);
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
        call(&state.borrow().host, "liquidGlass", &[root]);
        Ok(Self { state })
    }
    pub fn show(&self, view: &str) {
        self.state
            .borrow_mut()
            .show(view, Rc::downgrade(&self.state));
    }
    #[wasm_bindgen(js_name=setActive)]
    pub fn set_active(&self, active: bool) {
        self.state.borrow_mut().active(active);
    }
    pub fn dispose(&self) {
        self.state.borrow_mut().dispose();
    }
}

fn bind(state: &Rc<RefCell<Base>>) {
    let root = state.borrow().root.clone();
    let dialog = state.borrow().dialog.clone();
    let canvas = state.borrow().canvas.clone();
    let picker = query(&root, ".cd-slot-picker");
    let mut actions = vec![
        (query(&root, ".cd-to-fitting"), "click", "fitting"),
        (query(&root, ".cd-back"), "click", "base"),
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
        (canvas.clone(), "keydown", "rotate"),
        (canvas.clone(), "persona-preview-error", "previewError"),
        (canvas.clone(), "persona-hover", "personaHover"),
        (canvas, "webglcontextrestored", "restored"),
    ];
    for button in all(&root, "[data-slot]") {
        actions.push((button.clone(), "click", "slot"));
        actions.push((button.clone(), "pointerenter", "slotEnter"));
        actions.push((button.clone(), "pointerleave", "slotLeave"));
        actions.push((button.clone(), "pointerdown", "slotDown"));
        actions.push((button.clone(), "pointerup", "slotUp"));
        actions.push((button.clone(), "pointercancel", "slotUp"));
        actions.push((button, "contextmenu", "slotMenu"));
    }
    for (target, event, action) in actions {
        let target_copy = target.clone();
        let weak = Rc::downgrade(state);
        let listener = Listener::new(&target, event, move |event| {
            let Some(state) = weak.upgrade() else {
                return;
            };
            if action == "battle" || action == "fitting" {
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
                "base" | "fitting" => s.navigate(action, weak.clone()),
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
                "personaHover" => {
                    let slot = get(&event, "detail").as_string().unwrap_or_default();
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
        });
        state.borrow_mut().listeners.push(listener);
    }
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
            for selector in [".cd-enter", ".cd-to-fitting"] {
                set(&query(&s.home, selector), "disabled", &flag(true));
            }
            // Load the Persona during the tunnel journey, while the base remains visible.
            if target == "fitting" {
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
    if target == "battle" {
        enter_battle(state);
    } else {
        state.borrow_mut().navigate("fitting", Rc::downgrade(state));
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
        "{}img/base-stage/manifest.json",
        text(&get(&host, "baseUrl"))
    );
    let Ok(stage) = construct(&host, "HomeStage", &[canvas.clone(), js(&url)]) else {
        return;
    };
    let ready = get(&stage, "ready");
    state.borrow_mut().home_stage = stage;
    let weak = Rc::downgrade(state);
    spawn_local(async move {
        if JsFuture::from(Promise::resolve(&ready)).await.is_err() {
            return;
        }
        let Some(state) = weak.upgrade() else {
            return;
        };
        let mut s = state.borrow_mut();
        if s.disposed {
            return;
        }
        hidden(&canvas, false);
        if invoke(&s.home_stage, "draw", &[js("base"), number(0.)]).is_err() {
            hidden(&canvas, true);
            return;
        }
        s.home_ready = true;
        hidden(&query(&s.home, "img.cd-backdrop"), true);
        attr(&canvas, "data-projected-base", "true");
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
    let result = construct(
        &host,
        "DepthBackdrop",
        &[
            canvas.clone(),
            js(&format!("{base}img/base/fitting-room-v2.png")),
            js(&format!("{base}img/base/fitting-room-v2-depth.png")),
            number(941.0 / 1672.0),
            number(1.1),
        ],
    );
    if let Ok(depth) = result {
        let ready = get(&depth, "ready");
        state.borrow_mut().depth = depth;
        let weak = Rc::downgrade(state);
        spawn_local(async move {
            if JsFuture::from(Promise::resolve(&ready)).await.is_ok() {
                if let Some(state) = weak.upgrade() {
                    let mut s = state.borrow_mut();
                    if !s.disposed {
                        s.depth_ready = true;
                        hidden(&canvas, false);
                        hidden(&image, true);
                    }
                }
            }
        });
        setup_drift(state, &fitting, 941.0 / 1672.0, false);
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
