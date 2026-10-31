//! Persona creation: the instructor's briefing and contract, then the look picked step by step.
//! The base owns the events; this module only draws the screens and edits the look.
use super::dom_support::*;
use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};

/// Typical skin under the light-skin scan; swatches show it times each skin factor.
const SKIN_SWATCH: [u8; 3] = [0xd8, 0xb4, 0x9a];
/// Callsign offered on the first Persona.
pub const DEFAULT_CALLSIGN: &str = "Сокол";

/// The instructor's lines before the first Persona: (line, button). `{tag}` is the issued tag.
pub const BRIEFING: [(&str, &str); 6] = [
    (
        "Оператор, слышишь меня? Я Седой, твой инструктор. Говорю коротко, пока канал держится.",
        "Дальше",
    ),
    (
        "Воевать будешь не сам. Ляжешь в капсулу, наденешь корону и поведёшь Персону, боевого робота.",
        "Дальше",
    ),
    (
        "Персона чувствует всё, значит, и ты тоже. Её убьют, ты очнёшься в капсуле. Но больно будет по-настоящему.",
        "Дальше",
    ),
    (
        "Теперь контракт с ЧВК. Служба по контракту, платим водой. Отказ не принимается: умирать-то будешь не ты.",
        "Подписать",
    ),
    (
        "Подписано. Твой жетон {tag}. По нему тебя найдут и по нему заплатят.",
        "Дальше",
    ),
    (
        "Теперь соберём тебе Персону. Её делают похожей на оператора: так мозг быстрее принимает чужое тело.",
        "Собрать Персону",
    ),
];
/// The briefing line whose button signs the contract.
pub const SIGN_LINE: usize = 3;

