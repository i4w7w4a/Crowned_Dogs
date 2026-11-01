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

