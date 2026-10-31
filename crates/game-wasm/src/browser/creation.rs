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

/// Creation steps: (id, tab, instructor's line, camera on the face).
pub const STEPS: [(&str, &str, &str, bool); 7] = [
    ("sex", "Пол", "Мужская Персона или женская? Броня сядет на любую.", false),
    ("face", "Лицо", "Лицо. Пока на складе одно, новые уже в работе.", true),
    ("skin", "Кожа", "Тон кожи. Под нашими лампами все бледные, не пугайся.", true),
    ("eyes", "Глаза", "Цвет глаз. Мелочь, но по ним тебя узнают свои.", true),
    ("hair", "Волосы", "Причёска. Под каской не видно, но по базе ходишь без неё.", true),
    ("suit", "Костюм", "Цвет костюма. Выбирай, в чём не стыдно умирать.", false),
    ("callsign", "Позывной", "Последнее: позывной. Так тебя будут звать в эфире.", false),
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
    let mut picked = String::new();
    for item in items {
        let id = text(&get(item, "id"));
        let name = text(&get(item, "name"));
        if id == current {
            picked = name.clone();
        }
        out.push_str(&format!(
            "<button class=\"cd-swatch\" type=\"button\" data-look=\"{key}\" data-value=\"{id}\" aria-pressed=\"{}\" aria-label=\"{name}\" title=\"{name}\" style=\"--swatch:{}\"></button>",
            id == current,
            swatch_color(key, &text(&get(item, "color")))
        ));
    }
    out.push_str(&format!("</div><span class=\"cd-create-picked\">{picked}</span></div>"));
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

/// Step tabs; the current one is pressed.
pub fn tabs(step: usize) -> String {
    STEPS
        .iter()
        .enumerate()
        .map(|(i, (_, name, _, _))| {
            format!(
                "<button class=\"glass\" type=\"button\" role=\"tab\" data-step=\"{i}\" aria-selected=\"{}\">{name}</button>",
                i == step
            )
        })
        .collect()
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
