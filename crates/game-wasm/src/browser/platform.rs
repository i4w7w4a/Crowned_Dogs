//! Storage, device selection and the local duel object all run on the WASM side.
use super::{bridge::*, dom_support::{rust, try_rust}};
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
    let mirror = supplied.is_undefined();
    let storage = if mirror { storage() } else { supplied };
    if !present(&saved) || !present(&storage) {
        return false;
    }
    let Some(raw) = js_sys::JSON::stringify(&saved).ok().and_then(|raw| raw.as_string()) else {
        return false;
    };
    // The live loadout key stays what battle reads; each save also lands in the active kit's own slot.
    if mirror {
        store_set(&kit_key(active_kit()), &raw);
    }
    try_call(&storage, "setItem", &[LOADOUT_KEY.into(), raw.into()]).is_ok()
}

const LOOK_KEY: &str = "crowned-dogs:persona:v1";
/// The created Persona, or null before the player has made one.
#[wasm_bindgen]
pub fn load_local_look() -> JsValue {
    rust("client.look.load", &[store_get(LOOK_KEY)])
}
#[wasm_bindgen]
pub fn save_local_look(look: JsValue) -> bool {
    let saved = rust("client.look.save", &[look]);
    let Some(raw) = present(&saved)
        .then(|| js_sys::JSON::stringify(&saved).ok().and_then(|raw| raw.as_string()))
        .flatten()
    else {
        return false;
    };
    try_call(&storage(), "setItem", &[LOOK_KEY.into(), raw.into()]).is_ok()
}
/// The look the game paints: the saved one, or the default male Persona.
pub fn current_look() -> JsValue {
    let look = load_local_look();
    if present(&look) {
        look
    } else {
        rust("client.look.normalize", &[js_sys::Object::new().into()])
    }
}

