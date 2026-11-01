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

/// Sets up the eye shader on a head material; returns its uniforms. The head's model carries its two irises in the
/// mesh's own space (`cdEyes`: x, y, z, radius twice): the scans' UV atlases scatter an eye over several islands, so
/// the eyes are found by position, not by texture area.
fn head_hook(material: &JsValue, node: &JsValue, paint: &JsValue) -> JsValue {
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
        ]);
        set(&get(material, "userData"), "cdLook", &uniforms);
        let shared = uniforms.clone();
        let compile = Closure::<dyn FnMut(JsValue)>::new(move |shader: JsValue| {
            let own = get(&shader, "uniforms");
            for key in ["cdEyeA", "cdEyeB", "cdIris", "cdTime"] {
                set(&own, key, &get(&shared, key));
            }
            let vertex = string(&get(&shader, "vertexShader"))
                .replacen("#include <common>", "#include <common>\nvarying vec3 cdLocal;", 1)
                .replacen("#include <begin_vertex>", "#include <begin_vertex>\ncdLocal = position;", 1);
            set(&shader, "vertexShader", &vertex.into());
            let fragment = string(&get(&shader, "fragmentShader"))
                .replacen(
                    "#include <common>",
                    "#include <common>\nvarying vec3 cdLocal;\nuniform vec4 cdEyeA;\nuniform vec4 cdEyeB;\nuniform vec3 cdIris;\nuniform float cdTime;\nvec3 cdGlow = vec3( 0.0 );",
                    1,
                )
                .replacen(
                    "#include <map_fragment>",
                    concat!("#include <map_fragment>\n", include_str!("shaders/look-eyes.glsl")),
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
        let hook = object(&[("key", "look4".into()), ("compile", compile.clone())]);
        HOOKS.with(|h| h.set(material.unchecked_ref(), &hook));
        // Until the next tune, the hook runs on its own; tune then chains it with the rest.
        set(material, "onBeforeCompile", &compile);
        set(material, "needsUpdate", &true.into());
    }
    set(&get(&uniforms, "cdEyeA"), "value", &area(0));
    set(&get(&uniforms, "cdEyeB"), "value", &area(1));
    set(&get(&uniforms, "cdIris"), "value", &get(paint, "iris"));
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

/// Paints `look` onto `model`. Safe to call again after every change.
#[wasm_bindgen(js_name=sceneApplyLook)]
pub fn apply(model: JsValue, look: JsValue) {
    let paint = paint(&look);
    let skin = rgb(&get(&paint, "skin"));
    let face = format!("{HEAD}{}", string(&get(&paint, "face")));
    let mut cap_material = get(&get(&model, "userData"), "cdCap");
    for node in nodes(&model) {
        if !boolean(&get(&node, "isMesh")) {
            continue;
        }
        let material = get(&node, "material");
        let name = string(&get(&material, "name"));
        let mesh = string(&get(&node, "name"));
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
            set(&m, "visible", &get(&paint, "cap"));
            continue;
        }
        if name.starts_with(HEAD) {
            set(&node, "visible", &(name == face).into());
            tint(&material, skin);
            let uniforms = head_hook(&material, &node, &paint);
            tick(&node, &uniforms);
        } else if BODIES.contains(&name.as_str()) {
            tint(&material, skin);
        } else if name == "M_Suit" {
            tint(&material, rgb(&get(&paint, "suit")));
        } else if name == "M_Fem_HairCards" {
            tint(&material, rgb(&get(&paint, "hairCards")));
            hair_hook(&material);
            set(&material, "visible", &get(&paint, "cards"));
        }
    }
}
