//! The battle's renderer, picked per device by measurement (`game_graphics::backend`, stage 8).
//! `arena_backend` tells `arena.ts` which renderer to load; the arena feeds every drawn frame here,
//! and the finished measurement goes into `localStorage` as `cd-backend-<quality>`.
//! `?gpu=`, `?res=` and `?fsr=` are manual A/B runs: they neither use nor write the record.
use super::{
    bridge::*,
    platform::{store_get, store_set},
};
use game_graphics::backend::{self, Meter};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

struct Trial {
    key: String,
    renderer: JsValue,
    webgpu: bool,
    meter: Meter,
}

thread_local! { static TRIAL: RefCell<Option<Trial>> = const { RefCell::new(None) }; }

fn search() -> String {
    string(&get(&get(&global(), "location"), "search"))
}

fn param(name: &str) -> Option<String> {
    let s = search();
    s.trim_start_matches('?').split('&').find_map(|pair| {
        pair.strip_prefix(name)
            .and_then(|rest| rest.strip_prefix('='))
            .map(str::to_owned)
    })
}

fn manual() -> bool {
    ["gpu", "res", "fsr"].iter().any(|k| param(k).is_some())
}

fn key(quality: &str) -> String {
    format!("cd-backend-{quality}")
}

fn record(quality: &str) -> backend::Record {
    backend::parse(&string(&store_get(&key(quality))))
}

/// "webgpu" or "webgl": what this page load should draw the battle with.
#[wasm_bindgen]
pub fn arena_backend(has_webgpu: bool) -> String {
    let quality = param("q").unwrap_or_else(|| string(&store_get("cd-quality")));
    backend::choose(&record(&quality), &quality, has_webgpu).to_owned()
}

fn is_webgpu(renderer: &JsValue) -> bool {
    boolean(&get(renderer, "isWebGPURenderer"))
}

/// A battle (or a new quality in it) starts drawing with `renderer`.
pub fn start(renderer: &JsValue, quality: &str) {
    let same = TRIAL.with(|t| {
        t.borrow()
            .as_ref()
            .is_some_and(|x| x.renderer == *renderer && x.key == key(quality))
    });
    if same {
        return;
    }
    // measured only until the choice is made
    let trial = (!manual() && backend::driven(quality) && backend::measuring(&record(quality)))
        .then(|| {
            let webgpu = is_webgpu(renderer);
            let mut rec = record(quality);
            backend::started(&mut rec, if webgpu { "webgpu" } else { "webgl" });
            store_set(&key(quality), &backend::text(&rec));
            Trial {
                key: key(quality),
                renderer: renderer.clone(),
                webgpu,
                meter: Meter::default(),
            }
        });
    TRIAL.with(|t| *t.borrow_mut() = trial);
}

/// One drawn battle frame at dynamic-resolution `scale`.
pub fn frame(now: f64, scale: f64) {
    TRIAL.with(|t| {
        let mut t = t.borrow_mut();
        let Some(trial) = t.as_mut() else { return };
        let Some(score) = trial.meter.frame(now, scale) else {
            return;
        };
        let mut rec = backend::parse(&string(&store_get(&trial.key)));
        if trial.webgpu && boolean(&get(&get(&trial.renderer, "backend"), "isWebGLBackend")) {
            backend::fell_back(&mut rec);
        } else {
            backend::measured(
                &mut rec,
                if trial.webgpu { "webgpu" } else { "webgl" },
                score,
            );
        }
        store_set(&trial.key, &backend::text(&rec));
        *t = None;
    });
}

/// The arena is gone.
pub fn stop(renderer: &JsValue) {
    TRIAL.with(|t| {
        let mut t = t.borrow_mut();
        if t.as_ref().is_some_and(|trial| trial.renderer == *renderer) {
            *t = None;
        }
    });
}
