//! The frame counter switched on in the settings, and the `?perf=1` telemetry built on it.
//!
//! Per window it shows the frames the game actually drew each second, the 95th-percentile and the
//! longest gap between them, the processor's time per frame and, in battle, the graphics chip's
//! time per frame: a WebGL 2 timer query (`EXT_disjoint_timer_query_webgl2`) around the arena's
//! draw, or WebGPU timestamp queries (`trackTimestamp`, set by `gpu.ts` under `?perf=1`).
//! A processor time well under the frame time means the graphics chip holds the phone back.
//!
//! `?perf=1` turns the counter on without saving it, adds a second line (WebGL 2 / WebGPU,
//! quality, drawing-buffer size), logs every window to the console as `[perf] …`, and keeps
//! `globalThis.__cdPerf` = `{ window, run }` (run: every frame since load or the last
//! `__cdPerf.reset()`), so `?gpu=0` and `?gpu=1` can be compared on a phone or over DevTools.
use super::{bridge::*, platform::{store_get, store_set}};
use std::{cell::RefCell, collections::VecDeque};
use wasm_bindgen::{closure::Closure, JsValue};

const KEY: &str = "cd-fps";
/// Samples kept for the whole-run numbers (about ten minutes at 60 fps).
const RUN_CAP: usize = 36_000;

#[derive(Default)]
struct Samples {
    gaps: Vec<f64>,
    cpu: Vec<f64>,
    gpu: Vec<f64>,
}
impl Samples {
    fn clear(&mut self) {
        self.gaps.clear();
        self.cpu.clear();
        self.gpu.clear();
    }
}

#[derive(Default)]
struct Meter {
    on: Option<bool>,
    perf: Option<bool>,
    label: JsValue,
    start: f64,
    last: f64,
    /// When this frame's drawing began and how long it took (written by `done`).
    began: f64,
    work: Option<f64>,
    window: Samples,
    run: Samples,
    /// The battle renderer and quality, for the GPU clock and the `?perf=1` line.
    renderer: JsValue,
    quality: String,
    /// The 3D picture's dynamic-resolution scale.
    scale: f64,
    timer: Timer,
    /// Kept alive for the page's life: `__cdPerf.reset` and the WebGPU resolve callback.
    reset: Option<Closure<dyn FnMut()>>,
    resolved: Option<Closure<dyn FnMut(JsValue)>>,
}

/// GPU time: queries in flight on WebGL 2, or one WebGPU resolve in flight.
#[derive(Default)]
struct Timer {
    /// None = not probed yet for this renderer; Some(NULL) = no timer on this device.
    ext: Option<JsValue>,
    gl: JsValue,
    open: bool,
    pending: VecDeque<JsValue>,
    webgpu: bool,
    resolving: bool,
}

thread_local! { static METER: RefCell<Meter> = RefCell::new(Meter::default()); }

fn perf() -> bool {
    METER.with(|m| {
        *m.borrow_mut().perf.get_or_insert_with(|| {
            let search = get(&get(&global(), "location"), "search");
            present(&search) && {
                let params = construct(&get(&global(), "URLSearchParams"), &[search]);
                string(&call(&params, "get", &["perf".into()])) == "1"
            }
        })
    })
}

pub fn on() -> bool {
    perf() || METER.with(|m| *m.borrow_mut().on.get_or_insert_with(|| string(&store_get(KEY)) == "on"))
}

pub fn set_on(on: bool) {
    store_set(KEY, if on { "on" } else { "off" });
    let perf = perf();
    METER.with(|m| {
        let mut m = m.borrow_mut();
        m.on = Some(on);
        if !on && !perf && present(&m.label) {
            call(&m.label, "remove", &[]);
            m.label = JsValue::UNDEFINED;
        }
        m.start = 0.;
    });
}

/// The arena tells which renderer and quality it draws with; a new renderer is probed again.
pub fn battle(renderer: &JsValue, quality: &str) {
    if !on() {
        return;
    }
    METER.with(|m| {
        let mut m = m.borrow_mut();
        if m.renderer != *renderer {
            m.renderer = renderer.clone();
            m.timer = Timer::default();
        }
        m.quality = quality.to_owned();
    });
}

