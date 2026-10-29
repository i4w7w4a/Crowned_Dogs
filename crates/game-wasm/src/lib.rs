use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
mod browser;

/// The same Rust implementations run in the browser and in the Windows webview.
/// Errors cross the bridge as exceptions, never as a silent JavaScript fallback.
#[wasm_bindgen]
pub fn dispatch(method: &str, args_json: &str) -> Result<String, JsValue> {
    let args = serde_json::from_str(args_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = if method.starts_with("icons.") {
        game_ui::icons::dispatch(method, args)
    } else if method.starts_with("graphics.") {
        game_graphics::dispatch(method, args)
    } else if method.starts_with("ui.") {
        game_ui::dispatch(method, args)
    } else if method.starts_with("motion.") {
        game_motion::dispatch(method, args)
    } else if method.starts_with("client.")
        || method.starts_with("audio.")
        || method.starts_with("words.")
        || method.starts_with("quality.")
        || method.starts_with("server.")
    {
        game_client::dispatch(method, args)
    } else {
        game_core::dispatch(method.strip_prefix("core.").unwrap_or(method), args)
    };
    result
        .and_then(|value| serde_json::to_string(&value).map_err(|e| e.to_string()))
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn graphics_texture(kind: &str, size: u32) -> Result<Vec<u8>, JsValue> {
    if !(32..=2048).contains(&size) {
        return Err(JsValue::from_str("Texture size must be 32..2048"));
    }
    game_graphics::texture(kind, size).map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn audio_samples(kind: &str, rate: u32, seconds: f64, seed: u32) -> Vec<f32> {
    game_client::audio_samples(kind, rate, seconds, seed)
}

#[wasm_bindgen]
pub fn motion_compute(kind: &str, input: &[f64]) -> Result<Vec<f64>, JsValue> {
    (if kind.starts_with("arena.") {
        game_client::arena_compute(kind, input)
    } else {
        game_motion::compute(kind, input)
    })
    .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn engine_version() -> String {
    format!("Crowned Dogs Rust {}", env!("CARGO_PKG_VERSION"))
}