/// Five prepared equipment sets. The active one is mirrored into `LOADOUT_KEY`.
pub const KITS: usize = 5;
const ACTIVE_KIT_KEY: &str = "crowned-dogs:kit:v1";
fn kit_key(index: usize) -> String {
    format!("{LOADOUT_KEY}:kit{index}")
}
pub fn active_kit() -> usize {
    store_get(ACTIVE_KIT_KEY)
        .as_string()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|index| *index < KITS)
        .unwrap_or(0)
}
/// The loadout stored in one kit; an untouched kit starts as the starter loadout.
pub fn kit_loadout(index: usize) -> JsValue {
    if index == active_kit() {
        return load_local_loadout(JsValue::UNDEFINED);
    }
    rust("client.load", &[store_get(&kit_key(index))])
}
/// Makes another kit active and returns its loadout.
pub fn switch_kit(index: usize) -> JsValue {
    let index = index.min(KITS - 1);
    let current = active_kit();
    if index == current {
        return load_local_loadout(JsValue::UNDEFINED);
    }
    if let Some(raw) = store_get(LOADOUT_KEY).as_string() {
        store_set(&kit_key(current), &raw);
    }
    let loadout = kit_loadout(index);
    store_set(ACTIVE_KIT_KEY, &index.to_string());
    save_local_loadout(loadout.clone(), JsValue::UNDEFINED);
    loadout
}
/// Each gun keeps its fitted mods per kit, so swapping guns and back restores them.
const GUN_MODS_KEY: &str = "crowned-dogs:gunmods:v1";
pub fn swap_gun(prev: &JsValue, next: JsValue) -> JsValue {
    let old = string(&get(&get(prev, "weapon1"), "item"));
    let new = string(&get(&get(&next, "weapon1"), "item"));
    if old.is_empty() || old == new {
        return next;
    }
    let key = format!("{GUN_MODS_KEY}:kit{}", active_kit());
    let memory = store_get(&key)
        .as_string()
        .and_then(|raw| js_sys::JSON::parse(&raw).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| js_sys::Object::new().into());
    let mods = get(&get(prev, "weapon1"), "mods");
    if present(&mods) {
        set(&memory, &old, &mods);
    } else {
        let _ = js_sys::Reflect::delete_property(&memory.clone().into(), &old.as_str().into());
    }
    if let Some(raw) = js_sys::JSON::stringify(&memory).ok().and_then(|r| r.as_string()) {
        store_set(&key, &raw);
    }
    let saved = get(&memory, &new);
    if !present(&saved) || present(&get(&get(&next, "weapon1"), "mods")) {
        return next;
    }
    let mut result = next;
    for slot in js_sys::Object::keys(&saved.clone().into()).iter() {
        let id = js_sys::Reflect::get(&saved, &slot).unwrap_or(JsValue::NULL);
        if let Ok(fitted) = try_rust("client.equipWeaponMod", &[result.clone(), slot, id]) {
            result = fitted;
        }
    }
    result
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
/// The graphics level on <html data-quality>, so CSS can drop the liquid glass on weak levels.
pub fn mark_page_quality(id: &str) {
    let root = get(&document(), "documentElement");
    if present(&root) {
        set(&get(&root, "dataset"), "quality", &id.into());
    }
}
#[wasm_bindgen]
pub fn start_quality(gl: JsValue) -> JsValue {
    let params = construct(
        &get(&global(), "URLSearchParams"),
        &[get(&get(&global(), "location"), "search")],
    );
    let started = rust(
        "quality.start",
        &[
            call(&params, "get", &["q".into()]),
            store_get("cd-quality"),
            device_facts(&gl),
        ],
    );
    mark_page_quality(&string(&get(&started, "id")));
    started
}
#[wasm_bindgen]
pub fn keep_quality(id: &str) {
    store_set("cd-quality", id);
    mark_page_quality(id);
    super::live::mark(id);
}
thread_local! {
    static GPU_NAME: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}
/// The graphics card's name as WebGL reports it, asked once on a throwaway context.
pub fn gpu_name() -> String {
    if let Some(name) = GPU_NAME.with(|c| c.borrow().clone()) {
        return name;
    }
    let gl = try_call(&get(&global(), "document"), "createElement", &["canvas".into()])
        .and_then(|canvas| try_call(&canvas, "getContext", &["webgl2".into()]))
        .unwrap_or(JsValue::NULL);
    let name = if present(&gl) {
        let name = string(&get(&device_facts(&gl), "gpu"));
        let lose = call(&gl, "getExtension", &["WEBGL_lose_context".into()]);
        if present(&lose) {
            call(&lose, "loseContext", &[]);
        }
        name
    } else {
        "нет WebGL 2".to_owned()
    };
    GPU_NAME.with(|c| *c.borrow_mut() = Some(name.clone()));
    name
}
thread_local! {
    static PICKED: std::cell::RefCell<Option<JsValue>> = const { std::cell::RefCell::new(None) };
}
/// The quality picked for this device, which decides the assets a page loads. Asked once per
/// page, on a throwaway WebGL context; without a page (tests, workers) there is no canvas to
/// ask, so an empty entry: the plain assets.
fn picked() -> JsValue {
    if let Some(quality) = PICKED.with(|c| c.borrow().clone()) {
        return quality;
    }
    let quality = match try_call(&get(&global(), "document"), "createElement", &["canvas".into()]) {
        Err(_) => js_sys::Object::new().into(),
        Ok(canvas) => {
            let gl = try_call(&canvas, "getContext", &["webgl2".into()]).unwrap_or(JsValue::NULL);
            let id = string(&get(&start_quality(gl.clone()), "id"));
            if present(&gl) {
                let lose = call(&gl, "getExtension", &["WEBGL_lose_context".into()]);
                if present(&lose) {
                    call(&lose, "loseContext", &[]);
                }
            }
            get(&get(&rust("client.data", &[]), "QUALITY"), &id)
        }
    };
    PICKED.with(|c| *c.borrow_mut() = Some(quality.clone()));
    quality
}
/// The id of the quality picked for this device (empty without a page).
pub fn quality_id() -> String {
    string(&get(&picked(), "id"))
}
/// Whether this device gets the double-size painted backgrounds: its quality draws the arena's backdrop sharp.
pub fn sharp_pictures() -> bool {
    boolean(&get(&picked(), "sharpBackdrop"))
}
/// Whether this device takes the light WebP copies of the big painted backgrounds (Minimum, Low and
/// Medium): about a sixth of the PNG's download, the same picture to the eye.
#[wasm_bindgen(js_name = litePictures)]
pub fn lite_pictures() -> bool {
    boolean(&get(&picked(), "litePictures"))
}
/// A page-relative picture path, or its light copy when this quality takes one and the build ships it
/// (asset-cache.ts answers through `cdLitePicture`).
pub fn picture(path: &str) -> String {
    let lite = get(&global(), "cdLitePicture");
    if !lite_pictures() || !lite.is_function() {
        return path.into();
    }
    invoke(&lite, &[path.into()]).as_string().unwrap_or_else(|| path.into())
}
/// Whether this device loads the figure models' copies with GPU-compressed maps.
pub fn compressed_textures() -> bool {
    boolean(&get(&picked(), "compressedTextures"))
}

/// A fresh UUID for a duel's seed. `crypto.randomUUID` exists only on https and localhost, so
/// a phone opening the game by its home-network address builds one from random bytes instead.
fn random_seed() -> String {
    let crypto = get(&global(), "crypto");
    if let Ok(id) = try_call(&crypto, "randomUUID", &[]) {
        return string(&id);
    }
    let mut bytes = [0u8; 16];
    let buffer = js_sys::Uint8Array::new_with_length(16);
    call(&crypto, "getRandomValues", &[buffer.clone().into()]);
    buffer.copy_to(&mut bytes);
    bytes[6] = bytes[6] & 0x0f | 0x40;
    bytes[8] = bytes[8] & 0x3f | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
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
            set(&opts, "seed", &random_seed().into());
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
