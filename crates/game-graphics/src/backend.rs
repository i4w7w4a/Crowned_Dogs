//! Which renderer the battle draws with (stage 8): WebGPU only where a short measurement on this
//! device showed it faster than WebGL 2.
//!
//! Per quality (only «Минимум» and «Низкое», where WebGPU draws the same picture), the browser
//! keeps one record. Page loads take turns, WebGL 2 first, until each renderer has two measurements
//! (a phone's numbers swing with its temperature, one each is not enough); from then on the faster
//! one by the mean of its two is used. A measurement is 20 s of battle
//! after 8 s to settle (the dynamic resolution finds its scale). "Faster" is the frames per second
//! times the picture's pixels (the scale squared), at least 8% more, with a p95 no worse than 10%.
//! On a tie WebGL 2 stays: it is the path with the full picture on every quality.
//! A WebGPU trial that never finishes a measurement (three battles) or that three hands to its
//! WebGL fallback counts as lost. `VERSION` starts every record again when the rule changes.
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const WARMUP_MS: f64 = 8000.;
pub const MEASURE_MS: f64 = 20000.;
const WIN: f64 = 1.08;
const P95_SLACK: f64 = 1.1;
const MAX_TRIES: u32 = 3;
/// Measurements per renderer before the choice is made.
pub const SAMPLES: usize = 2;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Score {
    pub fps: f64,
    pub p95: f64,
    /// Mean dynamic-resolution scale over the measurement.
    pub scale: f64,
}

