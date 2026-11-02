//! Paints the player's chosen look onto a loaded Persona: skin, suit, hair and eyes.
//! The fitting room, the creation screen and the arena all call `apply` on their own model.
use super::bridge::*;
use super::scene_assets::nodes;
use js_sys::WeakMap;
use wasm_bindgen::prelude::*;

thread_local! {
    /// Head and hair materials with a shader of their own; `scene_assets::tune` chains it in.
    static HOOKS: WeakMap = WeakMap::new();
}
/// Every face is a head mesh of its own; only the chosen one shows.
const HEAD: &str = "M_Head_";
const BODIES: [&str; 2] = ["M_Male_Body", "M_Fem_Body"];
const CAP: &str = "M_Fem_HairCap";

/// Shader hook for a head material, if it has one.
pub fn hook(material: &JsValue) -> JsValue {
    if !material.is_object() {
        return JsValue::UNDEFINED;
    }
    HOOKS.with(|h| h.get(material.unchecked_ref()))
}

/// The look's paint values, straight from the saved look.
pub fn paint(look: &JsValue) -> JsValue {
    super::dom_support::rust("client.look.paint", &[look.clone()])
}

fn rgb(value: &JsValue) -> [f64; 3] {
    let v = values(value);
    [0, 1, 2].map(|i| v.get(i).map(num).unwrap_or(1.0))
}
/// Multiplies the material's own colour, remembered the first time it is touched.
fn tint(material: &JsValue, factor: [f64; 3]) {
    let color = get(material, "color");
    if !present(&color) {
        return;
    }
    let mut base = get(&get(material, "userData"), "cdBase");
    if !present(&base) {
        base = array(&[get(&color, "r"), get(&color, "g"), get(&color, "b")]).into();
        set(&get(material, "userData"), "cdBase", &base);
    }
    let b = rgb(&base);
    call(
        &color,
        "setRGB",
        &[
            (b[0] * factor[0]).into(),
            (b[1] * factor[1]).into(),
            (b[2] * factor[2]).into(),
        ],
    );
}

/// 0 to 3 for the faces m1/f1 to m4/f4.
fn face_index(face: &str) -> Option<usize> {
    face.get(1..).and_then(|d| d.parse::<usize>().ok()).filter(|i| (1..=4).contains(i)).map(|i| i - 1)
}

/// The hair pack's mask of the male scans' baked buzz cut (`tools/hair-hd/scalp_mask.py`, one channel per face, R m1 to
/// A m4) and the mean brightness of each face's buzz. Under a pack hairstyle the head shader paints the buzz in the
/// hair's colour; a pack without the mask paints nothing.
struct Scalp {
    map: JsValue,
    luma: Vec<f64>,
    on: bool,
}

fn scalp(model: &JsValue, packed: bool) -> Scalp {
    let node = call(model, "getObjectByName", &["Hair_scalp".into()]);
    let map = if present(&node) { get(&get(&node, "material"), "map") } else { JsValue::NULL };
    let luma: Vec<f64> = if present(&node) {
        values(&get(&get(&node, "userData"), "cdScalpLuma")).iter().map(num).collect()
    } else {
        Vec::new()
    };
    if present(&map) && !string(&get(&map, "colorSpace")).is_empty() {
        // a mask, not a colour: sampled as stored
        set(&map, "colorSpace", &"".into());
        set(&map, "needsUpdate", &true.into());
    }
    let on = packed && present(&map) && luma.len() == 4;
    Scalp { map: if present(&map) { map } else { JsValue::NULL }, luma, on }
}

