//! Small foreign-object boundary. State, scheduling and decisions stay in Rust.
use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};

pub fn get(object: &JsValue, key: &str) -> JsValue {
    if object.is_null() || object.is_undefined() {
        return JsValue::UNDEFINED;
    }
    Reflect::get(object, &key.into()).unwrap_throw()
}
pub fn set(object: &JsValue, key: &str, value: &JsValue) {
    Reflect::set(object, &key.into(), value).unwrap_throw();
}
pub fn call(object: &JsValue, key: &str, args: &[JsValue]) -> JsValue {
    try_call(object, key, args).unwrap_throw()
}
pub fn try_call(object: &JsValue, key: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let f = Reflect::get(object, &key.into())?;
    apply(&f, object, args)
}
pub fn apply(function: &JsValue, this: &JsValue, args: &[JsValue]) -> Result<JsValue, JsValue> {
    function
        .dyn_ref::<Function>()
        .ok_or_else(|| js_sys::Error::new("Expected callable host binding"))?
        .apply(this, &array(args))
}
pub fn invoke(function: &JsValue, args: &[JsValue]) -> JsValue {
    apply(function, &JsValue::UNDEFINED, args).unwrap_throw()
}
pub fn construct(constructor: &JsValue, args: &[JsValue]) -> JsValue {
    Reflect::construct(constructor.unchecked_ref::<Function>(), &array(args)).unwrap_throw()
}
pub fn array(values: &[JsValue]) -> Array {
    values.iter().collect()
}
pub fn object(values: &[(&str, JsValue)]) -> JsValue {
    let out = Object::new();
    for (key, value) in values {
        set(&out, key, value);
    }
    out.into()
}
pub fn num(value: &JsValue) -> f64 {
    value.as_f64().unwrap_or(0.0)
}
pub fn boolean(value: &JsValue) -> bool {
    value.as_bool().unwrap_or(false)
}
pub fn string(value: &JsValue) -> String {
    value.as_string().unwrap_or_default()
}
pub fn present(value: &JsValue) -> bool {
    !value.is_null() && !value.is_undefined()
}
pub fn at(value: &JsValue, i: usize) -> JsValue {
    Reflect::get(value, &(i as u32).into()).unwrap_throw()
}
pub fn values(value: &JsValue) -> Vec<JsValue> {
    Array::from(value).iter().collect()
}
pub fn global() -> JsValue {
    js_sys::global().into()
}
pub fn document() -> JsValue {
    get(&global(), "document")
}
pub fn json(value: &serde_json::Value) -> JsValue {
    js_sys::JSON::parse(&value.to_string()).unwrap_throw()
}
pub fn data(value: &JsValue) -> serde_json::Value {
    js_sys::JSON::stringify(value)
        .ok()
        .and_then(|s| s.as_string())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
