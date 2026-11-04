//! Frame packets: hot per-frame numbers cross the Rust/JS boundary as one typed array per batch,
//! not as a host call per object or per number.
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
export function read_trs(objs, out) {
  for (let i = 0, k = 0, l = objs.length; i < l; i++, k += 7) {
    const o = objs[i], p = o.position, q = o.quaternion;
    out[k] = p.x; out[k + 1] = p.y; out[k + 2] = p.z;
    out[k + 3] = q._x; out[k + 4] = q._y; out[k + 5] = q._z; out[k + 6] = q._w;
  }
}
export function write_trs(objs, src) {
  for (let i = 0, k = 0, l = objs.length; i < l; i++, k += 7) {
    const o = objs[i];
    o.position.set(src[k], src[k + 1], src[k + 2]);
    o.quaternion.set(src[k + 3], src[k + 4], src[k + 5], src[k + 6]);
  }
}
export function read_quats(qs, out) {
  for (let i = 0, k = 0, l = qs.length; i < l; i++, k += 4) {
    const q = qs[i];
    out[k] = q._x; out[k + 1] = q._y; out[k + 2] = q._z; out[k + 3] = q._w;
  }
}
export function write_quats_at(qs, at, src) {
  for (let i = 0, l = at.length; i < l; i++) {
    const k = i * 4;
    qs[at[i]].set(src[k], src[k + 1], src[k + 2], src[k + 3]);
  }
}
export function fill(v, out) {
  if (Array.isArray(v) || ArrayBuffer.isView(v)) {
    if (v.length > out.length) return v.length;
    for (let i = 0; i < v.length; i++) out[i] = v[i];
    return v.length;
  }
  if (v.isMatrix4) { out.set(v.elements); return 16; }
  v.toArray(out);
  return v.isMatrix3 ? 9 : v.isQuaternion || v.isVector4 || v.isEuler ? 4 : v.isVector2 ? 2 : 3;
}
export function world_of(o, out) {
  out.set(o.matrixWorld.elements);
}
export function parent_world_of(o, out) {
  out.set(o.parent.matrixWorld.elements);
}
export function set_quat(o, src) {
  o.quaternion.set(src[0], src[1], src[2], src[3]);
}
export function quat_of(o, out) {
  const q = o.quaternion;
  out[0] = q._x; out[1] = q._y; out[2] = q._z; out[3] = q._w;
}
export function xyz_of(v, out) {
  if (v == null) { out[0] = out[1] = out[2] = 0; return; }
  out[0] = v.x ?? 0; out[1] = v.y ?? 0; out[2] = v.z ?? 0;
}
export function world_pos_of(o, out) {
  // Object3D.getWorldPosition, minus the vector: refreshes the object's world matrix and reads it
  o.updateWorldMatrix(true, false);
  const e = o.matrixWorld.elements;
  out[0] = e[12]; out[1] = e[13]; out[2] = e[14];
}
export function set_pos(o, src) {
  o.position.set(src[0], src[1], src[2]);
}
export function place_frame(enemy, me, lights, q) {
  enemy.position.set(q[0], q[1], q[2]);
  me.position.set(q[4], q[5], q[6]);
  for (let i = 0, k = 8; i < lights.length; i++, k += 3) lights[i].position.set(q[k], q[k + 1], q[k + 2]);
  enemy.rotation.y = q[3];
  me.rotation.y = q[7];
}
export function put_uniform(m, key, v) {
  m.uniforms[key].value = v;
}
export function matrix(m, out) {
  out.set(m.elements);
}
"#)]
extern "C" {
    /// Each object's position and quaternion, 7 numbers apiece, into `out`.
    pub fn read_trs(objs: &Array, out: &mut [f64]);
    /// Puts back what `read_trs` read (Three's own setters, so change callbacks still run).
    pub fn write_trs(objs: &Array, src: &[f64]);
    /// Each quaternion's x, y, z, w into `out`.
    pub fn read_quats(qs: &Array, out: &mut [f64]);
    /// Sets quaternion `at[i]` from `src[4i..4i + 4]`.
    pub fn write_quats_at(qs: &Array, at: &[u32], src: &[f64]);
    /// A Three vector, quaternion, Euler or matrix's `toArray` into `out` (16 long); returns its length.
    /// Plain arrays are copied too; one longer than `out` is left and only its length returned.
    pub fn fill(v: &JsValue, out: &mut [f64]) -> u32;
    /// An object's world matrix (as Three last computed it) into `out`.
    pub fn world_of(o: &JsValue, out: &mut [f64]);
    /// The object's parent's world matrix into `out`.
    pub fn parent_world_of(o: &JsValue, out: &mut [f64]);
    /// Sets the object's own quaternion from `src[0..4]`.
    pub fn set_quat(o: &JsValue, src: &[f64]);
    /// The object's own quaternion into `out[0..4]`.
    pub fn quat_of(o: &JsValue, out: &mut [f64]);
    /// A vector's (or any object's) x, y, z into `out`.
    pub fn xyz_of(v: &JsValue, out: &mut [f64]);
    /// `getWorldPosition` without a vector: the object's world position into `out[0..3]`.
    pub fn world_pos_of(o: &JsValue, out: &mut [f64]);
    /// Sets the object's position from `src[0..3]`.
    pub fn set_pos(o: &JsValue, src: &[f64]);
    /// The battle's placement in one call: both fighters' positions (`q[0..3]`, `q[4..7]`) and
    /// turns (`q[3]`, `q[7]`), then each light's position from `q[8]` on, three numbers apiece.
    pub fn place_frame(enemy: &JsValue, me: &JsValue, lights: &js_sys::Array, q: &[f64]);
    /// `m.uniforms[key].value = v` in one call; `key` comes from the bridge's name cache.
    pub fn put_uniform(m: &JsValue, key: &JsValue, v: &JsValue);
    /// A Three matrix's 16 elements into `out`.
    pub fn matrix(m: &JsValue, out: &mut [f64]);
}