/// Sets up the eye shader on a head material; returns its uniforms. The head's model carries its two irises in the
/// mesh's own space (`cdEyes`: x, y, z, radius twice): the scans' UV atlases scatter an eye over several islands, so
/// the eyes are found by position, not by texture area. The same shader paints the scalp under pack hair (`Scalp`).
fn head_hook(material: &JsValue, node: &JsValue, paint: &JsValue, scalp: &Scalp) -> JsValue {
    let eyes = get(&get(node, "userData"), "cdEyes");
    let mut spots = if present(&eyes) { values(&eyes) } else { Vec::new() };
    spots.resize(8, 0.0.into());
    let area = |i: usize| array(&spots[i * 4..i * 4 + 4]).into();
    let mut uniforms = get(&get(material, "userData"), "cdLook");
    if !present(&uniforms) {
        uniforms = object(&[
            ("cdEyeA", object(&[("value", area(0))])),
            ("cdEyeB", object(&[("value", area(1))])),
            ("cdIris", object(&[("value", get(paint, "iris"))])),
            ("cdTime", object(&[("value", 0.0.into())])),
            ("cdScalp", object(&[("value", JsValue::NULL)])),
            ("cdScalpPick", object(&[("value", array(&[0.0.into(), 0.0.into(), 0.0.into(), 0.0.into()]).into())])),
            ("cdScalpTone", object(&[("value", array(&[0.0.into(), 0.0.into(), 0.0.into()]).into())])),
            ("cdScalpLuma", object(&[("value", 1.0.into())])),
        ]);
        set(&get(material, "userData"), "cdLook", &uniforms);
        let shared = uniforms.clone();
        let compile = Closure::<dyn FnMut(JsValue)>::new(move |shader: JsValue| {
            let own = get(&shader, "uniforms");
            for key in ["cdEyeA", "cdEyeB", "cdIris", "cdTime", "cdScalp", "cdScalpPick", "cdScalpTone", "cdScalpLuma"] {
                set(&own, key, &get(&shared, key));
            }
            let vertex = string(&get(&shader, "vertexShader"))
                .replacen("#include <common>", "#include <common>\nvarying vec3 cdLocal;", 1)
                .replacen("#include <begin_vertex>", "#include <begin_vertex>\ncdLocal = position;", 1);
            set(&shader, "vertexShader", &vertex.into());
            let fragment = string(&get(&shader, "fragmentShader"))
                .replacen(
                    "#include <common>",
                    "#include <common>\nvarying vec3 cdLocal;\nuniform vec4 cdEyeA;\nuniform vec4 cdEyeB;\nuniform vec3 cdIris;\nuniform float cdTime;\nuniform sampler2D cdScalp;\nuniform vec4 cdScalpPick;\nuniform vec3 cdScalpTone;\nuniform float cdScalpLuma;\nvec3 cdGlow = vec3( 0.0 );",
                    1,
                )
                .replacen(
                    "#include <map_fragment>",
                    concat!(
                        "#include <map_fragment>\n",
                        include_str!("shaders/look-eyes.glsl"),
                        include_str!("shaders/look-scalp.glsl")
                    ),
                    1,
                )
                .replacen(
                    "#include <emissivemap_fragment>",
                    "#include <emissivemap_fragment>\ntotalEmissiveRadiance += cdGlow;",
                    1,
                );
            set(&shader, "fragmentShader", &fragment.into());
        })
        .into_js_value();
        let hook = object(&[("key", "look5".into()), ("compile", compile.clone())]);
        HOOKS.with(|h| h.set(material.unchecked_ref(), &hook));
        // Until the next tune, the hook runs on its own; tune then chains it with the rest.
        set(material, "onBeforeCompile", &compile);
        set(material, "needsUpdate", &true.into());
    }
    set(&get(&uniforms, "cdEyeA"), "value", &area(0));
    set(&get(&uniforms, "cdEyeB"), "value", &area(1));
    set(&get(&uniforms, "cdIris"), "value", &get(paint, "iris"));
    // the face's own channel, from its material's name (M_Head_m3: blue)
    let channel = string(&get(material, "name"))
        .strip_prefix(HEAD)
        .filter(|face| face.starts_with('m'))
        .and_then(face_index);
    let mut pick = [0.0; 4];
    let mut luma = 1.0;
    if let (true, Some(i)) = (scalp.on, channel) {
        pick[i] = 1.0;
        luma = scalp.luma[i].max(0.001);
    }
    set(&get(&uniforms, "cdScalp"), "value", &scalp.map);
    set(&get(&uniforms, "cdScalpPick"), "value", &array(&pick.map(JsValue::from)).into());
    set(&get(&uniforms, "cdScalpTone"), "value", &get(paint, "hairCap"));
    set(&get(&uniforms, "cdScalpLuma"), "value", &luma.into());
    uniforms
}

/// Hair strips light the same from both sides: a strip seen from behind keeps the normal that
/// points away from the skull, so locks at the back of the silhouette do not go dark.
fn hair_hook(material: &JsValue) {
    if present(&hook(material)) {
        return;
    }
    // The strips' tips touch the edge of their texture. Repeating it filtered the far edge's dense
    // roots into each tip, which lay grey dashes across the nape.
    let map = get(material, "map");
    if present(&map) {
        for key in ["wrapS", "wrapT"] {
            set(&map, key, &1001.into()); // THREE.ClampToEdgeWrapping
        }
        set(&map, "needsUpdate", &true.into());
    }
    let compile = Closure::<dyn FnMut(JsValue)>::new(move |shader: JsValue| {
        let fragment = string(&get(&shader, "fragmentShader")).replacen(
            "#include <normal_fragment_begin>",
            "#include <normal_fragment_begin>
#ifdef DOUBLE_SIDED
normal *= faceDirection;
#endif",
            1,
        );
        set(&shader, "fragmentShader", &fragment.into());
    })
    .into_js_value();
    let hook = object(&[("key", "hair1".into()), ("compile", compile.clone())]);
    HOOKS.with(|h| h.set(material.unchecked_ref(), &hook));
    set(material, "onBeforeCompile", &compile);
    set(material, "needsUpdate", &true.into());
}

