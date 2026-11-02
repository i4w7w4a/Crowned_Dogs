//! The base screens come alive on the high and ultra qualities: dust hangs in the lamp light,
//! the lamps dip now and then, the rings turn, panels slide in and the base's name settles like a
//! radio readout. The motion lives in CSS (apps/web/src/base/live.css) under `:root.cd-live`;
//! this module marks the page and scatters the dust, so the lower qualities pay nothing.
//! The base photographs themselves come alive too (game-graphics live-scene.glsl): each picture's lamps,
//! screens, lights, water and air are listed here as a `Life`.
use super::dom_support::*;
use js_sys::Math::random;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::{JsValue, closure::Closure};

/// The shader that brings a photograph to life; a picture's fragment shader comes after it.
pub use game_graphics::LIVE_SCENE as SCENE;

/// What moves in one picture, in its own coordinates (u from the left, v from the top). Rects are
/// u0, v0, u1, v1; an empty rect is all zeros. See game-graphics live-scene.glsl for each field.
pub struct Life {
    pub lamps: &'static [[f64; 4]],
    pub screens: &'static [[f64; 4]],
    pub leds: &'static [[f64; 4]],
    pub steam: &'static [[f64; 4]],
    pub drips: &'static [[f64; 4]],
    pub tube: [f64; 4],
    pub wet: [f64; 4],
    pub net: [f64; 4],
    pub flow: [f64; 4],
    pub window: [f64; 4],
    pub sun: [f64; 4],
    pub mist: [f64; 4],
}
const NONE: [f64; 4] = [0.0; 4];
/// A picture with nothing listed: only the camera breathes.
pub const STILL: Life = Life {
    lamps: &[],
    screens: &[],
    leds: &[],
    steam: &[],
    drips: &[],
    tube: NONE,
    wet: NONE,
    net: NONE,
    flow: NONE,
    window: NONE,
    sun: NONE,
    mist: NONE,
};

/// The live scene's uniforms for `life`, at rest. `vec4` makes a THREE.Vector4.
pub fn uniforms(
    life: &Life,
    aspect: f64,
    vec4: &dyn Fn([f64; 4]) -> JsValue,
) -> Vec<(&'static str, JsValue)> {
    let value = |v: JsValue| {
        let u: JsValue = js_sys::Object::new().into();
        set(&u, "value", &v);
        u
    };
    let list = |spots: &[[f64; 4]], slots: usize| {
        let all = js_sys::Array::new();
        for i in 0..slots {
            all.push(&vec4(spots.get(i).copied().unwrap_or(NONE)));
        }
        value(all.into())
    };
    vec![
        ("uTime", value(number(0.0))),
        ("uLive", value(number(0.0))),
        ("uAspect", value(number(aspect))),
        ("uLamps", list(life.lamps, 8)),
        ("uScreens", list(life.screens, 3)),
        ("uLeds", list(life.leds, 6)),
        ("uSteam", list(life.steam, 2)),
        ("uDrips", list(life.drips, 4)),
        ("uTube", value(vec4(life.tube))),
        ("uWet", value(vec4(life.wet))),
        ("uNet", value(vec4(life.net))),
        ("uFlow", value(vec4(life.flow))),
        ("uWindow", value(vec4(life.window))),
        ("uSun", value(vec4(life.sun))),
        ("uMist", value(vec4(life.mist))),
    ]
}

/// A scene's clock: how alive it is (fading in over about two seconds when the quality allows
/// it, out when it no longer does) and the seconds it shows.
#[derive(Default)]
pub struct Clock {
    live: f64,
    stamp: f64,
}
impl Clock {
    /// Advances to now: (liveness 0–1, scene seconds).
    pub fn tick(&mut self) -> (f64, f64) {
        let now = js_sys::Date::now() / 1000.0;
        let dt = if self.stamp > 0.0 { (now - self.stamp).clamp(0.0, 0.1) } else { 0.0 };
        self.stamp = now;
        let goal = if on() { 1.0 } else { 0.0 };
        self.live += (goal - self.live) * (dt * 1.6).min(1.0);
        if (goal - self.live).abs() < 0.002 {
            self.live = goal;
        }
        (self.live, now % 1800.0)
    }
}

/// Screens that get dust in the air, and how many motes each.
const DUST: [(&str, usize); 3] = [(".cd-home", 34), (".cd-fitting", 26), (".cd-sync", 20)];
/// Letters a readout rolls through before it settles.
const GLYPHS: &[char] = &[
    'А', 'Б', 'В', 'Г', 'Д', 'Ж', 'З', 'К', 'Л', 'М', 'Н', 'П', 'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц',
    'Ш', 'Э', 'Я', '0', '1', '2', '3', '4', '7', '9', '#', '/', '+',
];
/// Ticks of a readout, 40 ms each.
const ROLL: u32 = 16;

