//! Optional male beard geometry, shared by authored preview and animated arena.
use super::bridge::*;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

pub(super) async fn install(host: &JsValue, scene: &JsValue, source: &str) {
    let (base, asset, animated) = if let Some(base) = source.strip_suffix("models/persona-male.json") {
        (base, "beards-male", true)
    } else if let Some(base) = source.strip_suffix("preview/persona-authored.json") {
        (base, "beards-authored", false)
    } else {
        return;
    };
    set(&get(scene, "userData"), "beardsReady", &false.into());
    let loaded = super::scene_assets::await_value(
        super::scene_assets::load_model(host.clone(), format!("{base}models/{asset}.json")).into(),
    ).await;
    let Ok(gltf) = loaded else { return; };
    let pack = get(&gltf, "scene");
    let anchor = if animated {
        call(scene, "getObjectByName", &["head".into()])
    } else {
        scene.clone()
    };
    let names: Vec<String> = super::scene_assets::nodes(&pack).iter()
        .filter(|node| boolean(&get(node, "isMesh")))
        .map(|node| string(&get(node, "name"))).collect();
    let complete = (1..=4).all(|face| ["boxed", "goatee", "full", "tapered"].iter()
        .all(|style| names.iter().any(|name| name.starts_with(&format!("Beard_m{face}_{style}_")))));
    if !present(&anchor) || !complete {
        super::view_utils::dispose(&pack, &[], true);
        return;
    }
    if !install_skin(host, scene, base, animated).await {
        super::view_utils::dispose(&pack, &[], true);
        return;
    }
    for node in super::scene_assets::nodes(&pack) {
        if boolean(&get(&node, "isMesh")) {
            set(&node, "visible", &false.into());
            set(&node, "frustumCulled", &false.into());
            let material = get(&node, "material");
            if boolean(&get(&material, "transparent")) {
                set(&material, "depthWrite", &false.into());
            }
        }
    }
    call(&anchor, "add", &[pack]);
    set(&get(scene, "userData"), "beardsReady", &true.into());
}

