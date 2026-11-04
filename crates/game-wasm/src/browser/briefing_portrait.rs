//! Sedoy's video link in the briefing comes alive on the live qualities (live.rs): he breathes,
//! blinks, his head wanders a little and his lips follow his voice. The depth backdrop
//! (scene_stage.rs, game-graphics depth-backdrop.glsl) draws the photograph over the still <img>;
//! the mouth and eye shapes are GPT edits of the same photograph, aligned and cut into
//! img/base/instructor-face.webp by tools/base-depth/build_instructor_face.py.
use super::dom_support::*;
use js_sys::{Math::random, Promise};
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{JsFuture, spawn_local};

/// The portrait's photograph, under img/.
pub const PICTURE: &str = "base/instructor.webp";
/// Where the face moves, in the photograph's coordinates (u from the left, v from the top).
pub struct Face {
    /// The 2 x 2 cells: slightly open, open, rounded mouth; eyes shut. Under img/.
    pub shapes: &'static str,
    pub mouth: [f64; 4],
    pub eyes: [f64; 4],
    /// The head's centre and radii.
    pub head: [f64; 4],
}
pub const FACE: Face = Face {
    shapes: "base/instructor-face.webp",
    mouth: [0.37, 0.34, 0.58, 0.49],
    eyes: [0.32, 0.22, 0.56, 0.32],
    head: [0.44, 0.27, 0.16, 0.25],
};

fn now() -> f64 {
    call(&get(&global(), "performance"), "now", &[]).as_f64().unwrap_or(0.0)
}

pub struct Portrait {
    backdrop: JsValue,
    canvas: JsValue,
    /// Loaded, and given up.
    ready: Rc<Cell<bool>>,
    gone: Rc<Cell<bool>>,
    /// When the line on screen began (ms) and how long reading it takes, for when no voice is heard.
    line: (f64, f64),
    /// The syllable said without a voice: its start, length (ms), how wide and how round.
    syllable: (f64, f64, f64, f64),
    /// The loudest moment heard lately, so quiet and loud lines move the lips alike.
    peak: f64,
    was_open: bool,
    open: f64,
    round: f64,
    round_goal: f64,
    /// When the next blink starts (ms).
    blink: f64,
    stamp: f64,
}
impl Portrait {
    /// Lays the live portrait over the still one in `root`'s `.cd-briefing-portrait`.
    pub fn new(host: &JsValue, root: &JsValue) -> Option<Self> {
        let frame = query(root, ".cd-briefing-portrait");
        if !exists(&frame) {
            return None;
        }
        let canvas = create("canvas", "cd-briefing-face");
        attr(&canvas, "aria-hidden", "true");
        hidden(&canvas, true);
        call(&frame, "append", &[canvas.clone()]);
        let base = text(&get(host, "baseUrl"));
        let backdrop = construct(
            host,
            "DepthBackdrop",
            &[
                canvas.clone(),
                js(&format!("{base}img/{PICTURE}")),
                js(&format!("{base}img/base/instructor-depth.png")),
                number(1.0),
                number(1.0),
            ],
        )
        .ok()?;
        let ready = Rc::new(Cell::new(false));
        let gone = Rc::new(Cell::new(false));
        let loading = get(&backdrop, "ready");
        let (shown, done, given_up) = (canvas.clone(), ready.clone(), gone.clone());
        spawn_local(async move {
            if JsFuture::from(Promise::resolve(&loading)).await.is_ok() && !given_up.get() {
                done.set(true);
                hidden(&shown, false);
            }
        });
        Some(Self {
            backdrop,
            canvas,
            ready,
            gone,
            line: (0.0, 0.0),
            syllable: (0.0, 0.0, 0.0, 0.0),
            peak: 0.02,
            was_open: false,
            open: 0.0,
            round: 0.0,
            round_goal: 0.0,
            blink: 0.0,
            stamp: 0.0,
        })
    }
    /// A new line on screen.
    pub fn say(&mut self, line: &str) {
        self.line = (now(), 400.0 + line.chars().count() as f64 * 65.0);
        self.syllable = (0.0, 0.0, 0.0, 0.0);
    }
    /// Draws the portrait at `now` (ms); `level` is the voice's loudness, below zero with no
    /// voice to hear (sound not started yet), and the lips then follow the text.
    pub fn frame(&mut self, now: f64, level: f64) {
        if !self.ready.get() {
            return;
        }
        let dt = if self.stamp > 0.0 {
            ((now - self.stamp) / 1000.0).clamp(0.0, 0.1)
        } else {
            0.0
        };
        self.stamp = now;
        let goal = if level >= 0.0 { self.heard(level) } else { self.read(now) };
        let rate = if goal > self.open { 30.0 } else { 16.0 };
        self.open += (goal - self.open) * (dt * rate).min(1.0);
        self.round += (self.round_goal - self.round) * (dt * 12.0).min(1.0);
        let wide = ((self.open - 0.4) / 0.4).clamp(0.0, 1.0);
        let slight = (self.open / 0.4).min(1.0) * (1.0 - wide);
        let shut = self.shut(now);
        call(
            &self.backdrop,
            "face",
            &[
                number(slight),
                number(wide * (1.0 - self.round)),
                number(wide * self.round),
                number(shut),
            ],
        );
        call(&self.backdrop, "draw", &[number(0.0), number(0.0)]);
    }
    fn heard(&mut self, level: f64) -> f64 {
        self.peak = level.max(self.peak * 0.997).max(0.02);
        let x = ((level / self.peak - 0.12) / 0.6).clamp(0.0, 1.0);
        let goal = x * x * (3.0 - 2.0 * x);
        // Each new syllable may round the lips.
        let open = goal > 0.35;
        if open && !self.was_open {
            self.round_goal = if random() < 0.3 { 1.0 } else { 0.0 };
        }
        self.was_open = open;
        goal
    }
    fn read(&mut self, now: f64) -> f64 {
        let (start, length) = self.line;
        if now > start + length {
            self.round_goal = 0.0;
            return 0.0;
        }
        if now >= self.syllable.0 + self.syllable.1 {
            // A short pause now and then, between words.
            self.syllable = if random() < 0.15 {
                (now, 140.0 + random() * 160.0, 0.0, 0.0)
            } else {
                let round = if random() < 0.3 { 1.0 } else { 0.0 };
                (now, 110.0 + random() * 120.0, 0.35 + random() * 0.65, round)
            };
            self.round_goal = self.syllable.3;
        }
        let phase = ((now - self.syllable.0) / self.syllable.1).clamp(0.0, 1.0);
        self.syllable.2 * (std::f64::consts::PI * phase).sin()
    }
    /// How shut the eyes are: a blink every two to six seconds, now and then a double one.
    fn shut(&mut self, now: f64) -> f64 {
        if self.blink == 0.0 {
            self.blink = now + 1500.0 + random() * 2500.0;
        }
        let age = now - self.blink;
        if age < 0.0 {
            return 0.0;
        }
        if age > 230.0 {
            self.blink = now + if random() < 0.15 { 120.0 } else { 2200.0 + random() * 4300.0 };
        }
        if age < 70.0 {
            age / 70.0
        } else if age < 110.0 {
            1.0
        } else {
            (1.0 - (age - 110.0) / 120.0).max(0.0)
        }
    }
    pub fn dispose(&mut self) {
        if self.gone.replace(true) {
            return;
        }
        call_optional(&self.backdrop, "dispose", &[]);
        call(&self.canvas, "remove", &[]);
    }
}
impl Drop for Portrait {
    fn drop(&mut self) {
        self.dispose();
    }
}
