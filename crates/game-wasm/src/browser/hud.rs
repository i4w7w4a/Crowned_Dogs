//! Fight log and liquid-glass refraction, including SVG/filter ownership.
use super::bridge::*;
use js_sys::{Uint8ClampedArray, WeakSet};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::{UnwrapThrowExt, prelude::*};

const NS: &str = "http://www.w3.org/2000/svg";
const EMPTY: &str =
    "<li class=\"empty\">Здесь появится каждый ход: кто куда стрелял и сколько снял.</li>";

#[wasm_bindgen]
pub struct RustFightLog {
    els: JsValue,
    count: u32,
}
#[wasm_bindgen]
impl RustFightLog {
    #[wasm_bindgen(constructor)]
    pub fn new(els: JsValue) -> Self {
        Self { els, count: 0 }
    }
    #[wasm_bindgen(getter)]
    pub fn open(&self) -> bool {
        !boolean(&get(&get(&self.els, "sheet"), "hidden"))
    }
    #[wasm_bindgen(js_name = setOpen)]
    pub fn set_open(&self, open: bool) {
        set(&get(&self.els, "sheet"), "hidden", &(!open).into());
        call(
            &get(&self.els, "button"),
            "setAttribute",
            &["aria-expanded".into(), open.to_string().into()],
        );
        if open {
            self.scroll();
        }
    }
    pub fn reset(&mut self, intro: &str) {
        self.count = 0;
        set(&get(&self.els, "list"), "innerHTML", &EMPTY.into());
        self.note(intro);
    }
    pub fn add(&mut self, report: JsValue, names: JsValue, weapons: JsValue) {
        let view = super::dom_support::rust("ui.fightLog", &[report, names, weapons]);
        let list = get(&self.els, "list");
        if self.count == 0 {
            set(&list, "innerHTML", &"".into());
        }
        self.count += 1;
        let li = call(&document(), "createElement", &["li".into()]);
        set(&li, "className", &"entry".into());
        set(&li, "innerHTML", &get(&view, "entry"));
        call(&list, "appendChild", &[li]);
        self.scroll();
        set(
            &get(&self.els, "ticker"),
            "innerHTML",
            &get(&view, "ticker"),
        );
        let button = get(&self.els, "button");
        call(&get(&button, "classList"), "remove", &["fresh".into()]);
        let _ = get(&button, "offsetWidth");
        call(&get(&button, "classList"), "add", &["fresh".into()]);
    }
    pub fn note(&self, text: &str) {
        set(&get(&self.els, "ticker"), "textContent", &text.into());
    }
}
impl RustFightLog {
    fn scroll(&self) {
        let list = get(&self.els, "list");
        set(&list, "scrollTop", &get(&list, "scrollHeight"));
    }
}

struct GlassState {
    defs: JsValue,
    filters: HashMap<String, String>,
}
struct Glass {
    observer: JsValue,
    watched: WeakSet,
    _callback: Closure<dyn FnMut(JsValue)>,
}
impl Drop for Glass {
    fn drop(&mut self) {
        call(&self.observer, "disconnect", &[]);
    }
}
thread_local! { static GLASS: RefCell<Option<Glass>> = const { RefCell::new(None) }; }