/// Clean foundations are an additive pack; the shared game models stay untouched.
async fn install_skin(host: &JsValue, scene: &JsValue, base: &str, animated: bool) -> bool {
    let loaded = super::scene_assets::await_value(super::scene_assets::load_model(
        host.clone(), format!("{base}models/beards-skin.json")).into()).await;
    let Ok(gltf) = loaded else { return false; };
    let pack = get(&gltf, "scene");
    call(scene, "updateMatrixWorld", &[true.into()]);
    call(&pack, "updateMatrixWorld", &[true.into()]);
    let carriers = super::scene_assets::nodes(&pack);
    let heads = super::scene_assets::nodes(scene);
    let expected = ["CleanHead_m2", "CleanHead_m4", "CleanNormals_m2", "CleanNormals_m4", "CleanNormalsAuthored_m2", "CleanNormalsAuthored_m4"];
    let mesh_names: Vec<_> = carriers.iter().filter(|n| boolean(&get(n, "isMesh"))).map(|n| string(&get(n, "name"))).collect();
    if mesh_names.len() != 6 || expected.iter().any(|name| mesh_names.iter().filter(|n| *n == name).count() != 1) {
        super::view_utils::dispose(&pack, &[], true);
        return false;
    }
    let mut swaps = Vec::new();
    let mut prepared = Vec::new();
    for id in ["m2", "m4"] {
        let carrier = carriers.iter().find(|node| string(&get(node, "name")) == format!("CleanHead_{id}"));
        let head = heads.iter().find(|node| string(&get(node, "name")) == format!("Head_{id}"));
        let (Some(carrier), Some(head)) = (carrier, head) else {
            for geometry in &prepared { call(geometry, "dispose", &[]); }
            super::view_utils::dispose(&pack, &[], true);
            return false;
        };
        let material = get(carrier, "material");
        if !present(&get(&material, "map")) || !present(&get(&material, "normalMap")) {
            for geometry in &prepared { call(geometry, "dispose", &[]); }
            super::view_utils::dispose(&pack, &[], true);
            return false;
        }
        let normal_name = format!("{}_{id}", if animated { "CleanNormals" } else { "CleanNormalsAuthored" });
        let normal = carriers.iter().find(|n| string(&get(n, "name")) == normal_name).unwrap();
        let Some(geometry) = repaired_geometry(host, head, normal) else {
            for geometry in &prepared { call(geometry, "dispose", &[]); }
            super::view_utils::dispose(&pack, &[], true);
            return false;
        };
        prepared.push(geometry.clone());
        swaps.push((head.clone(), material, get(head, "material"), id, geometry, get(head, "geometry")));
    }
    let slots = ["map", "normalMap", "roughnessMap", "metalnessMap", "aoMap"];
    let mut retired = Vec::new();
    let mut old_materials = Vec::new();
    let mut old_geometries = Vec::new();
    for (head, material, old, id, geometry, old_geometry) in swaps {
        for slot in slots {
            let texture = get(&old, slot);
            if present(&texture) && !retired.contains(&texture) { retired.push(texture); }
        }
        set(&material, "name", &format!("M_Head_{id}").into());
        set(&get(&material, "userData"), "cdFixedTextures", &true.into());
        set(&head, "material", &material);
        set(&head, "geometry", &geometry);
        if !old_materials.contains(&old) { old_materials.push(old); }
        if !old_geometries.contains(&old_geometry) { old_geometries.push(old_geometry); }
    }
    for geometry in old_geometries {
        if !heads.iter().any(|node| get(node, "geometry") == geometry) { call(&geometry, "dispose", &[]); }
    }
    for material in old_materials {
        if !heads.iter().any(|node| get(node, "material") == material) {
            call(&material, "dispose", &[]);
        }
    }
    // Imported textures have not yet been uploaded. Release only those no longer
    // referenced by any material, guarding shared ORM maps and other body parts.
    let mut closed = Vec::new();
    for texture in retired {
        let retained = heads.iter().any(|node| slots.iter().any(|slot|
            get(&get(node, "material"), slot) == texture));
        if !retained {
            call(&texture, "dispose", &[]);
            let image = get(&texture, "image");
            let still_used = heads.iter().any(|node| slots.iter().any(|slot|
                get(&get(&get(node, "material"), slot), "image") == image));
            if !still_used && !closed.contains(&image) && get(&image, "close").is_function() {
                call(&image, "close", &[]);
                closed.push(image);
            }
        }
    }
    let mut disposed = Vec::new();
    for node in carriers {
        let geometry = get(&node, "geometry");
        if present(&geometry) && !disposed.contains(&geometry) {
            call(&geometry, "dispose", &[]);
            disposed.push(geometry);
        }
    }
    // The four normal carriers are data only. Release their materials/textures,
    // preserving the two clean materials just adopted by the actual heads.
    let retained_materials = super::view_utils::materials(scene);
    let retained_textures = super::view_utils::textures(scene);
    for material in super::view_utils::materials(&pack) {
        if !retained_materials.contains(&material) { call(&material, "dispose", &[]); }
    }
    for texture in super::view_utils::textures(&pack) {
        if !retained_textures.contains(&texture) {
            call(&texture, "dispose", &[]);
            let image = get(&texture, "image");
            if !closed.contains(&image) && !retained_textures.iter().any(|t| get(t, "image") == image) && get(&image, "close").is_function() {
                call(&image, "close", &[]); closed.push(image);
            }
        }
    }
    true
}

// Blender's physical-rest export differs by up to 0.746µm after float transforms.
// A 2µm match accommodates that numeric drift without relaxing the 2mm repair cap.
const NORMAL_MATCH_EPSILON: f64 = 0.000_002;
const MAX_POSITION_REPAIR: f64 = 0.002;
fn vectors(attribute: &JsValue) -> Option<Vec<[f64; 3]>> {
    components::<3>(attribute)
}
fn components<const N: usize>(attribute: &JsValue) -> Option<Vec<[f64; N]>> {
    let count = get(attribute, "count").as_f64()?;
    if count < 1. || count.fract() != 0. || count > 1_000_000. || num(&get(attribute, "itemSize")) < N as f64 { return None; }
    (0..count as usize).map(|i| {
        let mut out = [0.; N];
        for (axis, method) in ["getX", "getY", "getZ", "getW"].iter().take(N).enumerate() {
            out[axis] = try_call(attribute, method, &[(i as u32).into()]).ok()?.as_f64()?;
            if !out[axis].is_finite() { return None; }
        }
        Some(out)
    }).collect()
}

