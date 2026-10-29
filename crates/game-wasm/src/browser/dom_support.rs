//! Browser API boundary shared by the Rust screen controllers. No JavaScript is evaluated.
use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

pub fn js(value: &str) -> JsValue {
    JsValue::from_str(value)
}
pub fn number(value: f64) -> JsValue {
    JsValue::from_f64(value)
}
pub fn flag(value: bool) -> JsValue {
    JsValue::from_bool(value)
}
pub fn global() -> JsValue {
    js_sys::global().into()
}
pub fn document() -> JsValue {
    get(&global(), "document")
}
pub fn get(object: &JsValue, key: &str) -> JsValue {
    Reflect::get(object, &js(key)).unwrap_or(JsValue::UNDEFINED)
}
pub fn set(object: &JsValue, key: &str, value: &JsValue) {
    Reflect::set(object, &js(key), value).unwrap_throw();
}
pub fn text(value: &JsValue) -> String {
    value.as_string().unwrap_or_default()
}
pub fn num(object: &JsValue, key: &str) -> f64 {
    get(object, key).as_f64().unwrap_or(0.0)
}
pub fn boolean(object: &JsValue, key: &str) -> bool {
    get(object, key).as_bool().unwrap_or(false)
}
pub fn exists(value: &JsValue) -> bool {
    !value.is_null() && !value.is_undefined()
}
pub fn args(values: &[JsValue]) -> Array {
    values.iter().collect()
}
pub fn invoke(object: &JsValue, key: &str, values: &[JsValue]) -> Result<JsValue, JsValue> {
    get(object, key)
        .dyn_into::<Function>()?
        .apply(object, &args(values))
}
pub fn call(object: &JsValue, key: &str, values: &[JsValue]) -> JsValue {
    invoke(object, key, values).unwrap_throw()
}
pub fn call_optional(object: &JsValue, key: &str, values: &[JsValue]) {
    if exists(object) {
        call(object, key, values);
    }
}
pub fn construct(host: &JsValue, key: &str, values: &[JsValue]) -> Result<JsValue, JsValue> {
    Reflect::construct(&get(host, key).dyn_into::<Function>()?, &args(values))
}
pub fn query(root: &JsValue, selector: &str) -> JsValue {
    call(root, "querySelector", &[js(selector)])
}
pub fn all(root: &JsValue, selector: &str) -> Vec<JsValue> {
    Array::from(&call(root, "querySelectorAll", &[js(selector)]))
        .iter()
        .collect()
}
pub fn create(tag: &str, class: &str) -> JsValue {
    let el = call(&document(), "createElement", &[js(tag)]);
    set(&el, "className", &js(class));
    el
}
pub fn html(el: &JsValue, value: &str) {
    set(el, "innerHTML", &js(value));
}
pub fn content(el: &JsValue, value: &str) {
    set(el, "textContent", &js(value));
}
pub fn hidden(el: &JsValue, value: bool) {
    set(el, "hidden", &flag(value));
}
pub fn attr(el: &JsValue, key: &str, value: &str) {
    call(el, "setAttribute", &[js(key), js(value)]);
}
pub fn toggle(el: &JsValue, class: &str, value: bool) {
    call(&get(el, "classList"), "toggle", &[js(class), flag(value)]);
}
pub fn data(el: &JsValue, key: &str) -> String {
    text(&get(&get(el, "dataset"), key))
}
pub fn style(el: &JsValue, key: &str, value: &str) {
    call(&get(el, "style"), "setProperty", &[js(key), js(value)]);
}
pub fn focus(el: &JsValue) {
    if exists(el) {
        let options = Object::new();
        set(&options, "preventScroll", &flag(true));
        call(el, "focus", &[options.into()]);
    }
}
pub fn closest(event: &JsValue, selector: &str) -> JsValue {
    let target = get(event, "target");
    if get(&target, "closest").is_function() {
        call(&target, "closest", &[js(selector)])
    } else {
        JsValue::NULL
    }
}
pub fn outside(event: &JsValue, element: &JsValue) -> bool {
    if get(event, "target") != *element {
        return false;
    }
    let r = call(element, "getBoundingClientRect", &[]);
    let x = num(event, "clientX");
    let y = num(event, "clientY");
    x < num(&r, "left") || x > num(&r, "right") || y < num(&r, "top") || y > num(&r, "bottom")
}
pub fn error(value: &JsValue, fallback: &str) -> String {
    if let Some(message) = value.as_string() {
        return message;
    }
    let message = text(&get(value, "message"));
    if message.is_empty() {
        fallback.into()
    } else {
        message
    }
}
pub fn rust(method: &str, values: &[JsValue]) -> JsValue {
    try_rust(method, values).unwrap_throw()
}
pub fn try_rust(method: &str, values: &[JsValue]) -> Result<JsValue, JsValue> {
    let input = js_sys::JSON::stringify(&args(values))?
        .as_string()
        .unwrap_or_default();
    js_sys::JSON::parse(&crate::dispatch(method, &input)?)
}

pub struct Listener {
    target: JsValue,
    kind: String,
    callback: Closure<dyn FnMut(JsValue)>,
}
impl Listener {
    pub fn new(target: &JsValue, kind: &str, handler: impl FnMut(JsValue) + 'static) -> Self {
        Self::with_passive(target, kind, handler, false)
    }
    pub fn passive(target: &JsValue, kind: &str, handler: impl FnMut(JsValue) + 'static) -> Self {
        Self::with_passive(target, kind, handler, true)
    }
    fn with_passive(
        target: &JsValue,
        kind: &str,
        handler: impl FnMut(JsValue) + 'static,
        passive: bool,
    ) -> Self {
        let callback = Closure::wrap(Box::new(handler) as Box<dyn FnMut(JsValue)>);
        let options = Object::new();
        set(&options, "passive", &flag(passive));
        call(
            target,
            "addEventListener",
            &[js(kind), callback.as_ref().clone(), options.into()],
        );
        Self {
            target: target.clone(),
            kind: kind.into(),
            callback,
        }
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        let _ = invoke(
            &self.target,
            "removeEventListener",
            &[js(&self.kind), self.callback.as_ref().clone()],
        );
    }
}
pub use wasm_bindgen::UnwrapThrowExt;