/// Keeps the eye light breathing: the head's clock moves on every frame it is drawn.
fn tick(node: &JsValue, uniforms: &JsValue) {
    let data = get(node, "userData");
    if present(&get(&data, "cdTick")) {
        return;
    }
    let time = get(uniforms, "cdTime");
    let update = Closure::<dyn FnMut()>::new(move || {
        set(&time, "value", &(js_sys::Date::now() / 1000.0 % 3600.0).into());
    })
    .into_js_value();
    set(&data, "cdTick", &update);
    set(node, "onBeforeRender", &update);
}

/// The closed Sfera/Filin shell hides facial hair while equipped. Read the actual
/// mesh state after dressing: the saved beard stays intact and returns on removal.
fn closed_helmet(model: &JsValue) -> bool {
    nodes(model).into_iter().any(|node| {
        if !boolean(&get(&node, "isMesh")) || string(&get(&node, "name")) != "Sfera" {
            return false;
        }
        let mut current = node;
        while present(&current) && !js_sys::Object::is(&current, model) {
            if get(&current, "visible").as_bool() == Some(false) {
                return false;
            }
            current = get(&current, "parent");
        }
        true
    })
}

fn beard_selection(model: &JsValue, paint: &JsValue) -> (String, bool) {
    let prefix = format!("Beard_{}_{}_", string(&get(paint, "face")), string(&get(paint, "beard")));
    let visible = string(&get(paint, "sex")) == "male"
        && string(&get(paint, "beard")) != "none"
        && !boolean(&get(&get(model, "userData"), "cdBeardOccluded"))
        && !boolean(&get(&get(model, "userData"), "cdBeardFirstPerson"))
        && !closed_helmet(model);
    (prefix, visible)
}

/// Beard packs are rigid head children, so the skinned-arms cutter does not see
/// them. Toggle only this player's facial hair, never the shared head bone.
#[wasm_bindgen(js_name=sceneSetBeardFirstPerson)]
pub fn first_person(model: JsValue, on: bool, look: JsValue) {
    set(&get(&model, "userData"), "cdBeardFirstPerson", &on.into());
    let paint = paint(&look);
    let (prefix, visible) = beard_selection(&model, &paint);
    let (hair, hair_on, _) = hair_selection(&model, &paint);
    for node in nodes(&model) {
        let name = string(&get(&node, "name"));
        if boolean(&get(&node, "isMesh")) && name.starts_with("Beard_") {
            set(&node, "visible", &(visible && name.starts_with(&prefix)).into());
        } else if boolean(&get(&node, "isMesh")) && name.starts_with("Hair_") {
            set(&node, "visible", &(hair_on && name.starts_with(&hair)).into());
        }
    }
}

/// A hairstyle from the Persona's hair pack (`Hair_<style>_*` under the head bone): its prefix,
/// whether it shows now, and whether the pack has it. Without it the long hair (female) or the
/// scan's buzz cut (male) stands in.
fn hair_selection(model: &JsValue, paint: &JsValue) -> (String, bool, bool) {
    let style = string(&get(paint, "hair"));
    let prefix = format!("Hair_{style}_");
    let packed = super::beards::hair_styles(&string(&get(paint, "sex"))).contains(&style.as_str())
        && present(&call(model, "getObjectByName", &[format!("{prefix}cards").into()]));
    let visible = packed
        && !boolean(&get(&get(model, "userData"), "cdBeardFirstPerson"))
        && !closed_helmet(model);
    (prefix, visible, packed)
}

/// The arena swaps untextured parts for a shared body colour; a hair cap or tie keeps one of its own.
fn own_material(model: &JsValue, node: &JsValue, name: &str) -> JsValue {
    let material = get(node, "material");
    if string(&get(&material, "name")) == name {
        return material;
    }
    let key = format!("cd{name}");
    let mut own = get(&get(model, "userData"), &key);
    if !present(&own) {
        own = call(&material, "clone", &[]);
        set(&own, "name", &name.into());
        set(&own, "roughness", &0.78.into());
        set(&own, "metalness", &0.0.into());
        set(&get(model, "userData"), &key, &own);
    }
    set(node, "material", &own);
    own
}