type Matrix = [f64; 16];
fn matrix(value: &JsValue) -> Option<Matrix> {
    let values = js_sys::Float64Array::new(&get(value, "elements")).to_vec();
    if values.len() != 16 || !values.iter().all(|v| v.is_finite()) { return None; }
    values.try_into().ok()
}
fn point(m: &Matrix, p: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| m[k]*p[0] + m[4+k]*p[1] + m[8+k]*p[2] + m[12+k])
}
fn transpose_direction(m: &Matrix, p: [f64; 3]) -> Option<[f64; 3]> {
    let v: [f64; 3] = std::array::from_fn(|k| m[k*4]*p[0] + m[k*4+1]*p[1] + m[k*4+2]*p[2]);
    let length = v.iter().map(|v| v*v).sum::<f64>().sqrt();
    if !length.is_finite() || length < 1e-12 { return None; }
    Some(v.map(|v| v/length))
}
fn inverse(m: &Matrix) -> Option<Matrix> {
    let (a,b,c,d,e,f,g,h,i) = (m[0],m[4],m[8],m[1],m[5],m[9],m[2],m[6],m[10]);
    let determinant = a*(e*i-f*h) - b*(d*i-f*g) + c*(d*h-e*g);
    if !determinant.is_finite() || determinant.abs() < 1e-12 { return None; }
    let mut out = [(e*i-f*h)/determinant, (f*g-d*i)/determinant, (d*h-e*g)/determinant, 0.,
        (c*h-b*i)/determinant, (a*i-c*g)/determinant, (b*g-a*h)/determinant, 0.,
        (b*f-c*e)/determinant, (c*d-a*f)/determinant, (a*e-b*d)/determinant, 0., 0.,0.,0.,1.];
    let offset = point(&out, [m[12],m[13],m[14]]);
    for k in 0..3 { out[12+k] = -offset[k]; }
    Some(out)
}
/// Quantized skinned heads store raw normalized positions; their metre scale is
/// inside bone inverses. Use each vertex's real rest weights (including neck),
/// then return corrected data to the exact same bind space.
fn rest_frames(mesh: &JsValue, count: usize) -> Option<Vec<Matrix>> {
    let world = get(mesh, "matrixWorld");
    if !boolean(&get(mesh, "isSkinnedMesh")) { return Some(vec![matrix(&world)?; count]); }
    let skeleton = get(mesh, "skeleton");
    let bones = values(&get(&skeleton, "bones"));
    let inverses = get(&skeleton, "boneInverses");
    let outer = try_call(&world, "clone", &[]).ok()?;
    try_call(&outer, "multiply", &[get(mesh, "bindMatrixInverse")]).ok()?;
    let mut frames = Vec::new();
    for (i, bone) in bones.iter().enumerate() {
        let m = try_call(&outer, "clone", &[]).ok()?;
        for transform in [get(bone, "matrixWorld"), at(&inverses, i), get(mesh, "bindMatrix")] {
            try_call(&m, "multiply", &[transform]).ok()?;
        }
        frames.push(matrix(&m)?);
    }
    let attributes = get(&get(mesh, "geometry"), "attributes");
    let joints = components::<4>(&get(&attributes, "skinIndex"))?;
    let weights = components::<4>(&get(&attributes, "skinWeight"))?;
    if joints.len() != count || weights.len() != count { return None; }
    joints.iter().zip(weights).map(|(joints, weights)| {
        if (weights.iter().sum::<f64>()-1.).abs() > 1e-4 { return None; }
        let mut out = [0.; 16];
        for k in 0..4 {
            if weights[k] < 0. || joints[k] < 0. || joints[k].fract() != 0. { return None; }
            if weights[k] == 0. { continue; }
            let frame = frames.get(joints[k] as usize)?;
            for j in 0..16 { out[j] += frame[j]*weights[k]; }
        }
        out[15] = 1.;
        Some(out)
    }).collect()
}

