//! Close-up hair for the Persona creation screen. While a female look is made the game hair
//! cards give their geometry over to models/persona-female-hd.glb: about 2,100 strips that
//! follow Art's strands over a wide underlayer (cards_hd.py). The game mesh keeps its material
//! and skeleton, so the look applies unchanged. The head stays the game's: at full detail its
//! surface rose through the hair cap and speckled the nape and temples.
use super::view_utils::*;
use js_sys::{Float32Array, Float64Array, Uint16Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Game mesh, and its close-up stand-in in the HD file.
const PAIRS: [(&str, &str); 1] = [("FemHairCards", "FemHairCardsHD")];
/// Both files bind this bone alike; it carries the stand-in into the game mesh's space.
const ANCHOR: &str = "head";

/// Swaps the close-up geometry in while a female look is made, and the game's back otherwise.
pub fn update(s: &JsValue) {
    let figure = get(s, "figure");
    let on = yes(s, "creating") && string(s, "figureSex") == "female" && exists(&figure);
    if on && !exists(&get(s, "hd")) {
        load(s);
        return;
    }
    swap(&figure, &get(s, "hd"), on);
}
/// Puts the game geometry back on `figure`, so disposing it frees what it loaded.
pub fn restore(figure: &JsValue) {
    swap(figure, &JsValue::NULL, false);
}
/// Frees the close-up geometry; the figure gets its own back first.
pub fn release(s: &JsValue) {
    restore(&get(s, "figure"));
    let hd = get(s, "hd");
    for (game, _) in PAIRS {
        if exists(&hd) {
            method_if(&get(&hd, game), "dispose", &[]);
        }
    }
    set(s, "hd", &JsValue::NULL);
}
fn swap(figure: &JsValue, hd: &JsValue, on: bool) {
    if !exists(figure) {
        return;
    }
    for mesh in nodes(figure, false) {
        let name = string(&mesh, "name");
        if !yes(&mesh, "isSkinnedMesh") || !PAIRS.iter().any(|(game, _)| *game == name) {
            continue;
        }
        if !exists(&get(&mesh, "cdLowGeometry")) {
            set(&mesh, "cdLowGeometry", &get(&mesh, "geometry"));
        }
        let fine = if on { get(hd, &name) } else { JsValue::NULL };
        let want = if exists(&fine) { fine } else { get(&mesh, "cdLowGeometry") };
        if get(&mesh, "geometry") != want {
            set(&mesh, "geometry", &want);
            set(&mesh, "boundingSphere", &JsValue::NULL);
            set(&mesh, "boundingBox", &JsValue::NULL);
        }
    }
}
fn load(s: &JsValue) {
    if yes(s, "hdLoading") {
        return;
    }
    set(s, "hdLoading", &flag(true));
    let h = get(s, "host");
    let url = format!("{}models/persona-female-hd.json", string(&h, "base"));
    let a = s.clone();
    spawn_local(async move {
        let Ok(p) = try_call(&h, "loadModel", &[text(&url)]) else {
            return;
        };
        // A failed load leaves the flag up: the game meshes stay and nothing retries.
        let Ok(gltf) = wait(p).await else {
            return;
        };
        let scene = get(&gltf, "scene");
        let figure = get(&a, "figure");
        if yes(&a, "disposed") || string(&a, "figureSex") != "female" || !exists(&figure) {
            dispose(&scene, &[], false);
            set(&a, "hdLoading", &flag(false));
            return;
        }
        let hd = object();
        for (game, fine) in PAIRS {
            if let (Some(low), Some(high)) = (find(&figure, game), find(&scene, fine)) {
                if let Some(geometry) = fit(&h, &low, &high) {
                    set(&hd, game, &geometry);
                }
            }
        }
        for m in materials(&scene) {
            method_if(&m, "dispose", &[]);
        }
        set(&a, "hd", &hd);
        set(&a, "hdLoading", &flag(false));
        update(&a);
    });
}
fn find(root: &JsValue, name: &str) -> Option<JsValue> {
    nodes(root, false)
        .into_iter()
        .find(|n| yes(n, "isSkinnedMesh") && string(n, "name") == name)
}
fn bone_names(mesh: &JsValue) -> Vec<String> {
    js_sys::Array::from(&get(mesh, "skeleton.bones"))
        .iter()
        .map(|b| string(&b, "name"))
        .collect()
}
/// The stand-in's geometry moved into the game mesh's space and joint order:
/// world = bone x inverse x bind x p for both, so p_low = (inverse_low bind_low)^-1 inverse_high bind_high p_high.
fn fit(h: &JsValue, low: &JsValue, high: &JsValue) -> Option<JsValue> {
    let low_bones = bone_names(low);
    let high_bones = bone_names(high);
    let li = low_bones.iter().position(|b| b == ANCHOR)?;
    let hi = high_bones.iter().position(|b| b == ANCHOR)?;
    let carry = |mesh: &JsValue, i: usize| {
        let m = call(&get(&get(mesh, "skeleton.boneInverses"), &i.to_string()), "clone", &[]);
        call(&m, "multiply", &[get(mesh, "bindMatrix")])
    };
    let m = call(&call(&carry(low, li), "invert", &[]), "multiply", &[carry(high, hi)]);
    let e = Float64Array::new(&get(&m, "elements")).to_vec();
    let geometry = get(high, "geometry");
    let p = read(&get(&geometry, "attributes.position"), 3);
    let out = Float32Array::new_with_length(p.len() as u32);
    for (i, v) in p.chunks(3).enumerate() {
        for k in 0..3 {
            let w = e[k] * v[0] + e[4 + k] * v[1] + e[8 + k] * v[2] + e[12 + k];
            out.set_index((3 * i + k) as u32, w as f32);
        }
    }
    call(
        &geometry,
        "setAttribute",
        &[text("position"), make(h, "THREE.BufferAttribute", &[out.into(), num(3.0)])],
    );
    let map: Vec<usize> = high_bones
        .iter()
        .map(|b| low_bones.iter().position(|l| l == b).unwrap_or(li))
        .collect();
    if map.iter().enumerate().any(|(i, j)| i != *j) {
        let joints = read(&get(&geometry, "attributes.skinIndex"), 4);
        let out = Uint16Array::new_with_length(joints.len() as u32);
        for (i, j) in joints.iter().enumerate() {
            out.set_index(i as u32, map.get(*j as usize).copied().unwrap_or(li) as u16);
        }
        call(
            &geometry,
            "setAttribute",
            &[text("skinIndex"), make(h, "THREE.BufferAttribute", &[out.into(), num(4.0)])],
        );
    }
    set(&geometry, "boundingSphere", &JsValue::NULL);
    set(&geometry, "boundingBox", &JsValue::NULL);
    Some(geometry)
}
/// An attribute's values as plain numbers, whether interleaved, quantized or not.
fn read(attribute: &JsValue, items: usize) -> Vec<f64> {
    let count = number(attribute, "count") as usize;
    let interleaved = yes(attribute, "isInterleavedBufferAttribute");
    let source = get(attribute, if interleaved { "data.array" } else { "array" });
    let stride = number(attribute, if interleaved { "data.stride" } else { "itemSize" }) as usize;
    let offset = if interleaved { number(attribute, "offset") as usize } else { 0 };
    let scale = if yes(attribute, "normalized") {
        match string(&source, "constructor.name").as_str() {
            "Int8Array" => 127.0,
            "Uint8Array" => 255.0,
            "Int16Array" => 32767.0,
            "Uint16Array" => 65535.0,
            _ => 1.0,
        }
    } else {
        1.0
    };
    let raw = Float64Array::new(&source).to_vec();
    let mut out = Vec::with_capacity(count * items);
    for i in 0..count {
        for k in 0..items {
            let v = raw[i * stride + offset + k] / scale;
            out.push(if scale > 1.0 { v.max(-1.0) } else { v });
        }
    }
    out
}
