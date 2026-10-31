//! Paints the player's chosen look onto a loaded Persona: skin, suit, hair and eyes.
//! The fitting room, the creation screen and the arena all call `apply` on their own model.
use super::bridge::*;
use super::scene_assets::nodes;
use js_sys::WeakMap;
use wasm_bindgen::prelude::*;

thread_local! {
    /// Head materials that carry the eye shader; `scene_assets::tune` chains it in.
    static HOOKS: WeakMap = WeakMap::new();
}
const HEADS: [&str; 2] = ["M_Male_Head", "M_Fem_Head"];
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

fn head_hook(material: &JsValue, paint: &JsValue) {
    let spots = values(&get(paint, "eyeSpots"));
    let area = |i: usize| array(&spots[i * 4..i * 4 + 4]).into();
    let mut uniforms = get(&get(material, "userData"), "cdLook");
    if !present(&uniforms) {
        uniforms = object(&[
            ("cdEyeA", object(&[("value", area(0))])),
            ("cdEyeB", object(&[("value", area(1))])),
            ("cdIris", object(&[("value", get(paint, "iris"))])),
        ]);
        set(&get(material, "userData"), "cdLook", &uniforms);
        let shared = uniforms.clone();
        let compile = Closure::<dyn FnMut(JsValue)>::new(move |shader: JsValue| {
            let own = get(&shader, "uniforms");
            for key in ["cdEyeA", "cdEyeB", "cdIris"] {
                set(&own, key, &get(&shared, key));
            }
            let vertex = string(&get(&shader, "vertexShader"))
                .replacen("#include <common>", "#include <common>\nvarying vec3 cdLocal;", 1)
                .replacen("#include <begin_vertex>", "#include <begin_vertex>\ncdLocal = position;", 1);
            set(&shader, "vertexShader", &vertex.into());
            let fragment = string(&get(&shader, "fragmentShader"))
                .replacen(
                    "#include <common>",
                    "#include <common>\nvarying vec3 cdLocal;\nuniform vec4 cdEyeA;\nuniform vec4 cdEyeB;\nuniform vec3 cdIris;\nvec3 cdGlow = vec3( 0.0 );",
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
        let hook = object(&[("key", "look3".into()), ("compile", compile.clone())]);
        HOOKS.with(|h| h.set(material.unchecked_ref(), &hook));
        // Until the next tune, the hook runs on its own; tune then chains it with the rest.
        set(material, "onBeforeCompile", &compile);
        set(material, "needsUpdate", &true.into());
    }
    set(&get(&uniforms, "cdEyeA"), "value", &area(0));
    set(&get(&uniforms, "cdEyeB"), "value", &area(1));
    set(&get(&uniforms, "cdIris"), "value", &get(paint, "iris"));
}

/// Paints `look` onto `model`. Safe to call again after every change.
#[wasm_bindgen(js_name=sceneApplyLook)]
pub fn apply(model: JsValue, look: JsValue) {
    let paint = paint(&look);
    let skin = rgb(&get(&paint, "skin"));
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
        if HEADS.contains(&name.as_str()) {
            tint(&material, skin);
            head_hook(&material, &paint);
        } else if BODIES.contains(&name.as_str()) {
            tint(&material, skin);
        } else if name == "M_Suit" {
            tint(&material, rgb(&get(&paint, "suit")));
        } else if name == "M_Fem_HairCards" {
            tint(&material, rgb(&get(&paint, "hairCards")));
            set(&material, "visible", &get(&paint, "cards"));
        }
    }
}