thread_local! {
    static LIVE: Cell<bool> = const { Cell::new(false) };
}

/// Marks the page for its quality: `cd-live` on high and ultra.
pub fn mark(id: &str) {
    let live = matches!(id, "high" | "ultra");
    toggle(&get(&document(), "documentElement"), "cd-live", live);
    let still = boolean(
        &call(
            &global(),
            "matchMedia",
            &[js("(prefers-reduced-motion: reduce)")],
        ),
        "matches",
    );
    LIVE.with(|cell| cell.set(live && !still));
}

/// Whether the screens move: a live quality and no request for reduced motion.
pub fn on() -> bool {
    LIVE.with(Cell::get)
}

/// Scatters the dust over each screen once; CSS shows it only on a live page. Every mote gets its
/// own size, path, pace and phase, and one in seven is a big soft out-of-focus fleck.
pub fn dress(root: &JsValue) {
    for (selector, count) in DUST {
        let screen = query(root, selector);
        if !exists(&screen) || exists(&query(&screen, ":scope > .cd-motes")) {
            continue;
        }
        let mut motes = String::new();
        for i in 0..count {
            let big = i % 7 == 3;
            let size = if big { 14.0 + random() * 18.0 } else { 1.5 + random() * 2.5 };
            let pace = 14.0 + random() * 18.0;
            motes.push_str(&format!(
                "<i{} style=\"left:{:.1}%;top:{:.1}%;--s:{size:.1}px;--d:{pace:.1}s;--t:-{:.1}s;--dx:{:.0}px;--dy:{:.0}px;--o:{:.2}\"></i>",
                if big { " class=\"b\"" } else { "" },
                random() * 100.0,
                random() * 92.0,
                random() * pace,
                (random() - 0.5) * 90.0,
                -30.0 - random() * 70.0,
                if big { 0.25 + random() * 0.3 } else { 0.35 + random() * 0.5 },
            ));
        }
        let layer = create("div", "cd-motes");
        attr(&layer, "aria-hidden", "true");
        html(&layer, &motes);
        call(&screen, "prepend", &[layer]);
    }
}

/// Rolls `el`'s text like a readout settling: random letters lock in from left to right.
/// Showing the screen again mid-roll starts over from the same word.
pub fn scramble(el: &JsValue) {
    if !exists(el) || !on() {
        return;
    }
    let dataset = get(el, "dataset");
    let rolling = get(&dataset, "rollText");
    let word = if rolling.is_string() {
        text(&rolling)
    } else {
        text(&get(el, "textContent"))
    };
    let letters: Vec<char> = word.chars().collect();
    if letters.iter().all(|c| c.is_whitespace()) {
        return;
    }
    let token = format!("{}", random());
    set(&dataset, "roll", &js(&token));
    set(&dataset, "rollText", &js(&word));
    let el = el.clone();
    let tick = Rc::new(Cell::new(0u32));
    let timer = Rc::new(Cell::new(0.0));
    let stop = {
        let el = el.clone();
        let timer = timer.clone();
        move |finish: Option<&str>| {
            call(&global(), "clearInterval", &[number(timer.get())]);
            if let Some(word) = finish {
                content(&el, word);
            }
            call(&el, "removeAttribute", &[js("data-roll")]);
            call(&el, "removeAttribute", &[js("data-roll-text")]);
        }
    };
    let id = timer.clone();
    let step = Closure::wrap(Box::new(move || {
        if data(&el, "roll") != token {
            // A newer roll took over.
            call(&global(), "clearInterval", &[number(timer.get())]);
            return;
        }
        let n = tick.get() + 1;
        tick.set(n);
        if n >= ROLL {
            return stop(Some(&word));
        }
        let shown: String = letters
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let lock = 4 + (i as u32 + 1) * (ROLL - 5) / letters.len() as u32;
                if n >= lock || !c.is_alphanumeric() {
                    c
                } else {
                    GLYPHS[(random() * GLYPHS.len() as f64) as usize % GLYPHS.len()]
                }
            })
            .collect();
        content(&el, &shown);
    }) as Box<dyn FnMut()>)
    .into_js_value();
    let interval = call(&global(), "setInterval", &[step, number(40.0)]);
    id.set(interval.as_f64().unwrap_or(0.0));
}
