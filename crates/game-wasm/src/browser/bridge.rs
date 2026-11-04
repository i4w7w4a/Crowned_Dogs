//! Small foreign-object boundary. State, scheduling and decisions stay in Rust.
use js_sys::{Array, Function, Object, Reflect};
use std::{cell::RefCell, collections::HashMap, hash::BuildHasherDefault};
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};

/// FxHash: the property names are short and ours, so the default SipHash's flood guard only cost
/// time on every get and set.
#[derive(Default)]
struct Fx(u64);
impl std::hash::Hasher for Fx {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(b)).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}
type Keys = HashMap<String, JsValue, BuildHasherDefault<Fx>>;

thread_local! {
    // Property names cross into JavaScript once: decoding the same short strings every frame
    // was a seventh of the battle's CPU time.
    static KEYS: RefCell<Keys> = RefCell::new(Keys::with_capacity_and_hasher(1024, Default::default()));
}
/// The JavaScript string for a property name, cached; generated names stop being cached at 8k.
pub fn cached_key(name: &str) -> JsValue {
    KEYS.with(|keys| {
        if let Some(k) = keys.borrow().get(name) {
            return k.clone();
        }
        let k = JsValue::from_str(name);
        let mut keys = keys.borrow_mut();
        if keys.len() < 8192 {
            keys.insert(name.to_owned(), k.clone());
        }
        k
    })
}

pub fn get(object: &JsValue, key: &str) -> JsValue {
    if object.is_null() || object.is_undefined() {
        return JsValue::UNDEFINED;
    }
    Reflect::get(object, &cached_key(key)).unwrap_throw()
}
pub fn set(object: &JsValue, key: &str, value: &JsValue) {
    Reflect::set(object, &cached_key(key), value).unwrap_throw();
}
pub fn call(object: &JsValue, key: &str, args: &[JsValue]) -> JsValue {
    try_call(object, key, args).unwrap_throw()
}
pub fn try_call(object: &JsValue, key: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let f = Reflect::get(object, &cached_key(key))?;
    apply(&f, object, args)
}
pub fn apply(function: &JsValue, this: &JsValue, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let f = function
        .dyn_ref::<Function>()
        .ok_or_else(|| js_sys::Error::new("Expected callable host binding"))?;
    // short calls skip building an argument array
    match args {
        [] => f.call0(this),
        [a] => f.call1(this, a),
        [a, b] => f.call2(this, a, b),
        [a, b, c] => f.call3(this, a, b, c),
        _ => f.apply(this, &array(args)),
    }
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
