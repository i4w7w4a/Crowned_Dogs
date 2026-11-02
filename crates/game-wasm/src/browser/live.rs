//! The base screens come alive on the high and ultra qualities: dust hangs in the lamp light,
//! the lamps dip now and then, the rings turn, panels slide in and the base's name settles like a
//! radio readout. The motion lives in CSS (apps/web/src/base/live.css) under `:root.cd-live`;
//! this module marks the page and scatters the dust, so the lower qualities pay nothing.
use super::dom_support::*;
use js_sys::Math::random;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::{JsValue, closure::Closure};

/// Screens that get dust in the air, and how many motes each.
const DUST: [(&str, usize); 3] = [(".cd-home", 34), (".cd-fitting", 26), (".cd-sync", 20)];
/// Letters a readout rolls through before it settles.
const GLYPHS: &[char] = &[
    'А', 'Б', 'В', 'Г', 'Д', 'Ж', 'З', 'К', 'Л', 'М', 'Н', 'П', 'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц',
    'Ш', 'Э', 'Я', '0', '1', '2', '3', '4', '7', '9', '#', '/', '+',
];
/// Ticks of a readout, 40 ms each.
const ROLL: u32 = 16;

/// Marks the page for its quality: `cd-live` on high and ultra.
pub fn mark(id: &str) {
    toggle(
        &get(&document(), "documentElement"),
        "cd-live",
        matches!(id, "high" | "ultra"),
    );
}

/// Whether the screens move: a live quality and no request for reduced motion.
fn on() -> bool {
    let root = get(&document(), "documentElement");
    call(&get(&root, "classList"), "contains", &[js("cd-live")]).as_bool() == Some(true)
        && !boolean(
            &call(
                &global(),
                "matchMedia",
                &[js("(prefers-reduced-motion: reduce)")],
            ),
            "matches",
        )
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
