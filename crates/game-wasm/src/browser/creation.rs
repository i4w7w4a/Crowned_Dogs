//! Persona creation panel: the contract card and the look choices under the fitting stand.
//! The base owns the events; this module only draws the choices and edits the look.
use super::dom_support::*;
use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};

/// Typical skin under the light-skin scan; swatches show it times each skin factor.
const SKIN_SWATCH: [u8; 3] = [0xd8, 0xb4, 0x9a];

fn list(options: &JsValue, key: &str) -> Vec<JsValue> {
    Array::from(&get(options, key)).iter().collect()
}
fn rgb(hex: &str) -> [u8; 3] {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0xffffff);
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}
fn swatch_color(key: &str, hex: &str) -> String {
    let c = rgb(hex);
    let c = if key == "skin" {
        [0, 1, 2].map(|i| (SKIN_SWATCH[i] as u32 * c[i] as u32 / 255) as u8)
    } else {
        c
    };
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}
fn segment(label: &str, key: &str, items: &[JsValue], current: &str) -> String {
    let mut out = format!(
        "<div class=\"cd-create-row\" role=\"group\" aria-label=\"{label}\"><span class=\"cd-overline\">{label}</span><div class=\"cd-create-seg\">"
    );
    for item in items {
        let id = text(&get(item, "id"));
        out.push_str(&format!(
            "<button class=\"glass\" type=\"button\" data-look=\"{key}\" data-value=\"{id}\" aria-pressed=\"{}\">{}</button>",
            id == current,
            text(&get(item, "name"))
        ));
    }
    out.push_str("</div></div>");
    out
}
fn swatches(label: &str, key: &str, items: &[JsValue], current: &str) -> String {
    let mut out = format!(
        "<div class=\"cd-create-row\" role=\"group\" aria-label=\"{label}\"><span class=\"cd-overline\">{label}</span><div class=\"cd-create-swatches\">"
    );
    for item in items {
        let id = text(&get(item, "id"));
        let name = text(&get(item, "name"));
        out.push_str(&format!(
            "<button class=\"cd-swatch\" type=\"button\" data-look=\"{key}\" data-value=\"{id}\" aria-pressed=\"{}\" aria-label=\"{name}\" title=\"{name}\" style=\"--swatch:{}\"></button>",
            id == current,
            swatch_color(key, &text(&get(item, "color")))
        ));
    }
    out.push_str("</div></div>");
    out
}

/// The choice rows for `look`; hair and faces follow the chosen sex.
pub fn markup(look: &JsValue) -> String {
    let options = rust("client.look.options", &[]);
    let sex = text(&get(look, "sex"));
    let own = get(&options, &sex);
    let pick = |key: &str| text(&get(look, key));
    let mut out = segment("ПОЛ", "sex", &list(&options, "sexes"), &sex);
    out.push_str(&segment("ЛИЦО", "face", &list(&own, "faces"), &pick("face")));
    out.push_str(&swatches("КОЖА", "skin", &list(&options, "skins"), &pick("skin")));
    out.push_str(&swatches("ГЛАЗА", "eyes", &list(&options, "eyes"), &pick("eyes")));
    out.push_str(&segment("ПРИЧЁСКА", "hair", &list(&own, "hairs"), &pick("hair")));
    if boolean(&own, "hairColor") {
        out.push_str(&swatches(
            "ЦВЕТ ВОЛОС",
            "hairColor",
            &list(&options, "hairColors"),
            &pick("hairColor"),
        ));
    }
    out.push_str(&swatches("КОСТЮМ", "suit", &list(&options, "suits"), &pick("suit")));
    out
}

/// `look` with one field changed, repaired so every field still fits the sex.
pub fn pick(look: &JsValue, key: &str, value: &str) -> JsValue {
    let next = js_sys::Object::assign(&js_sys::Object::new(), look.unchecked_ref());
    set(&next, key, &js(value));
    if key == "sex" {
        // Face and hair lists differ per sex; let normalize choose that sex's defaults.
        for field in ["face", "hair"] {
            set(&next, field, &js(""));
        }
    }
    rust("client.look.normalize", &[next.into()])
}

/// A fresh tag of the form K-XXX-XXX, issued when the contract is signed.
pub fn issue_tag() -> JsValue {
    let n = (js_sys::Math::random() * 1_000_000.0).floor().min(999_999.0);
    rust("core.formatTag", &[number(n)])
}