/// Match physical rest positions, not vertex order: meshopt and UV seams can
/// reindex or duplicate vertices. Every sample must match within 2µm.
/// Coincident samples are averaged deterministically. Optional repair targets
/// may move scan seams by at most 2mm; UVs, weights, indices and morph data remain
/// those of the original head. None of the original geometry is mutated.
fn repaired_geometry(host: &JsValue, head: &JsValue, companion: &JsValue) -> Option<JsValue> {
    let original = get(head, "geometry");
    let carrier = get(companion, "geometry");
    let positions = vectors(&get(&get(&original, "attributes"), "position"))?;
    let frames = rest_frames(head, positions.len())?;
    let carry = matrix(&get(companion, "matrixWorld"))?;
    let carry_inverse = inverse(&carry)?;
    let samples: Vec<_> = vectors(&get(&get(&carrier, "attributes"), "position"))?.into_iter().map(|p| point(&carry, p)).collect();
    let normals: Vec<_> = vectors(&get(&get(&carrier, "attributes"), "normal"))?.into_iter().map(|n| transpose_direction(&carry_inverse, n)).collect::<Option<_>>()?;
    if samples.len() != normals.len() { return None; }
    // GLTFLoader lowercases custom glTF semantics; also accept an unprocessed
    // BufferGeometry fixture with the literal exported attribute name.
    let attributes = get(&carrier, "attributes");
    let lower = get(&attributes, "_clean_position");
    let target_attribute = if present(&lower) { lower } else { get(&attributes, "_CLEAN_POSITION") };
    let targets = if present(&target_attribute) { Some(vectors(&target_attribute)?.into_iter().map(|p| point(&carry, p)).collect::<Vec<_>>()) } else { None };
    if let Some(targets) = &targets {
        if targets.len() != samples.len() { return None; }
        if samples.iter().zip(targets).any(|(p, q)| (0..3).map(|k| (p[k]-q[k]).powi(2)).sum::<f64>() > MAX_POSITION_REPAIR.powi(2)) { return None; }
    }
    let cell = |p: [f64; 3]| p.map(|v| (v / NORMAL_MATCH_EPSILON).floor() as i64);
    let mut buckets: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
    for (i, p) in samples.iter().enumerate() {
        let len = normals[i].iter().map(|v| v*v).sum::<f64>();
        if len < 1e-12 { return None; }
        buckets.entry(cell(*p)).or_default().push(i);
    }
    let mut output = Vec::<f32>::with_capacity(positions.len()*3);
    let mut repaired_positions = Vec::<f32>::with_capacity(positions.len()*3);
    for (raw, frame) in positions.into_iter().zip(frames) {
        let p = point(&frame, raw);
        let base = cell(p);
        let mut normal = [0.; 3];
        let mut target = [0.; 3];
        let mut count = 0;
        for x in -1..=1 { for y in -1..=1 { for z in -1..=1 {
            if let Some(indices) = buckets.get(&[base[0]+x, base[1]+y, base[2]+z]) {
                for &i in indices {
                    let distance = (0..3).map(|k| (samples[i][k]-p[k]).powi(2)).sum::<f64>();
                    if distance <= NORMAL_MATCH_EPSILON.powi(2) {
                        let length = normals[i].iter().map(|v| v*v).sum::<f64>().sqrt();
                        for k in 0..3 { normal[k] += normals[i][k]/length; }
                        if let Some(targets) = &targets { for k in 0..3 { target[k] += targets[i][k]; } }
                        count += 1;
                    }
                }
            }
        } } }
        let length = normal.iter().map(|v| v*v).sum::<f64>().sqrt();
        if count == 0 || length < 1e-12 { return None; }
        output.extend(transpose_direction(&frame, normal)?.map(|v| v as f32));
        if targets.is_some() {
            let target = target.map(|v| v / count as f64);
            if (0..3).map(|k| (p[k]-target[k]).powi(2)).sum::<f64>() > MAX_POSITION_REPAIR.powi(2) { return None; }
            repaired_positions.extend(point(&inverse(&frame)?, target).map(|v| v as f32));
        }
    }
    let geometry = try_call(&original, "clone", &[]).ok()?;
    let attribute = construct(&get(&get(host, "THREE"), "BufferAttribute"), &[js_sys::Float32Array::from(output.as_slice()).into(), 3.into()]);
    let prepared = (|| -> Option<()> {
        try_call(&geometry, "setAttribute", &["normal".into(), attribute]).ok()?;
        if targets.is_some() {
            let position = construct(&get(&get(host, "THREE"), "BufferAttribute"), &[js_sys::Float32Array::from(repaired_positions.as_slice()).into(), 3.into()]);
            try_call(&geometry, "setAttribute", &["position".into(), position]).ok()?;
        }
        if present(&get(&geometry, "index")) && present(&get(&get(&geometry, "attributes"), "uv")) {
            try_call(&geometry, "computeTangents", &[]).ok()?;
            let tangent = get(&get(&geometry, "attributes"), "tangent");
            vectors(&tangent)?;
            for i in 0..num(&get(&tangent, "count")) as usize {
                if !try_call(&tangent, "getW", &[(i as u32).into()]).ok()?.as_f64()?.is_finite() { return None; }
            }
        } else {
            try_call(&geometry, "deleteAttribute", &["tangent".into()]).ok()?;
        }
        try_call(&geometry, "computeBoundingBox", &[]).ok()?;
        try_call(&geometry, "computeBoundingSphere", &[]).ok()?;
        Some(())
    })();
    if prepared.is_none() { call(&geometry, "dispose", &[]); return None; }
    Some(geometry)
}