/// The arena's dynamic-resolution scale, for the `?perf=1` line.
pub fn scale(scale: f64) {
    METER.with(|m| m.borrow_mut().scale = scale);
}

/// The arena is gone: menus are metered without a GPU clock again.
pub fn battle_over(renderer: &JsValue) {
    METER.with(|m| {
        let mut m = m.borrow_mut();
        if m.renderer == *renderer {
            m.renderer = JsValue::UNDEFINED;
            m.timer = Timer::default();
        }
    });
}

/// Every drawing loop calls this with its animation-frame stamp; loops that share one frame count once.
pub fn frame(now: f64) {
    if !on() {
        return;
    }
    let perf = perf();
    let report = METER.with(|m| {
        let mut m = m.borrow_mut();
        if now - m.last < 1. {
            return None;
        }
        if let Some(work) = m.work.take() {
            m.window.cpu.push(work);
            if m.run.cpu.len() < RUN_CAP {
                m.run.cpu.push(work);
            }
        }
        m.began = clock();
        if m.start == 0. || now - m.last > 1000. {
            (m.start, m.last) = (now, now);
            m.window.clear();
            return None;
        }
        let gap = now - m.last;
        m.last = now;
        m.window.gaps.push(gap);
        if m.run.gaps.len() < RUN_CAP {
            m.run.gaps.push(gap);
        }
        let span = now - m.start;
        if span < if perf { 1000. } else { 500. } {
            return None;
        }
        let window = stats(&m.window);
        m.window.clear();
        m.start = now;
        Some(window)
    });
    let Some(window) = report else { return };
    show(&window, perf);
}

/// Called right before the arena draws: opens this frame's GPU timer where there is one.
pub fn gpu_begin() {
    if !on() {
        return;
    }
    METER.with(|m| {
        let mut m = m.borrow_mut();
        if !present(&m.renderer) {
            return;
        }
        if m.timer.ext.is_none() {
            probe(&mut m);
        }
        let t = &mut m.timer;
        poll(t);
        if t.webgpu || t.open || t.pending.len() >= 4 {
            return;
        }
        let Some(ext) = t.ext.clone().filter(present) else { return };
        let query = call(&t.gl, "createQuery", &[]);
        call(&t.gl, "beginQuery", &[get(&ext, "TIME_ELAPSED_EXT"), query.clone()]);
        t.pending.push_back(query);
        t.open = true;
    });
    flush_gpu();
}

/// Called right after the arena draws: closes the WebGL timer, or asks WebGPU for its timestamps.
pub fn gpu_end() {
    if !on() {
        return;
    }
    METER.with(|m| {
        let mut m = m.borrow_mut();
        if m.timer.open {
            let ext = m.timer.ext.clone().unwrap_or(JsValue::NULL);
            call(&m.timer.gl, "endQuery", &[get(&ext, "TIME_ELAPSED_EXT")]);
            m.timer.open = false;
            return;
        }
        if !m.timer.webgpu || m.timer.resolving {
            return;
        }
        if m.resolved.is_none() {
            m.resolved = Some(Closure::new(|ms: JsValue| {
                METER.with(|m| {
                    let mut m = m.borrow_mut();
                    m.timer.resolving = false;
                    let ms = num(&ms);
                    if ms.is_finite() && ms > 0. {
                        push_gpu(&mut m, ms);
                    }
                });
            }));
        }
        let promise = call(&m.renderer, "resolveTimestampsAsync", &["render".into()]);
        if present(&promise) {
            m.timer.resolving = true;
            let callback = m.resolved.as_ref().unwrap().as_ref().clone();
            call(&promise, "then", &[callback.clone(), callback]);
        }
    });
}

/// Every drawing loop calls this when its frame is drawn; with loops sharing a frame the last one
/// counts, from the first one's start.
pub fn done() {
    if !on() {
        return;
    }
    METER.with(|m| {
        let mut m = m.borrow_mut();
        m.work = Some(clock() - m.began);
    });
}

