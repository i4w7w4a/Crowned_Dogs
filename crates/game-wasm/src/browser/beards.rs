//! Optional male beard geometry, shared by authored preview and animated arena, and the female
//! hairstyles pack: rigid children of the head bone, hidden until a look picks one.
use super::bridge::*;
use wasm_bindgen::prelude::*;

/// Female hairstyles in models/hair-female.glb, each a set of `Hair_<style>_*` meshes.
pub(super) const HAIR_STYLES: [&str; 3] = ["pixie", "bob", "braid"];

pub(super) async fn install(host: &JsValue, scene: &JsValue, source: &str) {
    if let Some(base) = source.strip_suffix("models/persona-female.json") {
        install_hair(host, scene, base).await;
        return;
    }
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
    let complete = (1..=4).all(|face| ["boxed", "goatee", "full", "tapered", "stubble"].iter()
        .all(|style| names.iter().any(|name| name.starts_with(&format!("Beard_m{face}_{style}_")))));
    if !present(&anchor) || !complete {
        super::view_utils::dispose(&pack, &[], true);
        return;
    }
    hide(&pack);
    call(&anchor, "add", &[pack]);
    set(&get(scene, "userData"), "beardsReady", &true.into());
}

/// Without the pack, or with a style missing, a look with a new hairstyle shows the long hair.
async fn install_hair(host: &JsValue, scene: &JsValue, base: &str) {
    let loaded = super::scene_assets::await_value(
        super::scene_assets::load_model(host.clone(), format!("{base}models/hair-female.json")).into(),
    ).await;
    let Ok(gltf) = loaded else { return; };
    let pack = get(&gltf, "scene");
    let anchor = call(scene, "getObjectByName", &["head".into()]);
    let names: Vec<String> = super::scene_assets::nodes(&pack).iter()
        .filter(|node| boolean(&get(node, "isMesh")))
        .map(|node| string(&get(node, "name"))).collect();
    let complete = HAIR_STYLES.iter().all(|style| names.iter().any(|name| *name == format!("Hair_{style}_cards")));
    if !present(&anchor) || !complete {
        super::view_utils::dispose(&pack, &[], true);
        return;
    }
    hide(&pack);
    call(&anchor, "add", &[pack]);
}

fn hide(pack: &JsValue) {
    for node in super::scene_assets::nodes(pack) {
        if boolean(&get(&node, "isMesh")) {
            set(&node, "visible", &false.into());
            set(&node, "frustumCulled", &false.into());
            let material = get(&node, "material");
            if boolean(&get(&material, "transparent")) {
                set(&material, "depthWrite", &false.into());
            }
        }
    }
}

