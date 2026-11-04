//! Optional male beards and the hairstyles: rigid children of the head bone, hidden until a look
//! picks one. Each beard (one face, one style) and each hairstyle is its own small file
//! (`apps/web/scripts/split-packs.ts`), downloaded only when a look on this Persona asks for it.
use super::bridge::*;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};

/// Female hairstyles, each `models/hair-female-<style>.glb` with its `Hair_<style>_*` meshes.
pub(super) const HAIR_STYLES: [&str; 3] = ["pixie", "bob", "braid"];
/// Male hairstyles, each `models/hair-male-<style>.glb`, the same way.
pub(super) const MALE_HAIR_STYLES: [&str; 4] = ["crop", "slick", "medium", "ponytail"];
/// Beard styles; a beard is fitted to each of the four male faces: `models/beard-m<face>-<style>.glb`.
pub(super) const BEARD_STYLES: [&str; 5] = ["boxed", "goatee", "full", "tapered", "stubble"];

/// The hairstyles of one Persona.
pub(super) fn hair_styles(sex: &str) -> &'static [&'static str] {
    if sex == "male" { &MALE_HAIR_STYLES } else { &HAIR_STYLES }
}

thread_local! {
    static HOST: RefCell<JsValue> = const { RefCell::new(JsValue::UNDEFINED) };
    /// one download per file, shared by every Persona that wears it
    static FILES: RefCell<HashMap<String, JsValue>> = RefCell::new(HashMap::new());
    /// the (Persona, file) pairs on their way, so repeated looks do not start a second download
    static ON_THE_WAY: RefCell<Vec<(JsValue, String)>> = const { RefCell::new(Vec::new()) };
}

/// Marks a freshly loaded Persona as one that takes beards and hairstyles. Nothing downloads yet.
pub(super) async fn install(host: &JsValue, scene: &JsValue, source: &str) {
    let (base, sex) = if let Some(base) = source.strip_suffix("models/persona-male.json") {
        (base, "male")
    } else if let Some(base) = source.strip_suffix("models/persona-female.json") {
        (base, "female")
    } else {
        return;
    };
    HOST.with(|h| *h.borrow_mut() = host.clone());
    let data = get(scene, "userData");
    set(&data, "cdPackBase", &base.into());
    set(&data, "cdPackSex", &sex.into());
    if sex == "male" {
        // the beard choices open at once; each beard arrives when picked
        set(&data, "beardsReady", &true.into());
    }
}

/// The beard and hairstyle `paint` shows on `model` and the mesh that tells each is in place.
fn pieces(model: &JsValue, paint: &JsValue) -> Vec<(String, String)> {
    let data = get(model, "userData");
    let sex = string(&get(paint, "sex"));
    if string(&get(&data, "cdPackSex")) != sex {
        return Vec::new();
    }
    let mut out = Vec::new();
    let (face, beard) = (string(&get(paint, "face")), string(&get(paint, "beard")));
    let face_ok = face.len() == 2 && face.starts_with('m') && ('1'..='4').contains(&face.chars().nth(1).unwrap_or('0'));
    if sex == "male" && face_ok && BEARD_STYLES.contains(&beard.as_str()) {
        out.push((format!("beard-{face}-{beard}"), format!("Beard_{face}_{beard}_cards")));
    }
    let style = string(&get(paint, "hair"));
    if hair_styles(&sex).contains(&style.as_str()) {
        out.push((format!("hair-{sex}-{style}"), format!("Hair_{style}_cards")));
    }
    out
}

/// Starts downloading what the look needs and `model` lacks; the look is painted again when it is in.
pub(super) fn want(model: &JsValue, paint: &JsValue) {
    // my own figure in battle keeps only its arms: no hair or beard to download
    if boolean(&get(&get(model, "userData"), "cdArmsOnly")) {
        return;
    }
    let base = get(&get(model, "userData"), "cdPackBase");
    let Some(base) = base.as_string() else { return };
    for (file, probe) in pieces(model, paint) {
        if present(&call(model, "getObjectByName", &[probe.as_str().into()])) {
            continue;
        }
        let busy = ON_THE_WAY.with(|w| w.borrow().iter().any(|(m, f)| js_sys::Object::is(m, model) && *f == file));
        if busy {
            continue;
        }
        ON_THE_WAY.with(|w| w.borrow_mut().push((model.clone(), file.clone())));
        let model = model.clone();
        let url = format!("{base}models/{file}.json");
        spawn_local(async move {
            let loaded = file_scene(&url).await;
            ON_THE_WAY.with(|w| w.borrow_mut().retain(|(m, f)| !(js_sys::Object::is(m, &model) && *f == file)));
            let Some(scene) = loaded else { return };
            wear(&model, &scene, &probe);
        });
    }
}

/// The file's scene, downloaded once; a failed download is forgotten so a later look tries again.
async fn file_scene(url: &str) -> Option<JsValue> {
    let promise = FILES.with(|f| f.borrow().get(url).cloned()).unwrap_or_else(|| {
        let host = HOST.with(|h| h.borrow().clone());
        let promise: JsValue = super::scene_assets::load_model(host, url.to_string()).into();
        FILES.with(|f| f.borrow_mut().insert(url.to_string(), promise.clone()));
        promise
    });
    match JsFuture::from(js_sys::Promise::from(promise)).await {
        Ok(gltf) => Some(get(&gltf, "scene")),
        Err(_) => {
            FILES.with(|f| f.borrow_mut().remove(url));
            None
        }
    }
}

/// Hangs a copy of the piece under `model`'s head bone, hidden, with materials of its own (the look
/// tints them per Persona), and paints the look again so the piece shows.
fn wear(model: &JsValue, scene: &JsValue, probe: &str) {
    if present(&call(model, "getObjectByName", &[probe.into()])) {
        return;
    }
    let anchor = call(model, "getObjectByName", &["head".into()]);
    if !present(&anchor) || !present(&call(scene, "getObjectByName", &[probe.into()])) {
        return;
    }
    let piece = call(scene, "clone", &[true.into()]);
    for node in super::scene_assets::nodes(&piece) {
        if boolean(&get(&node, "isMesh")) {
            let material = call(&get(&node, "material"), "clone", &[]);
            // what the arena and the stand give every mesh of a fresh figure
            set(&node, "castShadow", &true.into());
            if get(&material, "alphaTest").as_f64().unwrap_or(0.0) > 0.0 {
                set(&material, "alphaToCoverage", &true.into());
            }
            set(&node, "material", &material);
        }
    }
    hide(&piece);
    call(&anchor, "add", &[piece.clone()]);
    // the Persona on the stand tunes its materials to the stand's light and draws again
    let viewer = get(model, "cdViewer");
    if present(&viewer) {
        super::persona::piece_arrived(&viewer, &piece);
    }
    let look = get(&get(model, "userData"), "cdLook");
    if present(&look) {
        super::look::apply(model.clone(), look);
    }
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
