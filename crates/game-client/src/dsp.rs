//! Audio sample generation and signal-level calculations, independent of WebAudio.
use crate::{data, num, text};
use serde_json::{json, Value};

fn gain(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}
fn noise(state: &mut u32) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state as f64 / 4294967296.0 * 2.0 - 1.0
}

/// `room` returns two consecutive channels, `noise` one. `clip` uses rate as
/// table length and seconds as drive. Seeded samples make native/WASM identical.
pub fn audio_samples(kind: &str, rate: u32, seconds: f64, seed: u32) -> Vec<f32> {
    if kind == "clip" {
        let n = rate.clamp(2, 65536) as usize;
        let drive = seconds.clamp(0.001, 100.0);
        let norm = drive.tanh();
        return (0..n)
            .map(|i| (drive * (i as f64 / (n - 1) as f64 * 2.0 - 1.0)).tanh() as f32 / norm as f32)
            .collect();
    }
    let rate = rate.clamp(8000, 192000) as usize;
    let seconds = seconds.clamp(0.01, 10.0);
    let n = (rate as f64 * seconds).floor() as usize;
    let mut state = if seed == 0 { 0x6d2b79f5 } else { seed };
    if kind == "noise" {
        return (0..n).map(|_| noise(&mut state) as f32).collect();
    }
    if kind != "room" {
        return vec![];
    }
    let mut out = vec![0.0; n * 2];
    let gap = (rate as f64 * 0.008).floor() as usize;
    for channel in 0..2 {
        let mut dull = 0.0;
        for i in gap..n {
            dull += 0.35 * (noise(&mut state) - dull);
            out[channel * n + i] =
                (dull * (-6.9 * (i - gap) as f64 / rate as f64 / seconds).exp()) as f32;
        }
    }
    out
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "audio.dsp.dbToGain" => json!(gain(num(&args[0]))),
        "audio.dsp.volume" => json!(gain(num(&args[0])) * num(&args[1]).clamp(0.0, 1.0).powi(2)),
        "audio.dsp.play" => {
            let def = &data()["SOUNDS"][text(&args[0])];
            let trim = num(&args[1]);
            let opts = &args[2];
            let pitch = num(&args[3]);
            let variation = num(&args[4]);
            json!({"playbackRate":2.0_f64.powf(pitch/12.0),"gainDb":num(&def["gainDb"])+trim+variation+num(&opts["gainDb"]),"bus":def["bus"],"duckDb":num(&def["duckDb"])})
        }
        "audio.dsp.radio" => {
            json!({"lowCutHz":380,"presenceHz":1600,"presenceDb":5,"highCutHz":3400,"drive":2.2,"hissDb":-50,"roomSeconds":0.6,"roomDb":-9})
        }
        // Reverb tail and return level of the fight room: the salt hall rings, the blocks are open air.
        "audio.dsp.room" => match text(&args[0]) {
            "blocks" => json!({"seconds":1.1,"returnDb":-8}),
            "off" => json!({"seconds":0.3,"returnDb":-60}),
            _ => json!({"seconds":2.6,"returnDb":-2}),
        },
        "audio.dsp.radioTrim" => json!(1.0 / (1.0 + num(&args[0]) * 0.35)),
        "audio.dsp.radioPlace" => {
            let far = num(&args[0]);
            json!({"direct":gain(-9.0*far),"room":gain(-9.0+6.0*far-3.0*(1.0-far))})
        }
        "audio.dsp.loop" => {
            let t = num(&args[0]);
            let duration = num(&args[1]);
            let now = num(&args[2]);
            let end = t + duration;
            let overlap = 1.5;
            json!({"end":end,"fadeInEnd":t+overlap,"fadeOutStart":end-overlap,"stopAt":end+0.05,"next":end-overlap,"delay":((end-overlap-now-1.0)*1000.0).max(0.0)})
        }
        _ => return Err(format!("Unknown DSP operation: {method}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn impulse_is_stereo_finite_and_decays() {
        let s = audio_samples("room", 48000, 0.6, 417);
        let n = s.len() / 2;
        assert_eq!(n, 28800);
        assert!(s.iter().all(|x| x.is_finite()));
        assert!(s[..384].iter().all(|x| *x == 0.0));
        let energy = |x: &[f32]| x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32;
        assert!(energy(&s[n - 4096..n]) < energy(&s[384..4480]) * 0.01);
        assert_ne!(&s[..n], &s[n..]);
        assert_eq!(s, audio_samples("room", 48000, 0.6, 417));
    }
    #[test]
    fn clip_is_symmetric_and_bounded() {
        let s = audio_samples("clip", 1024, 2.2, 0);
        assert!(s.iter().all(|v| v.abs() <= 1.00001));
        for i in 0..512 {
            assert!((s[i] + s[1023 - i]).abs() < 1e-6);
        }
    }
}
