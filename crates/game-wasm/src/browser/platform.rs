//! Storage, device selection and the local duel object all run on the WASM side.
use super::{bridge::*, dom_support::rust};
use wasm_bindgen::prelude::*;

const LOADOUT_KEY: &str = "crowned-dogs:loadout:v1";
fn storage() -> JsValue {
    js_sys::Reflect::get(&global(), &"localStorage".into()).unwrap_or(JsValue::NULL)
}
#[wasm_bindgen]
pub fn store_get(key: &str) -> JsValue {
    try_call(&storage(), "getItem", &[key.into()]).unwrap_or(JsValue::NULL)
}
#[wasm_bindgen]
pub fn store_set(key: &str, value: &str) {
    let _ = try_call(&storage(), "setItem", &[key.into(), value.into()]);
}
#[wasm_bindgen]
pub fn load_local_loadout(supplied: JsValue) -> JsValue {
    let storage = if supplied.is_undefined() {
        storage()
    } else {
        supplied
    };
    let raw = try_call(&storage, "getItem", &[LOADOUT_KEY.into()]).unwrap_or(JsValue::NULL);
    rust("client.load", &[raw])
}
#[wasm_bindgen]
pub fn save_local_loadout(loadout: JsValue, supplied: JsValue) -> bool {
    let saved = rust("client.save", &[loadout]);
    let storage = if supplied.is_undefined() {
        storage()
    } else {
        supplied
    };
    if !present(&saved) || !present(&storage) {
        return false;
    }
    js_sys::JSON::stringify(&saved)
        .ok()
        .is_some_and(|raw| try_call(&storage, "setItem", &[LOADOUT_KEY.into(), raw.into()]).is_ok())
}
fn default_number(value: JsValue, default: f64) -> JsValue {
    if present(&value) {
        value
    } else {
        default.into()
    }
}
fn device_facts(gl: &JsValue) -> JsValue {
    if !present(gl) {
        return object(&[("webgl", false.into())]);
    }
    let info = call(gl, "getExtension", &["WEBGL_debug_renderer_info".into()]);
    let gpu = call(
        gl,
        "getParameter",
        &[if present(&info) {
            get(&info, "UNMASKED_RENDERER_WEBGL")
        } else {
            get(gl, "RENDERER")
        }],
    );
    let nav = get(&global(), "navigator");
    let mobile = js_sys::RegExp::new("android|iphone|ipad|ipod|mobile", "i")
        .test(&string(&get(&nav, "userAgent")));
    let coarse = !mobile
        && get(&global(), "matchMedia").is_function()
        && boolean(&get(
            &call(&global(), "matchMedia", &["(pointer: coarse)".into()]),
            "matches",
        ));
    object(&[
        ("webgl", true.into()),
        ("gpu", gpu),
        ("memory", default_number(get(&nav, "deviceMemory"), 4.)),
        (
            "cores",
            default_number(get(&nav, "hardwareConcurrency"), 4.),
        ),
        ("phone", (mobile || coarse).into()),
    ])
}
#[wasm_bindgen]
pub fn guess_quality(gl: JsValue) -> JsValue {
    rust("quality.guess", &[device_facts(&gl)])
}
#[wasm_bindgen]
pub fn start_quality(gl: JsValue) -> JsValue {
    let params = construct(
        &get(&global(), "URLSearchParams"),
        &[get(&get(&global(), "location"), "search")],
    );
    rust(
        "quality.start",
        &[
            call(&params, "get", &["q".into()]),
            store_get("cd-quality"),
            device_facts(&gl),
        ],
    )
}
#[wasm_bindgen]
pub fn keep_quality(id: &str) {
    store_set("cd-quality", id);
}

#[wasm_bindgen]
pub struct RustLocalDuel {
    state: JsValue,
}
#[wasm_bindgen]
impl RustLocalDuel {
    #[wasm_bindgen(constructor)]
    pub fn new(options: JsValue) -> Self {
        let opts = js_sys::Object::assign(
            &js_sys::Object::new(),
            options.unchecked_ref::<js_sys::Object>(),
        );
        if !present(&get(&opts, "seed")) {
            set(
                &opts,
                "seed",
                &call(&get(&global(), "crypto"), "randomUUID", &[]),
            );
        }
        Self {
            state: rust("client.duel.create", &[opts.into()]),
        }
    }
    #[wasm_bindgen(getter)]
    pub fn me(&self) -> u32 {
        0
    }
    #[wasm_bindgen(getter)]
    pub fn bot(&self) -> u32 {
        1
    }
    #[wasm_bindgen(getter)]
    pub fn fighters(&self) -> JsValue {
        get(&self.state, "fighters")
    }
    #[wasm_bindgen(getter,js_name = match)]
    pub fn match_value(&self) -> JsValue {
        get(&self.state, "match")
    }
    #[wasm_bindgen(getter)]
    pub fn draft(&self) -> JsValue {
        get(&self.state, "draft")
    }
    pub fn press(&mut self, command: JsValue) -> JsValue {
        self.change("press", command)
    }
    #[wasm_bindgen(js_name = setMove)]
    pub fn set_move(&mut self, value: f64) -> JsValue {
        self.change("setMove", value.into())
    }
    #[wasm_bindgen(getter)]
    pub fn missing(&self) -> JsValue {
        self.query("missing")
    }
    #[wasm_bindgen(getter)]
    pub fn chance(&self) -> JsValue {
        self.query("chance")
    }
    #[wasm_bindgen(getter,js_name = plannedDistance)]
    pub fn planned_distance(&self) -> f64 {
        num(&self.query("plannedDistance"))
    }
    #[wasm_bindgen(getter)]
    pub fn spare(&self) -> JsValue {
        self.query("spare")
    }
    #[wasm_bindgen(getter)]
    pub fn over(&self) -> bool {
        boolean(&self.query("over"))
    }
    pub fn submit(&mut self) -> JsValue {
        self.change("submit", JsValue::UNDEFINED)
    }
    pub fn timeout(&mut self) -> JsValue {
        self.change("timeout", JsValue::UNDEFINED)
    }
}
impl RustLocalDuel {
    fn query(&self, property: &str) -> JsValue {
        rust("client.duel.query", &[self.state.clone(), property.into()])
    }
    fn change(&mut self, method: &str, value: JsValue) -> JsValue {
        let updated = rust(
            &format!("client.duel.{method}"),
            &[self.state.clone(), value],
        );
        // Fighters are stable caller objects, as in the former JS facade.
        let next = get(&updated, "state");
        set(&self.state, "match", &get(&next, "match"));
        set(&self.state, "draft", &get(&next, "draft"));
        get(&updated, "result")
    }
}
