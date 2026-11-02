//! Persona creation: the instructor's briefing and contract, then the look picked step by step.
//! The base owns the events; this module only draws the screens and edits the look.
use super::dom_support::*;
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
    elements(&get(options, key))
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
    segment_ready(label, key, items, current, true)
}
fn segment_ready(label: &str, key: &str, items: &[JsValue], current: &str, ready: bool) -> String {
    let mut out = format!(
        "<div class=\"cd-create-row\" role=\"group\" aria-label=\"{label}\"><div class=\"cd-create-seg\">"
    );
    for item in items {
        let id = text(&get(item, "id"));
        let disabled = if !ready && id != "none" { " disabled" } else { "" };
        out.push_str(&format!(
            "<button class=\"glass\" type=\"button\" data-look=\"{key}\" data-value=\"{id}\" aria-pressed=\"{}\"{disabled}>{}<span>{}</span></button>",
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
/// Beard choices stay unavailable until the current model has its actual fitted pack.
pub fn step_markup_with_beards(look: &JsValue, step: usize, beards_ready: bool) -> String {
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
            let hairs = list(&own, "hairs");
            let mut out = if sex == "male" && hairs.len() <= 1 {
                String::new()
            } else {
                segment("Причёска", "hair", &hairs, &pick("hair"))
            };
            // the male scan's buzz cut is baked into the head and keeps its own colour
            if boolean(&own, "hairColor") && pick("hair") != "buzz" {
                out.push_str(&swatches(
                    "Цвет волос",
                    "hairColor",
                    &list(&options, "hairColors"),
                    &pick("hairColor"),
                ));
            }
            if sex == "male" {
                out.push_str("<span class=\"cd-overline\">Борода</span>");
                out.push_str(&segment_ready("Борода", "beard", &list(&own, "beards"), &pick("beard"), beards_ready));
                if !beards_ready {
                    out.push_str("<span class=\"cd-create-picked\" role=\"status\">Бороды пока недоступны</span>");
                } else if pick("beard") != "none" {
                    out.push_str(&swatches("Цвет бороды", "beardColor", &list(&options, "beardColors"), &pick("beardColor")));
                }
            }
            out
        }
        "suit" => swatches("Костюм", "suit", &list(&options, "suits"), &pick("suit")),
        _ => String::new(),
    }
}

/// Icons for the steps, the camera, the buttons and the choices, on a 24 grid like the combat
/// icons: things are solid silhouettes with cut-outs, actions and signs stay thin lines.
fn icon_paths(id: &str) -> &'static str {
    match id {
        "none" => "<path d=\"M5 6.5c0 8 2.2 13 7 13s7-5 7-13M8 12h2m4 0h2M9 16c2 1 4 1 6 0\"/>",
        "boxed" => "<path fill=\"currentColor\" stroke=\"none\" d=\"M5 7l3 4v4l4 3 4-3v-4l3-4v9l-7 5-7-5zM8 12l4-2 4 2-1 2-3-1-3 1z\"/>",
        "goatee" => "<path fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\" d=\"M8 11l4-2 4 2 1 6-5 4-5-4zM9 14v2l3 2 3-2v-2l-3 1z\"/>",
        "full" => "<path fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\" d=\"M4 6l4 5 4-2 4 2 4-5v9c0 4-4 7-8 7s-8-3-8-7zM9 14v2h6v-2z\"/>",
        "tapered" => "<path fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\" d=\"M4 6l4 5 4-2 4 2 4-5-1 9-7 8-7-8zM9 14v2h6v-2z\"/>",
        "stubble" => "<g fill=\"currentColor\" stroke=\"none\"><circle cx=\"5\" cy=\"9\" r=\"1.1\"/><circle cx=\"19\" cy=\"9\" r=\"1.1\"/><circle cx=\"6\" cy=\"13\" r=\"1.1\"/><circle cx=\"9.5\" cy=\"12\" r=\"1.1\"/><circle cx=\"14.5\" cy=\"12\" r=\"1.1\"/><circle cx=\"18\" cy=\"13\" r=\"1.1\"/><circle cx=\"8\" cy=\"16.5\" r=\"1.1\"/><circle cx=\"12\" cy=\"16\" r=\"1.1\"/><circle cx=\"16\" cy=\"16.5\" r=\"1.1\"/><circle cx=\"10\" cy=\"19.5\" r=\"1.1\"/><circle cx=\"14\" cy=\"19.5\" r=\"1.1\"/></g>",
        "body" => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M9.2 4.4a2.8 2.8 0 1 0 5.6 0a2.8 2.8 0 1 0-5.6 0z\"/><path d=\"M7.5 8.1h9c1.4 0 2.4 1.2 2.1 2.6l-1.3 5.2h-1.8v6.1h-2.6v-6h-1.8v6H8.5v-6.1H6.7l-1.3-5.2c-.3-1.4.7-2.6 2.1-2.6z\"/></g>",
        "next" => "<path d=\"M14 5.5l6.5 6.5-6.5 6.5M20 12H3.5\"/>",
        "save" => "<path d=\"M4.5 12.5l5 5 10-11\"/>",
        "male" => "<circle cx=\"10\" cy=\"14\" r=\"5.5\"/><path d=\"M14 10l6-6M15 4h5v5\"/>",
        "female" => "<circle cx=\"12\" cy=\"9\" r=\"5.5\"/><path d=\"M12 14.5V21M9 18h6\"/>",
        "long" => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M5.6 21c-1.4-4.4-1.5-9.6.4-12.8C7.4 5.6 9.5 4 12 4s4.6 1.6 6 4.2c1.9 3.2 1.8 8.4.4 12.8zM12 8.6c-1.8 0-3 1.5-3 3.8 0 2.6 1.4 5.1 3 5.1s3-2.5 3-5.1c0-2.3-1.2-3.8-3-3.8z\"/></g>",
        "shaved" | "buzz" => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M7 12.5c0-4.3 2.2-7.2 5-7.2s5 2.9 5 7.2-2.2 7.2-5 7.2-5-2.9-5-7.2zM8.3 9.4a.8 .8 0 1 0 1.6 0a.8 .8 0 1 0-1.6 0zM9.8 7.9a.8 .8 0 1 0 1.6 0a.8 .8 0 1 0-1.6 0zM11.2 7.4a.8 .8 0 1 0 1.6 0a.8 .8 0 1 0-1.6 0zM12.6 7.9a.8 .8 0 1 0 1.6 0a.8 .8 0 1 0-1.6 0zM14.1 9.4a.8 .8 0 1 0 1.6 0a.8 .8 0 1 0-1.6 0z\"/></g>",
        "sex" => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M5.8 7.3a3.2 3.2 0 1 0 6.4 0a3.2 3.2 0 1 0-6.4 0z\"/><path d=\"M3.2 20.5v-1.3a5.8 5.8 0 0 1 11.6 0v1.3z\"/><path d=\"M14.7 8.8a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0-5 0z\"/><path d=\"M15.9 20.5v-1.3c0-1.7-.5-3.2-1.4-4.4a4.7 4.7 0 0 1 7 4.1v1.6z\"/></g>",
        "face" => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M12 3.2c-3.8 0-6.3 2.9-6.3 7 0 4.9 2.9 10.6 6.3 10.6s6.3-5.7 6.3-10.6c0-4.1-2.5-7-6.3-7zM8.45 10.9a1.05 1.05 0 1 0 2.1 0a1.05 1.05 0 1 0-2.1 0zM13.45 10.9a1.05 1.05 0 1 0 2.1 0a1.05 1.05 0 1 0-2.1 0zM10 15.1c1.3.8 2.7.8 4 0l.6 1c-1.7 1.1-3.5 1.1-5.2 0z\"/></g>",
        "skin" => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M12 3.3c3.1 3.9 6.2 7.4 6.2 10.9a6.2 6.2 0 0 1-12.4 0c0-3.5 3.1-7 6.2-10.9zM8.8 14.4a3.2 3.2 0 0 0 3.2 3.2v-1.5a1.7 1.7 0 0 1-1.7-1.7z\"/></g>",
        "eyes" => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M2.3 12S6 5.6 12 5.6 21.7 12 21.7 12 18 18.4 12 18.4 2.3 12 2.3 12zM8.4 12a3.6 3.6 0 1 0 7.2 0a3.6 3.6 0 1 0-7.2 0zM10.1 12a1.9 1.9 0 1 0 3.8 0a1.9 1.9 0 1 0-3.8 0z\"/></g>",
        "hair" => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M4.9 20.8c-.7-7.4 1.5-14 7.1-16.4 5.6 2.4 7.8 9 7.1 16.4h-3c.4-4.4-.6-8.2-4.1-10.9-3.5 2.7-4.5 6.5-4.1 10.9z\"/></g>",
        "suit" => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M8.3 3.6L12 6.4l3.7-2.8 5 2.8-1.9 5-2-.8V21H7.2V10.6l-2 .8-1.9-5zM11.4 7.9h1.2V21h-1.2z\"/></g>",
        _ => "<g fill=\"currentColor\" stroke=\"none\" fill-rule=\"evenodd\"><path d=\"M8.6 3.5h6.8l2.4 2.4v12.2l-2.4 2.4H8.6l-2.4-2.4V5.9zM9.2 11.3h5.6v1.4H9.2zM9.2 14.7h3.8v1.4H9.2zM11.3 6.1h1.4v1.4h-1.4z\"/></g>",
    }
}

/// Step tabs: an icon over a short name; the current one is pressed.
pub fn tabs(step: usize, male: bool) -> String {
    STEPS
        .iter()
        .enumerate()
        .map(|(i, (id, name, _))| {
            let label = if male && *id == "hair" { "Волосы и борода" } else { *name };
            format!(
                "<button class=\"glass\" type=\"button\" role=\"tab\" data-step=\"{i}\" aria-selected=\"{}\" aria-label=\"{label}\">{}<span>{name}</span></button>",
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
    // The beard takes the hair's colour (the two lists share their ids); its own swatches still change it after.
    if key == "hairColor" {
        set(&next, "beardColor", &js(value));
    }
    if key == "sex" {
        // Face and hair lists differ per sex; let normalize choose that sex's defaults.
        for field in ["face", "hair"] {
            set(&next, field, &js(""));
        }
    }
    rust("client.look.normalize", &[next.into()])
}

/// A fresh tag K-<call-up>-<order>, issued when the contract is signed (`game-core/src/tag.rs`).
pub fn issue_tag() -> JsValue {
    rust("core.localTag", &[number(js_sys::Date::now())])
}
