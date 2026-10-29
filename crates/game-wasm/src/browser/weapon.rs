//! Weapon workshop lifecycle, selection, focus restoration and viewer coordination.
use super::{dom_support::*, scroll::BrowserScroll};
use js_sys::{Array, Object};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use wasm_bindgen::{closure::Closure, prelude::*};

struct Weapon {
    host: JsValue,
    element: JsValue,
    sheet: JsValue,
    body: JsValue,
    scroll: BrowserScroll,
    loadout: JsValue,
    selected: Option<String>,
    origin: JsValue,
    viewer: JsValue,
    on_back: JsValue,
    on_change: JsValue,
    listeners: Vec<Listener>,
    sockets: Option<Closure<dyn FnMut(JsValue)>>,
    serial: u32,
    disposed: bool,
}
impl Weapon {
    fn render(&mut self, weak: Weak<RefCell<Self>>) {
        if !exists(&self.loadout) {
            return;
        }
        let view = rust("client.workshop.model", &[self.loadout.clone()]);
        let weapon = get(&view, "weapon");
        content(
            &query(&self.element, ".cw-class"),
            &text(&get(&view, "className")),
        );
        content(
            &query(&self.element, ".cw-name"),
            &text(&get(&view, "displayName")),
        );
        let kind = get(&weapon, "weapon");
        let shape = call(
            &self.host,
            "composeWeapon",
            &[kind.clone(), get(&view, "parts")],
        );
        let art = get(&get(&self.host, "WEAPON_ART"), &text(&kind));
        let markup = call(
            &self.host,
            "markup",
            &[shape.clone(), js(&format!("cw-weapon-{}", self.serial))],
        );
        self.serial += 1;
        html(
            &query(&self.element, ".cw-gun"),
            &text(&rust(
                "client.workshop.fallback",
                &[
                    call(&self.host, "bbox", &[shape]),
                    get(&art, "anchors"),
                    markup,
                ],
            )),
        );
        let real = exists(&get(&view, "model"));
        let model = query(&self.element, ".cw-model");
        toggle(&query(&self.element, ".cw-stage"), "cw-stage-3d", real);
        call(
            &query(&self.element, ".cw-gun"),
            "toggleAttribute",
            &[js("hidden"), flag(real)],
        );
        call(
            &query(&self.element, ".cw-links"),
            "toggleAttribute",
            &[js("hidden"), flag(!real)],
        );
        hidden(&model, !real);
        if real {
            if !exists(&self.viewer) {
                let callback = Closure::wrap(Box::new(move |points: JsValue| {
                    if let Some(state) = weak.upgrade() {
                        if let Ok(state) = state.try_borrow() {
                            state.draw_links(points);
                        }
                    }
                }) as Box<dyn FnMut(JsValue)>);
                let options = Object::new();
                set(&options, "workbench", &flag(true));
                set(&options, "onSockets", callback.as_ref());
                self.viewer =
                    construct(&self.host, "ItemViewer", &[model, options.into()]).unwrap_throw();
                self.sockets = Some(callback);
            }
            call(
                &self.viewer,
                "show",
                &[
                    get(&weapon, "id"),
                    get(&weapon, "name"),
                    self.loadout.clone(),
                ],
            );
        } else {
            call_optional(&self.viewer, "hide", &[]);
        }
        for socket in Array::from(&get(&view, "sockets")).iter() {
            let slot = text(&get(&socket, "slot"));
            let button = query(&self.element, &format!("[data-socket=\"{slot}\"]"));
            content(
                &query(&button, ".cw-socket-name"),
                &text(&get(&socket, "name")),
            );
            attr(
                &button,
                "aria-label",
                &format!(
                    "{}: {}",
                    text(&get(&socket, "name")),
                    text(&get(&socket, "title"))
                ),
            );
            attr(&button, "aria-haspopup", "dialog");
            attr(
                &button,
                "aria-expanded",
                &(boolean(&self.sheet, "open") && self.selected.as_deref() == Some(&slot))
                    .to_string(),
            );
            toggle(&button, "cw-modified", boolean(&socket, "modified"));
            set(&button, "title", &get(&socket, "title"));
        }
    }
    fn draw_links(&self, points: JsValue) {
        let stage = query(&self.element, ".cw-stage");
        let links = query(&self.element, ".cw-links");
        let w = num(&stage, "clientWidth");
        let h = num(&stage, "clientHeight");
        attr(&links, "viewBox", &format!("0 0 {w} {h}"));
        html(
            &links,
            &text(&rust(
                "client.workshop.links",
                &[points, number(w), number(h)],
            )),
        );
    }
    fn open(&self, title: &str) {
        content(&query(&self.element, "#cw-sheet-title"), title);
        call(&self.sheet, "showModal", &[]);
        call(&self.host, "liquidGlass", &[self.sheet.clone()]);
        set(&self.body, "scrollTop", &number(0.0));
        self.scroll.refresh();
        focus(&query(&self.element, ".cw-close"));
    }
    fn choices(&mut self, slot: &str) {
        if !exists(&self.loadout) {
            return;
        }
        self.origin = get(&document(), "activeElement");
        let weapon = slot == "weapon1";
        if !weapon {
            self.selected = Some(slot.into());
        }
        let choices = rust("client.workshop.choices", &[self.loadout.clone(), js(slot)]);
        for choice in Array::from(&choices).iter() {
            let id = get(&get(&choice, "item"), "id");
            let icon = if weapon {
                call(&self.host, "equipmentItemIcon", &[id])
            } else {
                let art = call(&self.host, "itemArt", &[id]);
                if exists(&art) {
                    let icon = call(
                        &self.host,
                        "svg",
                        &[
                            art,
                            js(&format!("workshop-part-{}", self.serial)),
                            number(6.0),
                        ],
                    );
                    self.serial += 1;
                    js(&text(&icon).replacen("<svg ", "<svg aria-hidden=\"true\" ", 1))
                } else {
                    js("<span aria-hidden=\"true\">—</span>")
                }
            };
            set(&choice, "icon", &icon);
        }
        html(
            &self.body,
            &text(&rust(
                "client.workshop.options",
                &[choices, js(if weapon { "weapon" } else { "mod" })],
            )),
        );
        let title = if weapon {
            "Сменить оружие".into()
        } else {
            text(&rust(
                "client.workshop.slotName",
                &[self.loadout.clone(), js(slot)],
            ))
        };
        self.open(&title);
        if !weapon {
            attr(
                &query(&self.element, &format!("[data-socket=\"{slot}\"]")),
                "aria-expanded",
                "true",
            );
        }
    }
    fn stats(&mut self) {
        if !exists(&self.loadout) {
            return;
        }
        self.origin = get(&document(), "activeElement");
        html(
            &self.body,
            &text(&rust("client.workshop.stats", &[self.loadout.clone()])),
        );
        self.open("Характеристики");
    }
    fn hide(&mut self) {
        hidden(&self.element, true);
        call_optional(&self.viewer, "setActive", &[flag(false)]);
        if boolean(&self.sheet, "open") {
            call(&self.sheet, "close", &[]);
        }
    }
    fn select(&mut self, event: JsValue, weak: Weak<RefCell<Self>>) {
        let button = closest(&event, "[data-mod], [data-weapon]");
        if !exists(&button) || !exists(&self.loadout) {
            return;
        }
        let weapon = data(&button, "weapon");
        let result = if !weapon.is_empty() {
            try_rust(
                "client.equipItem",
                &[self.loadout.clone(), js("weapon1"), js(&weapon)],
            )
        } else {
            try_rust(
                "client.equipWeaponMod",
                &[
                    self.loadout.clone(),
                    js(self.selected.as_deref().unwrap_or("")),
                    js(&data(&button, "mod")),
                ],
            )
        };
        match result {
            Ok(next) => {
                self.loadout = next.clone();
                let saved = invoke(&self.on_change, "call", &[JsValue::UNDEFINED, next]);
                match saved {
                    Ok(saved) => {
                        self.render(weak);
                        call(&self.sheet, "close", &[]);
                        let status = query(&self.element, ".cw-error");
                        let ok = saved.as_bool() == Some(true);
                        hidden(&status, ok);
                        content(
                            &status,
                            if ok {
                                ""
                            } else {
                                "Не удалось сохранить выбор. Он доступен до закрытия страницы."
                            },
                        );
                    }
                    Err(err) => self.show_error(err),
                }
            }
            Err(err) => self.show_error(err),
        }
    }
    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.hide();
        self.disposed = true;
        call_optional(&self.viewer, "dispose", &[]);
        self.scroll.dispose();
        self.listeners.clear();
        call(&self.element, "remove", &[]);
    }
    fn show_error(&self, err: JsValue) {
        let old = query(&self.body, ".cw-sheet-error");
        if exists(&old) {
            call(&old, "remove", &[]);
        }
        let message = create("p", "cw-sheet-error");
        attr(&message, "role", "alert");
        content(&message, &error(&err, "Не удалось установить деталь."));
        call(&self.body, "prepend", &[message]);
    }
}
impl Drop for Weapon {
    fn drop(&mut self) {
        self.dispose();
    }
}
#[wasm_bindgen]
pub struct BrowserWeapon {
    state: Rc<RefCell<Weapon>>,
}
impl Drop for BrowserWeapon {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.dispose();
        }
    }
}
#[wasm_bindgen]
impl BrowserWeapon {
    #[wasm_bindgen(constructor)]
    pub fn new(
        parent: JsValue,
        on_back: JsValue,
        on_change: JsValue,
        host: JsValue,
    ) -> Result<Self, JsValue> {
        let element = create("section", "cw-screen");
        hidden(&element, true);
        attr(&element, "aria-label", "Оружие");
        html(
            &element,
            &text(&rust(
                "client.workshop.initial",
                &[get(&host, "baseUrl"), get(&host, "backArrow")],
            )),
        );
        call(&parent, "append", &[element.clone()]);
        call(&host, "liquidGlass", &[element.clone()]);
        let sheet = query(&element, "dialog");
        let body = query(&element, ".cw-sheet-body");
        let scroll = BrowserScroll::new(body.clone())?;
        let state = Rc::new(RefCell::new(Weapon {
            host,
            element: element.clone(),
            sheet: sheet.clone(),
            body: body.clone(),
            scroll,
            loadout: JsValue::NULL,
            selected: None,
            origin: JsValue::NULL,
            viewer: JsValue::NULL,
            on_back,
            on_change,
            listeners: vec![],
            sockets: None,
            serial: 0,
            disposed: false,
        }));
        for (target, kind, action) in [
            (query(&element, ".cw-back"), "click", "back"),
            (query(&element, ".cw-info"), "click", "stats"),
            (query(&element, ".cw-change"), "click", "weapons"),
            (query(&element, ".cw-close"), "click", "close"),
            (query(&element, ".cw-sockets"), "click", "sockets"),
            (query(&element, ".cw-stage"), "click", "stage"),
            (sheet.clone(), "click", "outside"),
            (sheet, "close", "closed"),
            (body, "click", "select"),
        ] {
            let weak = Rc::downgrade(&state);
            let listener = Listener::new(&target, kind, move |event| {
                let Some(state) = weak.upgrade() else {
                    return;
                };
                if action == "back" {
                    let callback = state.borrow().on_back.clone();
                    call(&callback, "call", &[JsValue::UNDEFINED]);
                    return;
                }
                let Ok(mut s) = state.try_borrow_mut() else {
                    return;
                };
                if s.disposed {
                    return;
                }
                match action {
                    "stats" => s.stats(),
                    "weapons" => s.choices("weapon1"),
                    "close" => {
                        call(&s.sheet, "close", &[]);
                    }
                    "outside" => {
                        if outside(&event, &s.sheet) {
                            call(&s.sheet, "close", &[]);
                        }
                    }
                    "closed" => {
                        s.selected = None;
                        for button in all(&s.element, "[data-socket]") {
                            attr(&button, "aria-expanded", "false");
                        }
                        if !boolean(&s.element, "hidden") {
                            focus(&s.origin);
                        }
                    }
                    "sockets" | "stage" => {
                        let key = if action == "stage" {
                            "anchor"
                        } else {
                            "socket"
                        };
                        let button = closest(&event, &format!("[data-{key}]"));
                        if exists(&button) {
                            let slot = data(&button, key);
                            if action == "stage" {
                                focus(&query(&s.element, &format!("[data-socket=\"{slot}\"]")));
                            }
                            s.choices(&slot);
                        }
                    }
                    "select" => s.select(event, weak.clone()),
                    _ => {}
                }
            });
            state.borrow_mut().listeners.push(listener);
        }
        Ok(Self { state })
    }
    #[wasm_bindgen(getter)]
    pub fn element(&self) -> JsValue {
        self.state.borrow().element.clone()
    }
    pub fn show(&self, loadout: JsValue) {
        let mut state = self.state.borrow_mut();
        state.loadout = loadout;
        hidden(&state.element, false);
        state.render(Rc::downgrade(&self.state));
        focus(&query(&state.element, ".cw-back"));
    }
    pub fn hide(&self) {
        self.state.borrow_mut().hide();
    }
    pub fn dispose(&self) {
        self.state.borrow_mut().dispose();
    }
}