fn probe(m: &mut Meter) {
    let r = &m.renderer;
    let webgpu = boolean(&get(r, "isWebGPURenderer")) && !boolean(&get(&get(r, "backend"), "isWebGLBackend"));
    m.timer.webgpu = webgpu && boolean(&get(&get(r, "backend"), "trackTimestamp"));
    if webgpu {
        m.timer.ext = Some(JsValue::NULL);
        return;
    }
    let gl = call(r, "getContext", &[]);
    let ext = if present(&gl) {
        call(&gl, "getExtension", &["EXT_disjoint_timer_query_webgl2".into()])
    } else {
        JsValue::NULL
    };
    m.timer.gl = gl;
    m.timer.ext = Some(if present(&ext) { ext } else { JsValue::NULL });
}

// Finished WebGL timer results go through a small buffer, because the meter is borrowed while polling.
thread_local! { static GPU_SINK: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) }; }
fn poll(t: &mut Timer) {
    let Some(ext) = t.ext.clone().filter(present) else { return };
    while let Some(query) = t.pending.front().cloned() {
        if t.open && t.pending.len() == 1 {
            break;
        }
        let available = get(&t.gl, "QUERY_RESULT_AVAILABLE");
        if !boolean(&call(&t.gl, "getQueryParameter", &[query.clone(), available])) {
            break;
        }
        let disjoint = boolean(&call(&t.gl, "getParameter", &[get(&ext, "GPU_DISJOINT_EXT")]));
        if !disjoint {
            let ns = num(&call(&t.gl, "getQueryParameter", &[query.clone(), get(&t.gl, "QUERY_RESULT")]));
            GPU_SINK.with(|s| s.borrow_mut().push(ns / 1e6));
        }
        call(&t.gl, "deleteQuery", &[query]);
        t.pending.pop_front();
    }
}
fn flush_gpu() {
    let done = GPU_SINK.with(|s| std::mem::take(&mut *s.borrow_mut()));
    if done.is_empty() {
        return;
    }
    METER.with(|m| {
        let mut m = m.borrow_mut();
        for ms in done {
            push_gpu(&mut m, ms);
        }
    });
}
fn push_gpu(m: &mut Meter, ms: f64) {
    m.window.gpu.push(ms);
    if m.run.gpu.len() < RUN_CAP {
        m.run.gpu.push(ms);
    }
}

struct Stats {
    fps: f64,
    p95: f64,
    worst: f64,
    cpu: f64,
    cpu95: f64,
    gpu: Option<(f64, f64)>,
    frames: usize,
}

fn stats(s: &Samples) -> Stats {
    let mean = |v: &[f64]| if v.is_empty() { 0. } else { v.iter().sum::<f64>() / v.len() as f64 };
    Stats {
        // frames over the time they took: a stall over a second restarts the window and is left out
        fps: s.gaps.len() as f64 * 1000. / s.gaps.iter().sum::<f64>().max(1.),
        p95: percentile(&s.gaps, 0.95),
        worst: s.gaps.iter().copied().fold(0., f64::max),
        cpu: mean(&s.cpu),
        cpu95: percentile(&s.cpu, 0.95),
        gpu: (!s.gpu.is_empty()).then(|| (mean(&s.gpu), percentile(&s.gpu, 0.95))),
        frames: s.gaps.len(),
    }
}

fn percentile(v: &[f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.;
    }
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    v[((v.len() as f64 * q).ceil() as usize).clamp(1, v.len()) - 1]
}

fn stats_object(s: &Stats) -> JsValue {
    let r = |x: f64| JsValue::from_f64((x * 10.).round() / 10.);
    let (gpu, gpu95) = s.gpu.map_or((JsValue::NULL, JsValue::NULL), |(a, b)| (r(a), r(b)));
    object(&[
        ("fps", r(s.fps)),
        ("p95", r(s.p95)),
        ("worst", r(s.worst)),
        ("cpu", r(s.cpu)),
        ("cpu95", r(s.cpu95)),
        ("gpu", gpu),
        ("gpu95", gpu95),
        ("frames", JsValue::from_f64(s.frames as f64)),
    ])
}

