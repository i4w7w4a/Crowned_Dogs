//! Small Three/DOM boundary shared by the Rust preview controllers.
//! These functions only access platform objects; lifecycle and decisions live in Rust.
use js_sys::{Array, Function, Object, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt, closure::Closure};
use wasm_bindgen_futures::JsFuture;

pub fn object() -> JsValue {
    Object::new().into()
}
pub fn text(v: &str) -> JsValue {
    JsValue::from_str(v)
}
pub fn num(v: f64) -> JsValue {
    JsValue::from_f64(v)
}
pub fn flag(v: bool) -> JsValue {
    JsValue::from_bool(v)
}
pub fn get(o: &JsValue, name: &str) -> JsValue {
    name.split('.').fold(o.clone(), |v, key| {
        if !v.is_object() && !v.is_function() {
            JsValue::UNDEFINED
        } else {
            Reflect::get(&v, &super::bridge::cached_key(key)).unwrap_throw()
        }
    })
}
pub fn set(o: &JsValue, name: &str, v: &JsValue) {
    if let Some((path, key)) = name.rsplit_once('.') {
        set(&get(o, path), key, v);
    } else {
        Reflect::set(o, &super::bridge::cached_key(name), v).unwrap_throw();
    }
}
pub fn number(o: &JsValue, name: &str) -> f64 {
    get(o, name).as_f64().unwrap_or(0.0)
}
pub fn string(o: &JsValue, name: &str) -> String {
    get(o, name).as_string().unwrap_or_default()
}
pub fn yes(o: &JsValue, name: &str) -> bool {
    get(o, name).is_truthy()
}
pub fn exists(v: &JsValue) -> bool {
    !v.is_null() && !v.is_undefined()
}
pub fn array(values: &[JsValue]) -> JsValue {
    values.iter().cloned().collect::<Array>().into()
}
pub fn floats(values: &[f64]) -> JsValue {
    array(&values.iter().copied().map(num).collect::<Vec<_>>())
}
pub fn options(values: &[(&str, JsValue)]) -> JsValue {
    let o = object();
    for (k, v) in values {
        set(&o, k, v);
    }
    o
}
pub fn invoke(f: &JsValue, this: &JsValue, args: &[JsValue]) -> JsValue {
    f.unchecked_ref::<Function>()
        .apply(this, &Array::from(&array(args)))
        .unwrap_throw()
}
pub fn call(o: &JsValue, name: &str, args: &[JsValue]) -> JsValue {
    invoke(&get(o, name), o, args)
}
pub fn try_call(o: &JsValue, name: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    get(o, name)
        .unchecked_ref::<Function>()
        .apply(o, &Array::from(&array(args)))
}
pub fn make(host: &JsValue, name: &str, args: &[JsValue]) -> JsValue {
    Reflect::construct(
        get(host, name).unchecked_ref::<Function>(),
        &Array::from(&array(args)),
    )
    .unwrap_throw()
}
pub fn try_make(host: &JsValue, name: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    Reflect::construct(
        get(host, name).unchecked_ref::<Function>(),
        &Array::from(&array(args)),
    )
}
pub fn global() -> JsValue {
    js_sys::global().into()
}
pub fn document() -> JsValue {
    get(&global(), "document")
}
pub fn query(root: &JsValue, selector: &str) -> JsValue {
    call(root, "querySelector", &[text(selector)])
}
pub fn class(root: &JsValue, op: &str, name: &str) {
    call(&get(root, "classList"), op, &[text(name)]);
}
pub fn attr(root: &JsValue, name: &str, value: &str) {
    call(root, "setAttribute", &[text(name), text(value)]);
}
pub fn method_if(o: &JsValue, method: &str, args: &[JsValue]) {
    if exists(o) {
        call(o, method, args);
    }
}
pub fn components(v: &JsValue) -> Vec<f64> {
    Array::from(v)
        .iter()
        .map(|n| n.as_f64().unwrap_or(0.0))
        .collect()
}
pub fn json(v: &JsValue) -> serde_json::Value {
    serde_json::from_str(
        &js_sys::JSON::stringify(v)
            .unwrap_throw()
            .as_string()
            .unwrap(),
    )
    .unwrap()
}
pub fn from_json(v: serde_json::Value) -> JsValue {
    js_sys::JSON::parse(&v.to_string()).unwrap_throw()
}
pub fn graphics(method: &str, args: &JsValue) -> JsValue {
    from_json(game_graphics::dispatch(method, json(args)).unwrap())
}
pub fn ui(method: &str, args: &JsValue) -> JsValue {
    from_json(game_ui::dispatch(method, json(args)).unwrap())
}
pub async fn wait(v: JsValue) -> Result<JsValue, JsValue> {
    JsFuture::from(Promise::resolve(&v)).await
}
pub fn function(mut f: impl FnMut(JsValue) + 'static) -> JsValue {
    Closure::wrap(Box::new(move |v: JsValue| f(v)) as Box<dyn FnMut(JsValue)>).into_js_value()
}
pub fn listen(state: &JsValue, target: &JsValue, event: &str, callback: JsValue) {
    call(target, "addEventListener", &[text(event), callback.clone()]);
    Array::from(&get(state, "listeners")).push(&array(&[target.clone(), text(event), callback]));
}
pub fn unlisten(state: &JsValue) {
    for entry in Array::from(&get(state, "listeners")).iter() {
        let a = Array::from(&entry);
        call(&a.get(0), "removeEventListener", &[a.get(1), a.get(2)]);
    }
    set(state, "listeners", &Array::new().into());
}
pub fn observe(state: &JsValue, target: &JsValue, callback: JsValue) {
    let observer = make(&global(), "ResizeObserver", &[callback]);
    call(&observer, "observe", &[target.clone()]);
    set(state, "observer", &observer);
}
pub fn walk(root: &JsValue, visible: bool, out: &mut Vec<JsValue>) {
    if !exists(root) || visible && !yes(root, "visible") {
        return;
    }
    out.push(root.clone());
    for child in Array::from(&get(root, "children")).iter() {
        walk(&child, visible, out);
    }
}
pub fn nodes(root: &JsValue, visible: bool) -> Vec<JsValue> {
    let mut out = Vec::new();
    walk(root, visible, &mut out);
    out
}
pub fn unique(out: &mut Vec<JsValue>, v: JsValue) {
    if !out.iter().any(|a| a == &v) {
        out.push(v);
    }
}
pub fn materials(root: &JsValue) -> Vec<JsValue> {
    let mut out = Vec::new();
    for node in nodes(root, false) {
        if !yes(&node, "isMesh") {
            continue;
        }
        let m = get(&node, "material");
        if Array::is_array(&m) {
            for m in Array::from(&m).iter() {
                unique(&mut out, m);
            }
        } else if exists(&m) {
            unique(&mut out, m);
        }
    }
    out
}
pub fn textures(root: &JsValue) -> Vec<JsValue> {
    let mut out = Vec::new();
    for m in materials(root) {
        for v in Object::values(m.unchecked_ref::<Object>()).iter() {
            if yes(&v, "isTexture") {
                unique(&mut out, v);
            }
        }
    }
    out
}
pub fn dispose(root: &JsValue, extra: &[JsValue], close_images: bool) {
    let mut geometries = Vec::new();
    for n in nodes(root, false) {
        if yes(&n, "isMesh") {
            unique(&mut geometries, get(&n, "geometry"));
        }
    }
    let mut tex = textures(root);
    for t in extra {
        unique(&mut tex, t.clone());
    }
    for g in geometries {
        method_if(&g, "dispose", &[]);
    }
    for m in materials(root) {
        method_if(&m, "dispose", &[]);
    }
    let mut images = Vec::new();
    for t in tex {
        let image = get(&t, "image");
        if close_images && get(&image, "close").is_function() && !images.contains(&image) {
            call(&image, "close", &[]);
            images.push(image);
        }
        method_if(&t, "dispose", &[]);
    }
}
pub fn bounds(host: &JsValue, root: &JsValue) -> JsValue {
    let b = make(host, "THREE.Box3", &[]);
    for n in nodes(root, true) {
        if !yes(&n, "isMesh") {
            continue;
        }
        let g = get(&n, "geometry");
        call(&g, "computeBoundingBox", &[]);
        let gb = get(&g, "boundingBox");
        if exists(&gb) {
            let transformed = call(&gb, "clone", &[]);
            call(&transformed, "applyMatrix4", &[get(&n, "matrixWorld")]);
            call(&b, "union", &[transformed]);
        }
    }
    b
}
pub fn error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}