/// Paints `look` onto `model`. Safe to call again after every change.
#[wasm_bindgen(js_name=sceneApplyLook)]
pub fn apply(model: JsValue, look: JsValue) {
    let paint = paint(&look);
    // dress() repaints the worn pieces with this look when the kit's colours change
    set(&get(&model, "userData"), "cdLook", &look);
    let gear = super::dom_support::rust(
        "client.look.gearTints",
        &[look.clone(), get(&get(&model, "userData"), "cdColors")],
    );
    let skin = rgb(&get(&paint, "skin"));
    let face = format!("{HEAD}{}", string(&get(&paint, "face")));
    let (beard, beard_on) = beard_selection(&model, &paint);
    let (hair, hair_on, packed) = hair_selection(&model, &paint);
    // a pack style the model lacks shows the long hair rather than none
    let stand_in = super::beards::HAIR_STYLES.contains(&string(&get(&paint, "hair")).as_str()) && !packed;
    let scalp = scalp(&model, packed);
    let mut cap_material = get(&get(&model, "userData"), "cdCap");
    for node in nodes(&model) {
        if !boolean(&get(&node, "isMesh")) {
            continue;
        }
        let material = get(&node, "material");
        let name = string(&get(&material, "name"));
        let mesh = string(&get(&node, "name"));
        if mesh.starts_with("Beard_") {
            // Keep only the style fitted to this head. Materials can be shared across
            // variants, so visibility belongs to the mesh, not to its material.
            set(&node, "visible", &(beard_on && mesh.starts_with(&beard)).into());
            tint(&material, rgb(&get(&paint, "beardColor")));
            if name.starts_with("M_Beard_Cards") {
                hair_hook(&material);
            }
            continue;
        }
        if mesh == "Hair_scalp" {
            // carries the scalp mask only (`scalp`) and stays hidden
            continue;
        }
        if mesh.starts_with("Hair_") {
            set(&node, "visible", &(hair_on && mesh.starts_with(&hair)).into());
            // the pack's shape keys fit it to each face's own skull (game_cards.py), in face order
            let fits = get(&node, "morphTargetInfluences");
            if present(&fits) {
                let chosen = face_index(&string(&get(&paint, "face")));
                for i in 0..values(&fits).len() {
                    set(&fits, &i.to_string(), &(if chosen == Some(i) { 1.0 } else { 0.0 }).into());
                }
            }
            if mesh.ends_with("_cap") {
                let m = own_material(&model, &node, "M_Hair_Cap");
                let c = rgb(&get(&paint, "hairCap"));
                call(&get(&m, "color"), "setRGB", &[c[0].into(), c[1].into(), c[2].into()]);
            } else if mesh.ends_with("_fade") {
                // the crop's clipper fade and the slick's hairline lie on the skull and tile round it: no strip hook, whose
                // clamped texture would smear the first column into stripes
                tint(&material, rgb(&get(&paint, "hairCards")));
            } else if mesh.ends_with("_tie") {
                let m = own_material(&model, &node, "M_Hair_Tie");
                call(&get(&m, "color"), "setRGB", &[0.012.into(), 0.012.into(), 0.013.into()]);
            } else {
                tint(&material, rgb(&get(&paint, "hairCards")));
                hair_hook(&material);
            }
            continue;
        }
        if mesh.starts_with("FemHairCap") {
            // The arena swaps untextured parts for a shared body colour; hair needs its own.
            if name != CAP {
                if !present(&cap_material) {
                    cap_material = call(&material, "clone", &[]);
                    set(&cap_material, "name", &CAP.into());
                    set(&cap_material, "roughness", &0.78.into());
                    set(&cap_material, "metalness", &0.0.into());
                    set(&get(&model, "userData"), "cdCap", &cap_material);
                }
                set(&node, "material", &cap_material);
            }
            let m = get(&node, "material");
            let c = rgb(&get(&paint, "hairCap"));
            call(&get(&m, "color"), "setRGB", &[c[0].into(), c[1].into(), c[2].into()]);
            set(&m, "visible", &(boolean(&get(&paint, "cap")) || stand_in).into());
            continue;
        }
        if name.starts_with(HEAD) {
            set(&node, "visible", &(name == face).into());
            tint(&material, skin);
            let uniforms = head_hook(&material, &node, &paint, &scalp);
            tick(&node, &uniforms);
        } else if BODIES.contains(&name.as_str()) {
            tint(&material, skin);
        } else if present(&get(&gear, &name)) {
            // suits, the cloth that follows them and every worn piece given a colour in the kit
            tint(&material, rgb(&get(&gear, &name)));
        } else if name == "M_Fem_HairCards" {
            tint(&material, rgb(&get(&paint, "hairCards")));
            hair_hook(&material);
            set(&material, "visible", &(boolean(&get(&paint, "cards")) || stand_in).into());
        }
    }
}