/// Creation steps: (id, tab, camera on the face).
pub const STEPS: [(&str, &str, bool); 7] = [
    ("sex", "Пол", false),
    ("face", "Лицо", true),
    ("skin", "Кожа", true),
    ("eyes", "Глаза", true),
    ("hair", "Волосы", true),
    ("suit", "Костюм", false),
    ("callsign", "Позывной", false),
];

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
        "<div class=\"cd-create-row\" role=\"group\" aria-label=\"{label}\"><div class=\"cd-create-seg\">"
    );
    for item in items {
        let id = text(&get(item, "id"));
        out.push_str(&format!(
            "<button class=\"glass\" type=\"button\" data-look=\"{key}\" data-value=\"{id}\" aria-pressed=\"{}\">{}<span>{}</span></button>",
            id == current,
            icon(if key == "face" { "face" } else { &id }),
            text(&get(item, "name"))
        ));
    }
    out.push_str("</div></div>");
    out
}
fn swatches(label: &str, key: &str, items: &[JsValue], current: &str) -> String {
    let mut out = format!(
        "<div class=\"cd-create-row\" role=\"group\" aria-label=\"{label}\"><span class=\"cd-overline\">{label}</span>"
    );
    let picked = items
        .iter()
        .find(|item| text(&get(item, "id")) == current)
        .map(|item| text(&get(item, "name")))
        .unwrap_or_default();
    out.push_str(&format!(
        "<span class=\"cd-create-picked\">{picked}</span><div class=\"cd-create-swatches\">"
    ));
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

/// The choices of one creation step; hair and faces follow the chosen sex.
pub fn step_markup(look: &JsValue, step: usize) -> String {
    let options = rust("client.look.options", &[]);
    let sex = text(&get(look, "sex"));
    let own = get(&options, &sex);
    let pick = |key: &str| text(&get(look, key));
    match STEPS[step.min(STEPS.len() - 1)].0 {
        "sex" => segment("Пол", "sex", &list(&options, "sexes"), &sex),
        "face" => segment("Лицо", "face", &list(&own, "faces"), &pick("face")),
        "skin" => swatches("Кожа", "skin", &list(&options, "skins"), &pick("skin")),
        "eyes" => swatches("Глаза", "eyes", &list(&options, "eyes"), &pick("eyes")),
        "hair" => {
            let mut out = segment("Причёска", "hair", &list(&own, "hairs"), &pick("hair"));
            if boolean(&own, "hairColor") {
                out.push_str(&swatches(
                    "Цвет волос",
                    "hairColor",
                    &list(&options, "hairColors"),
                    &pick("hairColor"),
                ));
            }
            out
        }
        "suit" => swatches("Костюм", "suit", &list(&options, "suits"), &pick("suit")),
        _ => String::new(),
    }
}

/// Line icons for the steps, the camera, the buttons and the choices, drawn on a 24 grid like
/// the combat icons.
fn icon_paths(id: &str) -> &'static str {
    match id {
        "body" => "<circle cx=\"12\" cy=\"4.8\" r=\"2.3\"/><path d=\"M7.6 21v-6.4L6 9.6c0-1.1.8-2 1.9-2h8.2c1.1 0 1.9.9 1.9 2l-1.6 5V21M12 13.6V21\"/>",
        "next" => "<path d=\"M14 5l7 7-7 7M21 12H3\"/>",
        "save" => "<path d=\"M4.5 12.5l5 5 10-11\"/>",
        "male" => "<circle cx=\"10\" cy=\"14\" r=\"5.5\"/><path d=\"M14 10l6-6M15 4h5v5\"/>",
        "female" => "<circle cx=\"12\" cy=\"9\" r=\"5.5\"/><path d=\"M12 14.5V21M9 18h6\"/>",
        "long" => "<path d=\"M6 21c-1.2-4-1.4-9 .3-12.3C7.6 6 9.6 4.5 12 4.5s4.4 1.5 5.7 4.2C19.4 12 19.2 17 18 21\"/><path d=\"M9 10.6c.3 3.2 1.3 5.6 3 6.6 1.7-1 2.7-3.4 3-6.6\"/>",
        "shaved" | "buzz" => "<path d=\"M7 12.5c0-4.3 2.2-7 5-7s5 2.7 5 7-2.2 7-5 7-5-2.7-5-7z\"/><path d=\"M8.6 8.3h.01M10.3 6.9h.01M12 6.5h.01M13.7 6.9h.01M15.4 8.3h.01\"/>",
        "sex" => "<circle cx=\"9\" cy=\"7.5\" r=\"3\"/><path d=\"M3.5 20v-1a5.5 5.5 0 0 1 11 0v1\"/><circle cx=\"17\" cy=\"9\" r=\"2.4\"/><path d=\"M15.6 14a4.6 4.6 0 0 1 5.9 4.4V20\"/>",
        "face" => "<path d=\"M12 3.2c-3.8 0-6.3 2.9-6.3 7 0 4.9 2.9 10.6 6.3 10.6s6.3-5.7 6.3-10.6c0-4.1-2.5-7-6.3-7z\"/><path d=\"M9.4 10.8h.01M14.6 10.8h.01M10.1 15.6c1.2.7 2.6.7 3.8 0\"/>",
        "skin" => "<path d=\"M12 3.5c3.1 3.9 6 7.3 6 10.7a6 6 0 0 1-12 0c0-3.4 2.9-6.8 6-10.7z\"/><path d=\"M9.2 14.6a2.9 2.9 0 0 0 2.8 2.8\"/>",
        "eyes" => "<path d=\"M2.5 12S6 5.8 12 5.8 21.5 12 21.5 12 18 18.2 12 18.2 2.5 12 2.5 12z\"/><circle cx=\"12\" cy=\"12\" r=\"3.1\"/><path d=\"M12 12h.01\"/>",
        "hair" => "<path d=\"M5.5 20.5c-.6-7.2 1.4-13.6 6.5-15.6 5.1 2 7.1 8.4 6.5 15.6\"/><path d=\"M12 4.9c-1.6 3.2-2.4 6.6-2.2 10.4M12 4.9c2.6 2.4 3.6 5.6 3.4 9.4\"/>",
        "suit" => "<path d=\"M8.3 3.8L12 6.6l3.7-2.8 4.8 2.7-1.8 4.8-2.1-.9V21H7.4V10.4l-2.1.9-1.8-4.8z\"/><path d=\"M12 6.6V21\"/>",
        _ => "<rect x=\"6.2\" y=\"3.5\" width=\"11.6\" height=\"17\" rx=\"4\"/><path d=\"M12 6.8h.01M9.2 12h5.6M9.2 15.4h3.8\"/>",
    }
}

/// Step tabs: an icon over a short name; the current one is pressed.
pub fn tabs(step: usize) -> String {
    STEPS
        .iter()
        .enumerate()
        .map(|(i, (id, name, _))| {
            format!(
                "<button class=\"glass\" type=\"button\" role=\"tab\" data-step=\"{i}\" aria-selected=\"{}\" aria-label=\"{name}\">{}<span>{name}</span></button>",
                i == step,
                icon(id)
            )
        })
        .collect()
}
/// One line icon as inline SVG.
pub fn icon(id: &str) -> String {
    format!(
        "<svg class=\"cd-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\">{}</svg>",
        icon_paths(id)
    )
}

/// A line as words that appear one after another, as if spoken.
pub fn spoken(line: &str) -> String {
    line.split(' ')
        .enumerate()
        .map(|(i, word)| format!("<span style=\"--i:{i}\">{word}</span>"))
        .collect::<Vec<_>>()
        .join(" ")
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
