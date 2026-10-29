//! Radio speaker graph and positional mixing; samples and policies share game-client.
use super::{bridge::*, dom_support::rust};
use js_sys::Float32Array;
use wasm_bindgen::prelude::*;

fn parameter(node: &JsValue, name: &str, value: JsValue) {
    set(&get(node, name), "value", &value);
}
fn connect(from: &JsValue, to: &JsValue) {
    call(from, "connect", &[to.clone()]);
}
fn samples(kind: &str, rate: u32, seconds: f64, seed: u32) -> Float32Array {
    Float32Array::from(crate::audio_samples(kind, rate, seconds, seed).as_slice())
}
fn buffer(ctx: &JsValue, kind: &str, seconds: f64) -> JsValue {
    let rate = num(&get(ctx, "sampleRate")) as u32;
    let data = samples(
        kind,
        rate,
        seconds,
        (js_sys::Math::random() * u32::MAX as f64) as u32,
    );
    let channels = if kind == "room" { 2 } else { 1 };
    let length = data.length() / channels;
    let out = call(
        ctx,
        "createBuffer",
        &[channels.into(), length.into(), rate.into()],
    );
    for channel in 0..channels {
        call(
            &out,
            "copyToChannel",
            &[
                data.subarray(channel * length, (channel + 1) * length)
                    .into(),
                channel.into(),
            ],
        );
    }
    out
}

#[wasm_bindgen]
pub struct RustRadio {
    ctx: JsValue,
    input: JsValue,
    output: JsValue,
    direct: JsValue,
    room: JsValue,
    hiss: JsValue,
    hiss_gain: JsValue,
    hiss_db: JsValue,
}
#[wasm_bindgen]
impl RustRadio {
    #[wasm_bindgen(constructor)]
    pub fn new(ctx: JsValue) -> Self {
        let config = rust("audio.dsp.radio", &[]);
        let low = call(&ctx, "createBiquadFilter", &[]);
        set(&low, "type", &"highpass".into());
        parameter(&low, "frequency", get(&config, "lowCutHz"));
        parameter(&low, "Q", 0.8.into());
        let presence = call(&ctx, "createBiquadFilter", &[]);
        set(&presence, "type", &"peaking".into());
        parameter(&presence, "frequency", get(&config, "presenceHz"));
        parameter(&presence, "gain", get(&config, "presenceDb"));
        parameter(&presence, "Q", 0.9.into());
        let high = call(&ctx, "createBiquadFilter", &[]);
        set(&high, "type", &"lowpass".into());
        parameter(&high, "frequency", get(&config, "highCutHz"));
        parameter(&high, "Q", 0.8.into());
        let clip = call(&ctx, "createWaveShaper", &[]);
        set(
            &clip,
            "curve",
            &samples("clip", 1024, num(&get(&config, "drive")), 0).into(),
        );
        set(&clip, "oversample", &"2x".into());
        let trim = call(&ctx, "createGain", &[]);
        parameter(
            &trim,
            "gain",
            rust("audio.dsp.radioTrim", &[get(&config, "drive")]),
        );
        let speaker = call(&ctx, "createGain", &[]);
        connect(&low, &presence);
        connect(&presence, &high);
        connect(&high, &clip);
        connect(&clip, &trim);
        connect(&trim, &speaker);
        let out = call(&ctx, "createGain", &[]);
        let panner = call(&ctx, "createStereoPanner", &[]);
        connect(&out, &panner);
        let direct = call(&ctx, "createGain", &[]);
        connect(&speaker, &direct);
        connect(&direct, &out);
        let room = call(&ctx, "createConvolver", &[]);
        set(
            &room,
            "buffer",
            &buffer(&ctx, "room", num(&get(&config, "roomSeconds"))),
        );
        let room_level = call(&ctx, "createGain", &[]);
        parameter(
            &room_level,
            "gain",
            rust("audio.dsp.dbToGain", &[get(&config, "roomDb")]),
        );
        connect(&speaker, &room);
        connect(&room, &room_level);
        connect(&room_level, &out);
        let hiss = call(&ctx, "createBufferSource", &[]);
        set(&hiss, "buffer", &buffer(&ctx, "noise", 2.));
        set(&hiss, "loop", &true.into());
        let band = call(&ctx, "createBiquadFilter", &[]);
        set(&band, "type", &"bandpass".into());
        parameter(&band, "frequency", 2500.into());
        parameter(&band, "Q", 0.7.into());
        let hiss_gain = call(&ctx, "createGain", &[]);
        parameter(&hiss_gain, "gain", 0.into());
        connect(&hiss, &band);
        connect(&band, &hiss_gain);
        connect(&hiss_gain, &speaker);
        call(&hiss, "start", &[]);
        Self {
            ctx,
            input: low,
            output: panner,
            direct,
            room: room_level,
            hiss,
            hiss_gain,
            hiss_db: get(&config, "hissDb"),
        }
    }
    #[wasm_bindgen(getter)]
    pub fn input(&self) -> JsValue {
        self.input.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn output(&self) -> JsValue {
        self.output.clone()
    }
    pub fn place(&self, pan: f64, far: f64) {
        let t = get(&self.ctx, "currentTime");
        call(
            &get(&self.output, "pan"),
            "setTargetAtTime",
            &[pan.into(), t.clone(), 0.3.into()],
        );
        let levels = rust("audio.dsp.radioPlace", &[far.into()]);
        call(
            &get(&self.direct, "gain"),
            "setTargetAtTime",
            &[get(&levels, "direct"), t.clone(), 0.3.into()],
        );
        call(
            &get(&self.room, "gain"),
            "setTargetAtTime",
            &[get(&levels, "room"), t, 0.3.into()],
        );
    }
    #[wasm_bindgen(js_name = setHiss)]
    pub fn set_hiss(&self, on: bool) {
        let target = if on {
            rust("audio.dsp.dbToGain", &[self.hiss_db.clone()])
        } else {
            0.into()
        };
        call(
            &get(&self.hiss_gain, "gain"),
            "setTargetAtTime",
            &[target, get(&self.ctx, "currentTime"), 0.08.into()],
        );
    }
    pub fn dispose(&self) {
        let _ = try_call(&self.hiss, "stop", &[]);
        call(&self.hiss, "disconnect", &[]);
        call(&self.input, "disconnect", &[]);
        call(&self.output, "disconnect", &[]);
    }
}
