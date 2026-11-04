//! Dynamic resolution of the battle's 3D picture.
//!
//! The arena canvas holds only the 3D frame; the HUD is DOM on top of it, so lowering the canvas'
//! pixel ratio lowers the 3D picture and nothing else. The scale runs from 0.74 to 1.0 of the
//! quality's own pixel ratio (pixels go with the square of it).
//!
//! The controller reads the 95th-percentile gap between drawn frames per window, not the average,
//! because a weak phone's trouble is frequent long frames. Hysteresis and delays:
//! - down: two over-budget windows in a row (p95 above 1.4× the target) cut the scale by about
//!   the square root of target/p95 (a 0.75–0.95 step), so a GPU-bound frame lands near the budget;
//! - up: only a probe, after a run of in-budget windows (p95 within 1.25× the target), one 0.1 step;
//!   if the next two windows go over budget the probe is undone and the wait before the next
//!   one doubles (6 → 12 → … → 48 windows), so a frame that sits on the edge does not flicker;
//! - the window right after a change is skipped (the resize itself costs a frame or two).
//!
//! FSR is the rung above 1.0 (`fsr`): at full scale, after the same run of in-budget windows, a
//! probe draws the screen larger than the frame and lets the lens enlarge it with FSR1 EASU. It is
//! undone like a scale probe, and over budget it is the first thing dropped, before the scale.
//! Only where the arena allows it (`allow_fsr`: the screen has more pixels than the frame, no `?fsr=`).
//!
//! Target: 30 fps on «Минимум» (its frame cap), 60 elsewhere. Only «Минимум» and «Низкое» are
//! driven; higher presets stay at 1.0 (the ultra preset is not touched). `?res=0.7` pins the
//! scale for A/B runs (0.5–1.0), `?res=1` switches the controller off.

/// Art 2026-11-04: not below 74% — at half scale the picture went visibly blocky.
pub const MIN: f64 = 0.74;
pub const MAX: f64 = 1.0;
const STEP: f64 = 0.05;
const WINDOW_MS: f64 = 1000.;
const MIN_FRAMES: usize = 8;
// 1.4: on a 90 Hz phone a 30 fps frame that misses one refresh is 44 ms (1.33×); that alone is
// not worth a quarter of the pixels (measured on TECNO KI7: p95 stayed 45 ms even at 0.5).
const OVER: f64 = 1.4;
const FITS: f64 = 1.25;
const FIRST_PROBE: u32 = 6;
const LAST_PROBE: u32 = 48;

#[derive(Debug, Clone, PartialEq)]
pub struct DynRes {
    pub scale: f64,
    /// Target gap between frames in milliseconds; 0 = controller off (scale pinned).
    target: f64,
    gaps: Vec<f64>,
    start: f64,
    last: f64,
    over: u32,
    fits: u32,
    /// In-budget windows needed before the next probe up, and whether a probe is on trial.
    probe_after: u32,
    trial: Option<(f64, u32)>,
    /// The probe on trial is FSR's, not a scale step.
    trial_fsr: bool,
    skip: bool,
    /// The screen is drawn larger than the frame and enlarged with FSR1 EASU.
    pub fsr: bool,
    /// The arena lets the controller switch FSR on (kept across qualities).
    fsr_ok: bool,
    /// Over budget at the lowest scale: the quality guard may step the preset down.
    pub floor: bool,
}

impl Default for DynRes {
    fn default() -> Self {
        Self {
            scale: MAX,
            target: 0.,
            gaps: Vec::with_capacity(128),
            start: 0.,
            last: 0.,
            over: 0,
            fits: 0,
            probe_after: FIRST_PROBE,
            trial: None,
            trial_fsr: false,
            skip: false,
            fsr: false,
            fsr_ok: false,
            floor: false,
        }
    }
}

fn quantize(scale: f64) -> f64 {
    ((scale / STEP).round() / (1. / STEP)).clamp(MIN, MAX)
}

fn percentile95(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    v[((v.len() as f64 * 0.95).ceil() as usize).clamp(1, v.len()) - 1]
}

impl DynRes {
    /// `pinned` comes from `?res=`: Some(x) holds the scale at x (1.0 = off).
    pub fn new(quality: &str, pinned: Option<f64>) -> Self {
        let mut d = Self::default();
        d.follow(quality, pinned);
        d
    }

