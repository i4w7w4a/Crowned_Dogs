//! Optional male beard geometry, shared by authored preview and animated arena.
use super::bridge::*;
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
    if !install_skin(host, scene, base).await {
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
async fn install_skin(host: &JsValue, scene: &JsValue, base: &str) -> bool {
    let loaded = super::scene_assets::await_value(super::scene_assets::load_model(
        host.clone(), format!("{base}models/beards-skin.json")).into()).await;
    let Ok(gltf) = loaded else { return false; };
    let pack = get(&gltf, "scene");
    let carriers = super::scene_assets::nodes(&pack);
    let heads = super::scene_assets::nodes(scene);
    if carriers.iter().filter(|n| boolean(&get(n, "isMesh"))).count() != 2 {
        super::view_utils::dispose(&pack, &[], true);
        return false;
    }
    let mut swaps = Vec::new();
    for id in ["m2", "m4"] {
        let carrier = carriers.iter().find(|node| string(&get(node, "name")) == format!("CleanHead_{id}"));
        let head = heads.iter().find(|node| string(&get(node, "name")) == format!("Head_{id}"));
        let (Some(carrier), Some(head)) = (carrier, head) else {
            super::view_utils::dispose(&pack, &[], true);
            return false;
        };
        let material = get(carrier, "material");
        if !present(&get(&material, "map")) || !present(&get(&material, "normalMap")) {
            super::view_utils::dispose(&pack, &[], true);
            return false;
        }
        swaps.push((head.clone(), material, get(head, "material"), id));
    }
    let slots = ["map", "normalMap", "roughnessMap", "metalnessMap", "aoMap"];
    let mut retired = Vec::new();
    let mut old_materials = Vec::new();
    for (head, material, old, id) in swaps {
        for slot in slots {
            let texture = get(&old, slot);
            if present(&texture) && !retired.contains(&texture) { retired.push(texture); }
        }
        set(&material, "name", &format!("M_Head_{id}").into());
        set(&get(&material, "userData"), "cdFixedTextures", &true.into());
        set(&head, "material", &material);
        if !old_materials.contains(&old) { old_materials.push(old); }
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
    true
}
