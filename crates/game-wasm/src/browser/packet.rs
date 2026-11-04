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
    /// A Three matrix's 16 elements into `out`.
    pub fn matrix(m: &JsValue, out: &mut [f64]);
}