fn attrs(el: &JsValue, values: &[(&str, JsValue)]) {
    for (key, value) in values {
        call(el, "setAttribute", &[(*key).into(), value.clone()]);
    }
}
fn svg(tag: &str) -> JsValue {
    call(&document(), "createElementNS", &[NS.into(), tag.into()])
}
fn draw_map(w: u32, h: u32, r: f64, rim: f64) -> JsValue {
    let canvas = call(&document(), "createElement", &["canvas".into()]);
    set(&canvas, "width", &w.into());
    set(&canvas, "height", &h.into());
    let ctx = call(&canvas, "getContext", &["2d".into()]);
    let img = call(&ctx, "createImageData", &[w.into(), h.into()]);
    let pixels = get(&img, "data").unchecked_into::<Uint8ClampedArray>();
    let mut bytes = vec![0u8; w as usize * h as usize * 4];
    let hw = w as f64 / 2.;
    let hh = h as f64 / 2.;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f64 + 0.5 - hw;
            let dy = y as f64 + 0.5 - hh;
            let qx = dx.abs() - (hw - r);
            let qy = dy.abs() - (hh - r);
            let (depth, nx, ny) = if qx > 0. && qy > 0. {
                let length = qx.hypot(qy);
                (r - length, qx / length, qy / length)
            } else if qx > qy {
                (r - qx, 1., 0.)
            } else {
                (r - qy, 0., 1.)
            };
            let k = (1. - depth / rim).clamp(0., 1.).powi(2);
            let i = (y as usize * w as usize + x as usize) * 4;
            // Uint8ClampedArray uses ties-to-even conversion, unlike a plain u8 cast.
            bytes[i] = (128. - dx.signum() * nx * k * 127.)
                .round_ties_even()
                .clamp(0., 255.) as u8;
            bytes[i + 1] = (128. - dy.signum() * ny * k * 127.)
                .round_ties_even()
                .clamp(0., 255.) as u8;
            bytes[i + 2] = 128;
            bytes[i + 3] = 255;
        }
    }
    pixels.copy_from(&bytes);
    call(&ctx, "putImageData", &[img, 0.into(), 0.into()]);
    call(&canvas, "toDataURL", &[])
}
fn make_filter(s: &mut GlassState, id: &str, w: u32, h: u32, r: f64) {
    if !present(&s.defs) {
        let root = svg("svg");
        attrs(
            &root,
            &[
                ("width", 0.into()),
                ("height", 0.into()),
                ("aria-hidden", "true".into()),
            ],
        );
        set(&get(&root, "style"), "position", &"absolute".into());
        s.defs = svg("defs");
        call(&root, "appendChild", &[s.defs.clone()]);
        call(&get(&document(), "body"), "appendChild", &[root]);
    }
    let rim = 12f64.min(h as f64 / 3.);
    let f = svg("filter");
    attrs(
        &f,
        &[
            ("id", id.into()),
            ("x", 0.into()),
            ("y", 0.into()),
            ("width", w.into()),
            ("height", h.into()),
            ("filterUnits", "userSpaceOnUse".into()),
            ("primitiveUnits", "userSpaceOnUse".into()),
            ("color-interpolation-filters", "sRGB".into()),
        ],
    );
    let map = svg("feImage");
    attrs(
        &map,
        &[
            ("href", draw_map(w, h, r, rim)),
            ("x", 0.into()),
            ("y", 0.into()),
            ("width", w.into()),
            ("height", h.into()),
            ("preserveAspectRatio", "none".into()),
            ("result", "map".into()),
        ],
    );
    let bend = svg("feDisplacementMap");
    attrs(
        &bend,
        &[
            ("in", "SourceGraphic".into()),
            ("in2", "map".into()),
            ("scale", (1.5 * rim).into()),
            ("xChannelSelector", "R".into()),
            ("yChannelSelector", "G".into()),
        ],
    );
    call(&f, "append", &[map, bend]);
    call(&s.defs, "appendChild", &[f]);
}
#[wasm_bindgen]
pub fn liquid_glass(root: JsValue) {
    GLASS.with(|slot| {
        let mut glass = slot.borrow_mut();
        if glass.is_none() {
            let brands = get(
                &get(&get(&global(), "navigator"), "userAgentData"),
                "brands",
            );
            if !present(&brands)
                || !values(&brands)
                    .iter()
                    .any(|b| string(&get(b, "brand")) == "Chromium")
            {
                return;
            }
            let state = Rc::new(RefCell::new(GlassState {
                defs: JsValue::UNDEFINED,
                filters: HashMap::new(),
            }));
            let observer_ref = Rc::new(RefCell::new(JsValue::UNDEFINED));
            let observer_callback = observer_ref.clone();
            let callback = Closure::<dyn FnMut(JsValue)>::new(move |entries| {
                for entry in values(&entries) {
                    let el = get(&entry, "target");
                    if !boolean(&get(&el, "isConnected")) {
                        call(&observer_callback.borrow(), "unobserve", &[el]);
                        continue;
                    }
                    let w = num(&get(&el, "offsetWidth")) as u32;
                    let h = num(&get(&el, "offsetHeight")) as u32;
                    if w == 0 || h == 0 {
                        continue;
                    }
                    let style = call(&global(), "getComputedStyle", &[el.clone()]);
                    let radius = js_sys::parse_float(&string(&get(&style, "borderTopLeftRadius")));
                    let r = (if radius.is_nan() { 0. } else { radius })
                        .min(w as f64 / 2.)
                        .min(h as f64 / 2.);
                    let key = format!("{w}x{h}r{}", r.round());
                    let mut state = state.borrow_mut();
                    let id = if let Some(id) = state.filters.get(&key) {
                        id.clone()
                    } else {
                        let id = format!("lg{}", state.filters.len());
                        state.filters.insert(key, id.clone());
                        make_filter(&mut state, &id, w, h, r);
                        id
                    };
                    call(
                        &get(&el, "style"),
                        "setProperty",
                        &["--lens".into(), format!("url(#{id})").into()],
                    );
                }
            });
            let observer = construct(
                &get(&global(), "ResizeObserver"),
                &[callback.as_ref().clone()],
            );
            *observer_ref.borrow_mut() = observer.clone();
            *glass = Some(Glass {
                observer,
                watched: WeakSet::new(),
                _callback: callback,
            });
        }
        let s = glass.as_ref().unwrap_throw();
        let root = if present(&root) { root } else { document() };
        for el in values(&call(&root, "querySelectorAll", &[".glass".into()])) {
            let object = el.unchecked_ref::<js_sys::Object>();
            if !s.watched.has(object) {
                s.watched.add(object);
                call(&s.observer, "observe", &[el]);
            }
        }
    });
}
