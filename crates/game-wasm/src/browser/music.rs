//! Host deck dispatch and non-repeating choices. Audio policy remains native Rust.
use super::bridge::*;
use js_sys::{Map, Object};
use wasm_bindgen::prelude::*;

fn audio(method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    game_client::dispatch(&format!("audio.{method}"), data(&array(args).into()))
        .map(|value| json(&value))
        .map_err(|error| js_sys::Error::new(&error).into())
}
fn weak(value: JsValue) -> JsValue {
    construct(&get(&global(), "WeakRef"), &[value])
}
fn deref(value: &JsValue) -> JsValue {
    call(value, "deref", &[])
}

#[wasm_bindgen]
pub fn audio_pick_other(n: f64, last: JsValue, random: JsValue) -> Result<f64, JsValue> {
    let sample = if n > 1.0 {
        apply(&random, &JsValue::UNDEFINED, &[])?
    } else {
        0.into()
    };
    audio("pickOther", &[n.into(), last, sample]).map(|value| num(&value))
}
#[wasm_bindgen]
pub fn audio_jitter(spread: JsValue, random: JsValue) -> Result<f64, JsValue> {
    let amplitude = num(&spread);
    let sample = if amplitude != 0.0 && !amplitude.is_nan() {
        apply(&random, &JsValue::UNDEFINED, &[])?
    } else {
        0.into()
    };
    audio("jitter", &[spread, sample]).map(|value| num(&value))
}

#[wasm_bindgen]
pub struct RustVariantPicker {
    last: Map,
    random: JsValue,
}
#[wasm_bindgen]
impl RustVariantPicker {
    #[wasm_bindgen(constructor)]
    pub fn new(random: JsValue) -> RustVariantPicker {
        Self {
            last: Map::new(),
            random: weak(random),
        }
    }
    pub fn pick(&self, key: &str, items: JsValue) -> Result<JsValue, JsValue> {
        let index = audio_pick_other(
            num(&get(&items, "length")),
            self.last.get(&key.into()),
            deref(&self.random),
        )?;
        self.last.set(&key.into(), &index.into());
        Ok(at(&items, index as usize))
    }
}

#[wasm_bindgen]
pub struct RustShuffle {
    items: JsValue,
    current: JsValue,
    random: JsValue,
}
#[wasm_bindgen]
impl RustShuffle {
    #[wasm_bindgen(constructor)]
    pub fn new(items: JsValue, random: JsValue) -> Result<RustShuffle, JsValue> {
        if num(&get(&items, "length")) == 0.0 {
            return Err(js_sys::Error::new("Empty playlist").into());
        }
        Ok(Self {
            items,
            current: object(&[]),
            random: weak(random),
        })
    }
    #[wasm_bindgen(getter)]
    pub fn now(&self) -> JsValue {
        get(&self.current, "value")
    }
    pub fn next(&self) -> Result<JsValue, JsValue> {
        let current = self.now();
        let last = if current.is_undefined() {
            JsValue::UNDEFINED
        } else {
            try_call(&self.items, "indexOf", &[current])?
        };
        let index = audio_pick_other(num(&get(&self.items, "length")), last, deref(&self.random))?;
        let value = at(&self.items, index as usize);
        set(&self.current, "value", &value);
        Ok(value)
    }
    pub fn set(&self, item: JsValue) -> Result<(), JsValue> {
        if !boolean(&try_call(&self.items, "includes", &[item.clone()])?) {
            return Err(js_sys::Error::new("Not in the playlist").into());
        }
        set(&self.current, "value", &item);
        Ok(())
    }
}

#[wasm_bindgen]
pub struct RustMusicDirector {
    host: JsValue,
    state: JsValue,
    playlist: JsValue,
}
#[wasm_bindgen]
impl RustMusicDirector {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, playlist: JsValue) -> Result<RustMusicDirector, JsValue> {
        Ok(Self {
            host: weak(host),
            state: object(&[("value", audio("music.new", &[])?)]),
            playlist: object(&[("value", playlist)]),
        })
    }
    #[wasm_bindgen(getter)]
    pub fn state(&self) -> Result<JsValue, JsValue> {
        audio("music.state", &[get(&self.state, "value")])
    }
    pub fn send(&self, method: &str, argument: JsValue) -> Result<(), JsValue> {
        let host = deref(&self.host);
        let random = if boolean(&audio(
            "music.needsRandom",
            &[get(&self.state, "value"), method.into(), argument.clone()],
        )?) {
            try_call(&host, "random", &[])?
        } else {
            0.into()
        };
        let result = audio(
            "music.step",
            &[get(&self.state, "value"), method.into(), argument, random],
        )?;
        // Update before calling the deck: its callback may synchronously query or change the director.
        set(&self.state, "value", &get(&result, "state"));
        let deck = get(&host, "deck");
        for operation in values(&get(&result, "operations")) {
            try_call(
                &deck,
                &string(&get(&operation, "method")),
                &values(&get(&operation, "args")),
            )?;
        }
        Ok(())
    }
    pub fn playlist(&self, list: JsValue) -> Result<(), JsValue> {
        if Object::is(&list, &get(&self.playlist, "value")) {
            return Ok(());
        }
        self.send("playlist", list.clone())?;
        set(&self.playlist, "value", &list);
        Ok(())
    }
}