    /// A new quality: a new target. The scale starts again from 1.0.
    pub fn follow(&mut self, quality: &str, pinned: Option<f64>) {
        let target = match quality {
            "minimal" => 1000. / 30.,
            "low" => 1000. / 60.,
            _ => 0.,
        };
        *self = Self { target, fsr_ok: self.fsr_ok, ..Self::default() };
        if let Some(scale) = pinned {
            self.target = 0.;
            // a pinned scale may go below the floor, down to 0.5, for A/B runs
            self.scale = ((scale / STEP).round() / (1. / STEP)).clamp(0.5, MAX);
        }
    }

    /// Whether FSR may be probed; switching it off drops FSR at once (true = it changed).
    pub fn allow_fsr(&mut self, ok: bool) -> bool {
        self.fsr_ok = ok;
        if !ok && self.fsr {
            self.fsr = false;
            self.trial = None;
            self.trial_fsr = false;
            return true;
        }
        false
    }

    pub fn active(&self) -> bool {
        self.target > 0.
    }

    /// Feed one drawn frame's animation stamp. Returns the new scale when it changes.
    pub fn frame(&mut self, now: f64) -> Option<f64> {
        if !self.active() {
            return None;
        }
        let gap = now - self.last;
        self.last = now;
        // a hidden tab or a load stall: start the window again
        if self.start == 0. || gap > 1000. || gap <= 0. {
            self.start = now;
            self.gaps.clear();
            return None;
        }
        self.gaps.push(gap);
        if now - self.start < WINDOW_MS || self.gaps.len() < MIN_FRAMES {
            return None;
        }
        let p95 = percentile95(&mut self.gaps);
        self.gaps.clear();
        self.start = now;
        if std::mem::take(&mut self.skip) {
            return None;
        }
        self.window(p95)
    }

    fn window(&mut self, p95: f64) -> Option<f64> {
        let over = p95 > self.target * OVER;
        let fits = p95 <= self.target * FITS;
        self.floor = over && self.scale <= MIN;
        if let Some((before, left)) = self.trial {
            if over {
                // the probe up did not hold: back down, wait longer before the next one
                self.trial = None;
                self.probe_after = (self.probe_after * 2).min(LAST_PROBE);
                self.over = 0;
                self.fits = 0;
                if std::mem::take(&mut self.trial_fsr) {
                    return self.set_fsr(false);
                }
                return self.set(before);
            }
            self.trial = (left > 1).then_some((before, left - 1));
            if self.trial.is_none() {
                self.trial_fsr = false;
            }
        }
        if over {
            self.fits = 0;
            self.over += 1;
            if self.over >= 2 && self.fsr {
                // the cheapest thing to give back first
                self.over = 0;
                return self.set_fsr(false);
            }
            if self.over >= 2 && self.scale > MIN {
                self.over = 0;
                let step = (self.target / p95).sqrt().clamp(0.75, 0.95);
                return self.set(quantize((self.scale * step).min(self.scale - STEP)));
            }
            return None;
        }
        self.over = 0;
        if fits {
            self.fits += 1;
            if self.fits >= self.probe_after && self.scale < MAX {
                self.fits = 0;
                self.trial = Some((self.scale, 2));
                return self.set(quantize(self.scale + 0.1));
            }
            if self.fits >= self.probe_after && self.fsr_ok && !self.fsr {
                self.fits = 0;
                self.trial = Some((self.scale, 2));
                self.trial_fsr = true;
                return self.set_fsr(true);
            }
        } else {
            self.fits = 0;
        }
        None
    }

    fn set_fsr(&mut self, on: bool) -> Option<f64> {
        self.fsr = on;
        self.skip = true;
        Some(self.scale)
    }

    fn set(&mut self, scale: f64) -> Option<f64> {
        if (scale - self.scale).abs() < 1e-9 {
            return None;
        }
        self.scale = scale;
        self.skip = true;
        Some(scale)
    }
}