fn show(w: &Stats, perf: bool) {
    let (label, context, run) = METER.with(|m| {
        let mut m = m.borrow_mut();
        if !present(&m.label) || !boolean(&get(&m.label, "isConnected")) {
            let document = get(&global(), "document");
            let label = call(&document, "createElement", &["div".into()]);
            set(&label, "className", &"cd-fps-meter".into());
            set(&get(&label, "style"), "cssText", &"position:fixed;left:12px;top:calc(max(6px,env(safe-area-inset-top)) + 54px);z-index:2147483000;pointer-events:none;padding:3px 8px;border-radius:8px;background:rgba(0,0,0,.6);color:#fff;font:500 10px/1.25 ui-monospace,Menlo,Consolas,monospace;white-space:pre".into());
            call(&get(&document, "body"), "append", &[label.clone()]);
            m.label = label;
        }
        let scale = if m.scale > 0. { m.scale } else { 1. };
        let context = perf.then(|| (context(&m), scale));
        let run = perf.then(|| stats(&m.run));
        (m.label.clone(), context, run)
    });
    // short lines: the label sits next to the score on a phone
    // under ?perf=1 a missing GPU clock (no timer query on this device) shows as «ГП —»
    let gpu = w.gpu.map_or(if perf { " · ГП —".into() } else { String::new() }, |(gpu, _)| format!(" · ГП {gpu:.1}"));
    let mut text = format!(
        "{:.0} к/с · p95 {:.0} · макс {:.0}\nЦП {:.1}{gpu} мс",
        w.fps, w.p95, w.worst, w.cpu
    );
    if let (Some(((backend, quality, size), scale)), Some(run)) = (context, run) {
        text.push_str(&format!("\n{backend} · {quality} · {scale:.2}× · {size}"));
        let gpu = w.gpu.map_or("—".into(), |(a, b)| format!("{a:.1}/{b:.1}"));
        log(&format!(
            "[perf] {backend} {quality} x{scale:.2} {size} fps {:.1} p95 {:.1} worst {:.1} cpu {:.1}/{:.1} gpu {gpu} ms",
            w.fps, w.p95, w.worst, w.cpu, w.cpu95
        ));
        publish(w, &run, &backend, &quality, &size, scale);
    }
    set(&label, "textContent", &text.into());
}

fn context(m: &Meter) -> (String, String, String) {
    let r = &m.renderer;
    if !present(r) {
        return ("меню".into(), "—".into(), "—".into());
    }
    let backend = if boolean(&get(r, "isWebGPURenderer")) {
        if boolean(&get(&get(r, "backend"), "isWebGLBackend")) { "WebGPU→WebGL2" } else { "WebGPU" }
    } else {
        "WebGL2"
    };
    let canvas = get(r, "domElement");
    let size = format!("{}×{}", num(&get(&canvas, "width")), num(&get(&canvas, "height")));
    (backend.into(), m.quality.clone(), size)
}

fn publish(w: &Stats, run: &Stats, backend: &str, quality: &str, size: &str, scale: f64) {
    let g = global();
    let mut target = get(&g, "__cdPerf");
    if !present(&target) {
        target = object(&[]);
        set(&g, "__cdPerf", &target);
    }
    METER.with(|m| {
        let mut m = m.borrow_mut();
        if m.reset.is_none() {
            m.reset = Some(Closure::new(|| {
                METER.with(|m| {
                    m.borrow_mut().run.clear();
                });
            }));
        }
        set(&target, "reset", m.reset.as_ref().unwrap().as_ref());
        // the battle renderer, for poking at it over DevTools
        set(&target, "renderer", &m.renderer);
    });
    set(&target, "window", &stats_object(w));
    set(&target, "run", &stats_object(run));
    set(&target, "backend", &backend.into());
    set(&target, "quality", &quality.into());
    set(&target, "size", &size.into());
    set(&target, "scale", &scale.into());
}

fn log(text: &str) {
    call(&get(&global(), "console"), "log", &[text.into()]);
}

fn clock() -> f64 {
    num(&call(&get(&global(), "performance"), "now", &[]))
}