impl Score {
    fn work(&self) -> f64 {
        self.fps * self.scale * self.scale
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct Record {
    #[serde(default)]
    pub v: u32,
    #[serde(default)]
    pub gl: Vec<Score>,
    #[serde(default)]
    pub gpu: Vec<Score>,
    /// Battles started on WebGPU without a finished measurement since its last one.
    #[serde(default)]
    pub tries: u32,
    /// WebGPU was asked for and three gave its WebGL fallback: no real WebGPU here.
    #[serde(default)]
    pub broken: bool,
}

/// The qualities the choice is made for; the others always draw with WebGL 2.
pub fn driven(quality: &str) -> bool {
    matches!(quality, "minimal" | "low")
}

pub fn parse(text: &str) -> Record {
    serde_json::from_str::<Record>(text)
        .ok()
        .filter(|r| r.v == VERSION)
        .unwrap_or_default()
}

pub fn text(record: &Record) -> String {
    serde_json::to_string(record).unwrap_or_default()
}

pub fn faster(gpu: &Score, gl: &Score) -> bool {
    gpu.work() > gl.work() * WIN && gpu.p95 <= gl.p95 * P95_SLACK
}

/// What this page load draws the battle with: "webgl" or "webgpu".
pub fn choose(record: &Record, quality: &str, has_webgpu: bool) -> &'static str {
    if !has_webgpu || !driven(quality) || record.v != VERSION || record.broken {
        return "webgl";
    }
    let (gl, gpu) = (record.gl.len(), record.gpu.len());
    if gl < SAMPLES || gpu < SAMPLES {
        // take turns, WebGL 2 first; an unfinished WebGPU trial gives up after a few battles
        let gpu_turn = gpu < gl || gl >= SAMPLES;
        return if gpu_turn && record.tries < MAX_TRIES {
            "webgpu"
        } else {
            "webgl"
        };
    }
    if faster(&mean(&record.gpu), &mean(&record.gl)) {
        "webgpu"
    } else {
        "webgl"
    }
}

pub fn mean(scores: &[Score]) -> Score {
    let n = scores.len().max(1) as f64;
    let sum = |f: fn(&Score) -> f64| scores.iter().map(f).sum::<f64>() / n;
    Score {
        fps: sum(|s| s.fps),
        p95: sum(|s| s.p95),
        scale: sum(|s| s.scale),
    }
}

fn fresh(record: &mut Record) {
    if record.v != VERSION {
        *record = Record {
            v: VERSION,
            ..Record::default()
        };
    }
}

/// A battle starts on `backend`; an unmeasured WebGPU counts one more try.
pub fn started(record: &mut Record, backend: &str) {
    fresh(record);
    if backend == "webgpu" && record.gpu.len() < SAMPLES {
        record.tries += 1;
    }
}

/// A finished measurement; the first `SAMPLES` per backend are kept.
pub fn measured(record: &mut Record, backend: &str, score: Score) {
    fresh(record);
    match backend {
        "webgl" if record.gl.len() < SAMPLES => record.gl.push(score),
        "webgpu" if record.gpu.len() < SAMPLES => {
            record.gpu.push(score);
            record.tries = 0;
        }
        _ => {}
    }
}

/// Whether this page load still measures (the choice is not made yet).
pub fn measuring(record: &Record) -> bool {
    record.gl.len() < SAMPLES || record.gpu.len() < SAMPLES
}

/// WebGPU was asked for but three drew with its WebGL fallback.
pub fn fell_back(record: &mut Record) {
    fresh(record);
    record.broken = true;
}

/// Frames of one battle: 8 s to settle, then 20 s counted. A stall over a second (a hidden tab, a
/// late download) is left out, not counted as a slow frame.
#[derive(Default, Debug)]
pub struct Meter {
    first: f64,
    last: f64,
    gaps: Vec<f64>,
    scales: f64,
    done: bool,
}

impl Meter {
    /// Feed one drawn frame; returns the score once, when the measurement is complete.
    pub fn frame(&mut self, now: f64, scale: f64) -> Option<Score> {
        if self.done {
            return None;
        }
        let gap = now - self.last;
        self.last = now;
        if self.first == 0. {
            self.first = now;
            return None;
        }
        if gap > 1000. || gap <= 0. {
            self.first += gap.max(0.);
            return None;
        }
        if now - self.first < WARMUP_MS {
            return None;
        }
        self.gaps.push(gap);
        self.scales += scale;
        let span: f64 = self.gaps.iter().sum();
        if span < MEASURE_MS {
            return None;
        }
        self.done = true;
        let n = self.gaps.len() as f64;
        let mut sorted = self.gaps.clone();
        sorted.sort_by(f64::total_cmp);
        let p95 = sorted[((n * 0.95).ceil() as usize).clamp(1, sorted.len()) - 1];
        Some(Score {
            fps: n * 1000. / span,
            p95,
            scale: self.scales / n,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(fps: f64, p95: f64, scale: f64) -> Score {
        Score { fps, p95, scale }
    }

    #[test]
    fn webgl_first_then_a_webgpu_trial_then_the_faster() {
        let mut r = Record::default();
        for (gl, gpu) in [(24., 30.), (22., 28.)] {
            assert_eq!(choose(&r, "minimal", true), "webgl");
            started(&mut r, "webgl");
            measured(&mut r, "webgl", score(gl, 50., 0.8));
            assert_eq!(choose(&r, "minimal", true), "webgpu");
            started(&mut r, "webgpu");
            measured(&mut r, "webgpu", score(gpu, 40., 0.9));
        }
        assert!(!measuring(&r));
        assert_eq!(choose(&r, "minimal", true), "webgpu");
        // a later measurement does not change a made choice
        measured(&mut r, "webgpu", score(1., 400., 0.74));
        assert_eq!(choose(&r, "minimal", true), "webgpu");
        // round trip through the browser's storage
        assert_eq!(choose(&parse(&text(&r)), "minimal", true), "webgpu");
        // no WebGPU in this browser, or a quality WebGPU does not draw fully
        assert_eq!(choose(&r, "minimal", false), "webgl");
        assert_eq!(choose(&r, "medium", true), "webgl");
    }

    #[test]
    fn a_tie_or_a_worse_p95_keeps_webgl() {
        let gl = score(30., 34., 1.);
        assert!(!faster(&score(30., 34., 1.), &gl));
        assert!(!faster(&score(31., 34., 1.), &gl));
        assert!(!faster(&score(30., 45., 1.1), &gl));
        assert!(faster(&score(30., 35., 1.05), &score(30., 34., 0.95)));
    }

    #[test]
    fn an_unfinished_or_fallen_back_webgpu_gives_up() {
        let mut r = Record::default();
        measured(&mut r, "webgl", score(20., 60., 0.74));
        assert_eq!(choose(&r, "low", true), "webgpu");
        measured(&mut r, "webgpu", score(20., 60., 0.74));
        measured(&mut r, "webgl", score(20., 60., 0.74));
        for _ in 0..3 {
            assert_eq!(choose(&r, "low", true), "webgpu");
            started(&mut r, "webgpu");
        }
        assert_eq!(choose(&r, "low", true), "webgl");
        let mut r = Record::default();
        measured(&mut r, "webgl", score(20., 60., 0.74));
        fell_back(&mut r);
        assert_eq!(choose(&r, "low", true), "webgl");
    }

    #[test]
    fn an_old_or_broken_record_starts_again() {
        assert_eq!(
            parse("{\"v\":0,\"gl\":[{\"fps\":1,\"p95\":1,\"scale\":1}]}"),
            Record::default()
        );
        assert_eq!(
            parse("{\"v\":1,\"gl\":{\"fps\":1,\"p95\":1,\"scale\":1}}"),
            Record::default()
        );
        assert_eq!(parse("not json"), Record::default());
        assert_eq!(parse(""), Record::default());
    }

    #[test]
    fn the_meter_settles_then_counts_and_leaves_out_a_stall() {
        let mut m = Meter::default();
        let mut t = 1.;
        let mut got = None;
        // a stall halfway through the warm-up
        for _ in 0..150 {
            t += 33.3;
            assert!(m.frame(t, 1.).is_none());
        }
        t += 5000.;
        while got.is_none() && t < 100_000. {
            t += 33.3;
            got = m.frame(t, 0.8);
        }
        let s = got.unwrap();
        assert!((s.fps - 30.).abs() < 0.2, "{s:?}");
        assert!((s.scale - 0.8).abs() < 1e-9);
        assert!((s.p95 - 33.3).abs() < 0.01);
        // 8 s settle + 20 s, the stall not counted
        assert!(
            t > 5000. + WARMUP_MS + MEASURE_MS - 100. && t < 5000. + WARMUP_MS + MEASURE_MS + 100.,
            "{t}"
        );
        assert!(m.frame(t + 33.3, 1.).is_none(), "once per battle");
    }
}