/// `?res=` from the page address: a number pins the scale, anything else (or none) leaves it automatic.
pub fn pinned(search: &str) -> Option<f64> {
    search
        .trim_start_matches('?')
        .split('&')
        .find_map(|kv| kv.strip_prefix("res="))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `secs` seconds of frames `gap` ms apart; returns every change.
    fn run(d: &mut DynRes, t: &mut f64, gap: f64, secs: f64) -> Vec<f64> {
        let mut out = vec![];
        let end = *t + secs * 1000.;
        while *t < end {
            *t += gap;
            if let Some(s) = d.frame(*t) {
                out.push(s);
            }
        }
        out
    }

    #[test]
    fn only_minimal_and_low_are_driven() {
        assert!(DynRes::new("minimal", None).active());
        assert!(DynRes::new("low", None).active());
        for q in ["medium", "high", "ultra"] {
            let mut d = DynRes::new(q, None);
            let mut t = 1.;
            assert!(run(&mut d, &mut t, 200., 20.).is_empty());
            assert_eq!(d.scale, 1.);
        }
    }

    #[test]
    fn pinned_scale_holds() {
        let mut d = DynRes::new("minimal", Some(0.7));
        assert_eq!(d.scale, 0.7);
        let mut t = 1.;
        assert!(run(&mut d, &mut t, 200., 20.).is_empty());
        assert_eq!(pinned("?cal&res=0.6&q=low"), Some(0.6));
        assert_eq!(pinned("?res=auto"), None);
        assert_eq!(pinned("?cal"), None);
    }

    #[test]
    fn slow_frames_go_down_to_the_floor_and_say_so() {
        let mut d = DynRes::new("minimal", None);
        let mut t = 1.;
        let changes = run(&mut d, &mut t, 100., 30.);
        assert!(!changes.is_empty());
        assert!(changes.windows(2).all(|w| w[1] < w[0]));
        assert_eq!(d.scale, MIN);
        assert!(d.floor);
    }

    #[test]
    fn in_budget_frames_stay_and_a_failed_probe_backs_off() {
        let mut d = DynRes::new("low", None);
        let mut t = 1.;
        // 60 fps: nothing to do at full scale
        assert!(run(&mut d, &mut t, 16.7, 20.).is_empty());
        // 30 fps: two windows over, then down
        let down = run(&mut d, &mut t, 33., 4.);
        assert_eq!(down.len(), 1);
        let low = d.scale;
        // frames fit again: after the probe wait it tries one step up
        let up = run(&mut d, &mut t, 16.7, 8.);
        assert_eq!(up, vec![quantize(low + 0.1)]);
        // the probe makes it slow: it goes back, and the next probe waits twice as long
        let back = run(&mut d, &mut t, 30., 2.05);
        assert_eq!(back, vec![low]);
        let quiet = run(&mut d, &mut t, 16.7, 8.);
        assert!(quiet.is_empty(), "{quiet:?} {d:?}");
        assert_eq!(run(&mut d, &mut t, 16.7, 6.), vec![quantize(low + 0.1)]);
    }

    #[test]
    fn fsr_is_the_rung_above_full_scale() {
        let mut d = DynRes::new("minimal", None);
        let mut t = 1.;
        // not allowed: nothing happens at full scale
        assert!(run(&mut d, &mut t, 33.3, 20.).is_empty());
        assert!(!d.fsr);
        d.allow_fsr(true);
        run(&mut d, &mut t, 33.3, 8.);
        assert!(d.fsr, "{d:?}");
        assert_eq!(d.scale, 1.);
        // it holds while frames fit
        assert!(run(&mut d, &mut t, 33.3, 20.).is_empty());
        assert!(d.fsr);
        // slow: FSR goes first, then the scale
        let back = run(&mut d, &mut t, 60., 3.5);
        assert!(!d.fsr);
        assert_eq!(back, vec![1.]);
        run(&mut d, &mut t, 60., 4.);
        assert!(d.scale < 1.);
        // a quality change keeps the permission, not the state
        d.follow("low", None);
        assert!(!d.fsr && d.fsr_ok);
        assert!(d.allow_fsr(false) == false);
    }

    #[test]
    fn a_failed_fsr_probe_backs_off() {
        let mut d = DynRes::new("minimal", None);
        d.allow_fsr(true);
        let mut t = 1.;
        run(&mut d, &mut t, 33.3, 8.);
        assert!(d.fsr);
        // the probe makes frames slow: undone after the trial's first over-budget window
        run(&mut d, &mut t, 60., 2.05);
        assert!(!d.fsr);
        assert_eq!(d.scale, 1.);
        // the next probe waits twice as long
        run(&mut d, &mut t, 33.3, 8.);
        assert!(!d.fsr);
        run(&mut d, &mut t, 33.3, 6.);
        assert!(d.fsr);
    }

    #[test]
    fn a_stall_is_not_a_slow_window() {
        let mut d = DynRes::new("minimal", None);
        let mut t = 1.;
        run(&mut d, &mut t, 33.3, 3.);
        t += 5000.;
        assert!(run(&mut d, &mut t, 33.3, 3.).is_empty());
        assert_eq!(d.scale, 1.);
    }
}
