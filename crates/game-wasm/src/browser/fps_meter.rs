//! The frame counter switched on in the settings: frames the game actually drew each second
//! and the longest gap between them, so a phone's smoothness can be read without a console.
use super::{bridge::*, platform::{store_get, store_set}};
use std::cell::RefCell;
use wasm_bindgen::JsValue;

const KEY: &str = "cd-fps";

#[derive(Default)]
struct Meter {
    on: Option<bool>,
    label: JsValue,
    start: f64,
    last: f64,
    frames: u32,
    worst: f64,
}
thread_local! { static METER: RefCell<Meter> = RefCell::new(Meter::default()); }

pub fn on() -> bool {
    METER.with(|m| *m.borrow_mut().on.get_or_insert_with(|| string(&store_get(KEY)) == "on"))
}

pub fn set_on(on: bool) {
    store_set(KEY, if on { "on" } else { "off" });
    METER.with(|m| {
        let mut m = m.borrow_mut();
        m.on = Some(on);
        if !on && present(&m.label) {
            call(&m.label, "remove", &[]);
            m.label = JsValue::UNDEFINED;
        }
        m.start = 0.;
    });
}

/// Every drawing loop calls this with its animation-frame stamp; loops that share one frame count once.
pub fn frame(now: f64) {
    if !on() {
        return;
    }
    METER.with(|m| {
        let mut m = m.borrow_mut();
        if now - m.last < 1. {
            return;
        }
        if m.start == 0. || now - m.last > 1000. {
            (m.start, m.last, m.frames, m.worst) = (now, now, 0, 0.);
            return;
        }
        m.worst = m.worst.max(now - m.last);
        m.last = now;
        m.frames += 1;
        let span = now - m.start;
        if span < 500. {
            return;
        }
        if !present(&m.label) || !boolean(&get(&m.label, "isConnected")) {
            let document = get(&global(), "document");
            let label = call(&document, "createElement", &["div".into()]);
            set(&label, "className", &"cd-fps-meter".into());
            set(&get(&label, "style"), "cssText", &"position:fixed;left:12px;top:calc(max(6px,env(safe-area-inset-top)) + 54px);z-index:2147483000;pointer-events:none;padding:3px 8px;border-radius:8px;background:rgba(0,0,0,.6);color:#fff;font:500 11px/1.2 ui-monospace,Menlo,Consolas,monospace;white-space:nowrap".into());
            call(&get(&document, "body"), "append", &[label.clone()]);
            m.label = label;
        }
        let text = format!("{:.0} к/с · {:.0} мс", f64::from(m.frames) * 1000. / span, m.worst);
        set(&m.label, "textContent", &text.into());
        (m.start, m.frames, m.worst) = (now, 0, 0.);
    });
}
