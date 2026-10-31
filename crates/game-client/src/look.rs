//! Persona appearance: what the player picks at creation and how each pick tints the model.
use crate::text;
use serde_json::{json, Value};

/// (id, name, sRGB colour). Colours are targets or multipliers, see `paint`.
type Choice = (&'static str, &'static str, u32);

const SEXES: [(&str, &str); 2] = [("male", "Мужская"), ("female", "Женская")];
/// One head per Persona for now; new heads from Tripo join these lists.
const FACES: [(&str, &str, &str); 2] = [("male", "m1", "Лицо 1"), ("female", "f1", "Лицо 1")];
/// Multipliers over the baked skin texture; white keeps the scan as it is.
const SKINS: [Choice; 6] = [
    ("light", "Светлая", 0xffffff),
    ("warm", "Тёплая", 0xecd2bc),
    ("tan", "Смуглая", 0xd2aa88),
    ("olive", "Оливковая", 0xbea27e),
    ("brown", "Тёмная", 0x9c7458),
    ("deep", "Очень тёмная", 0x6e4c38),
];
/// Eye glow colours. The ids are the old iris list's, so saved looks still load.
const EYES: [Choice; 6] = [
    ("grey", "Лёд", 0x8fd6ff),
    ("blue", "Синие", 0x3f8cff),
    ("green", "Зелёные", 0x4dff88),
    ("hazel", "Янтарные", 0xffb43a),
    ("brown", "Красные", 0xff3d2e),
    ("dark", "Фиолетовые", 0xa35cff),
];
/// (sex, id, name, hair cap, hair cards). The male scan has its buzz cut baked into the head.
const HAIRS: [(&str, &str, &str, bool, bool); 4] = [
    ("male", "buzz", "Ёжик", false, false),
    ("female", "long", "Длинные", true, true),
    ("female", "short", "Короткие", true, false),
    ("female", "shaved", "Под ноль", false, false),
];
const HAIR_COLORS: [Choice; 6] = [
    ("black", "Чёрные", 0x1e1b1a),
    ("dark", "Тёмно-русые", 0x3e2f25),
    ("brown", "Каштановые", 0x674d3a),
    ("light", "Русые", 0x9a7d5c),
    ("blond", "Светлые", 0xc2a57a),
    ("grey", "Седые", 0xa9a6a2),
];
const SUITS: [Choice; 6] = [
    ("graphite", "Графит", 0x524b46),
    ("olive", "Олива", 0x4a4d38),
    ("coyote", "Койот", 0x6b5a44),
    ("black", "Чёрный", 0x1f1f20),
    ("urban", "Серый", 0x5c6064),
    ("navy", "Тёмно-синий", 0x2a3340),
];
/// Average sRGB of the baked hair cards and suit textures; picks are scaled against them.
const HAIR_BASE: [f64; 3] = [0.406, 0.304, 0.229];
const SUIT_BASE: [f64; 3] = [0.322, 0.295, 0.270];
/// Each head's two irises in its mesh's own space (x, y, z, radius). The scans' UV atlases scatter
/// an eye over several islands, so the eyes are found by position, not by texture area.
const EYE_SPOTS: [(&str, [f64; 8]); 2] = [
    ("male", [-0.037, 1.7935, 0.086, 0.006, 0.0292, 1.793, 0.0858, 0.006]),
    ("female", [-0.283, 0.092, 0.623, 0.05, 0.266, 0.103, 0.622, 0.05]),
];
pub const CALLSIGN_MAX: usize = 16;

fn hex(c: u32) -> String {
    format!("#{c:06x}")
}
fn srgb(c: u32) -> [f64; 3] {
    [(c >> 16) & 255, (c >> 8) & 255, c & 255].map(|v| v as f64 / 255.)
}
fn linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn lin(c: [f64; 3]) -> [f64; 3] {
    c.map(linear)
}
/// Linear-space factor that turns `base` into `target` when multiplied in the shader.
/// The first choice in each list is the scan itself and stays untouched.
fn scale(target: u32, base: [f64; 3], scan: u32) -> [f64; 3] {
    if target == scan {
        return [1.; 3];
    }
    let (t, b) = (lin(srgb(target)), lin(base));
    [t[0] / b[0], t[1] / b[1], t[2] / b[2]]
}
fn round(c: [f64; 3]) -> Value {
    json!(c.map(|v| (v * 10000.).round() / 10000.))
}
fn find(list: &[Choice], id: &str) -> Option<Choice> {
    list.iter().copied().find(|c| c.0 == id)
}
fn choices(list: &[Choice]) -> Value {
    json!(list
        .iter()
        .map(|(id, name, c)| json!({"id": id, "name": name, "color": hex(*c)}))
        .collect::<Vec<_>>())
}

/// Trimmed callsign, or the reason it cannot be used.
pub fn callsign(raw: &str) -> Result<String, &'static str> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = name.chars().count();
    if count < 2 {
        return Err("Позывной: от 2 букв");
    }
    if count > CALLSIGN_MAX {
        return Err("Позывной: не длиннее 16 знаков");
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == ' ' || c == '-')
    {
        return Err("Позывной: только буквы, цифры, пробел и дефис");
    }
    Ok(name)
}

fn defaults(sex: &str) -> Value {
    let female = sex == "female";
    json!({
        "sex": if female { "female" } else { "male" },
        "callsign": "",
        "tag": "",
        "face": if female { "f1" } else { "m1" },
        "skin": "light",
        "eyes": "grey",
        "hair": if female { "long" } else { "buzz" },
        "hairColor": "brown",
        "suit": "graphite",
    })
}

/// Fills or repairs every field so the result always paints; the callsign is kept as typed.
pub fn normalize(look: &Value) -> Value {
    let sex = if text(&look["sex"]) == "female" { "female" } else { "male" };
    let mut out = defaults(sex);
    let pick = |key: &str, ok: &dyn Fn(&str) -> bool| -> Option<String> {
        let v = text(&look[key]);
        ok(v).then(|| v.to_string())
    };
    if let Some(v) = pick("face", &|v| FACES.iter().any(|f| f.0 == sex && f.1 == v)) {
        out["face"] = json!(v);
    }
    if let Some(v) = pick("hair", &|v| HAIRS.iter().any(|h| h.0 == sex && h.1 == v)) {
        out["hair"] = json!(v);
    }
    for (key, list) in [
        ("skin", &SKINS[..]),
        ("eyes", &EYES[..]),
        ("hairColor", &HAIR_COLORS[..]),
        ("suit", &SUITS[..]),
    ] {
        if let Some(v) = pick(key, &|v| find(list, v).is_some()) {
            out[key] = json!(v);
        }
    }
    let name: String = text(&look["callsign"]).chars().take(CALLSIGN_MAX + 8).collect();
    out["callsign"] = json!(name);
    out["tag"] = look["tag"].clone();
    if !out["tag"].is_string() {
        out["tag"] = json!("");
    }
    out
}

/// A finished look: every field known and a usable callsign.
pub fn valid(look: &Value) -> bool {
    look.is_object()
        && normalize(look) == *look
        && callsign(text(&look["callsign"])).as_deref() == Ok(text(&look["callsign"]))
}

/// Everything a renderer needs to show this look; colours are linear-space factors.
pub fn paint(look: &Value) -> Value {
    let look = normalize(look);
    let sex = text(&look["sex"]);
    let hair = HAIRS
        .iter()
        .find(|h| h.0 == sex && h.1 == text(&look["hair"]))
        .copied()
        .unwrap_or(HAIRS[0]);
    let hair_color = find(&HAIR_COLORS, text(&look["hairColor"])).unwrap_or(HAIR_COLORS[2]);
    let eyes = find(&EYES, text(&look["eyes"])).unwrap_or(EYES[0]);
    let spots = EYE_SPOTS.iter().find(|a| a.0 == sex).map(|a| a.1).unwrap_or([0.; 8]);
    let cap = lin(srgb(hair_color.2)).map(|v| v * 0.75);
    json!({
        "sex": sex,
        "model": format!("persona-{sex}"),
        "skin": round(lin(srgb(find(&SKINS, text(&look["skin"])).unwrap_or(SKINS[0]).2))),
        "suit": round(scale(find(&SUITS, text(&look["suit"])).unwrap_or(SUITS[0]).2, SUIT_BASE, SUITS[0].2)),
        "hairCards": round(scale(hair_color.2, HAIR_BASE, HAIR_COLORS[2].2)),
        "hairCap": round(cap),
        "cap": hair.3,
        "cards": hair.4,
        "iris": round(lin(srgb(eyes.2))),
        "eyeSpots": json!(spots),
    })
}

/// Lists for the creation screen; hair styles and faces depend on the chosen Persona.
pub fn options() -> Value {
    let per_sex = |sex: &str| {
        json!({
            "faces": FACES.iter().filter(|f| f.0 == sex)
                .map(|f| json!({"id": f.1, "name": f.2})).collect::<Vec<_>>(),
            "hairs": HAIRS.iter().filter(|h| h.0 == sex)
                .map(|h| json!({"id": h.1, "name": h.2})).collect::<Vec<_>>(),
            "hairColor": sex == "female",
        })
    };
    json!({
        "sexes": SEXES.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
        "male": per_sex("male"),
        "female": per_sex("female"),
        "skins": choices(&SKINS),
        "eyes": choices(&EYES),
        "hairColors": choices(&HAIR_COLORS),
        "suits": choices(&SUITS),
        "callsignMax": CALLSIGN_MAX,
    })
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    match method {
        "client.look.options" => Ok(options()),
        "client.look.normalize" => Ok(normalize(&args[0])),
        "client.look.paint" => Ok(paint(&args[0])),
        "client.look.callsign" => Ok(match callsign(text(&args[0])) {
            Ok(name) => json!({"ok": true, "callsign": name}),
            Err(why) => json!({"ok": false, "error": why}),
        }),
        "client.look.load" => {
            let saved = args[0]
                .as_str()
                .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
                .unwrap_or(Value::Null);
            Ok(if saved["version"] == 1 && valid(&saved["look"]) {
                saved["look"].clone()
            } else {
                Value::Null
            })
        }
        "client.look.save" => Ok(if valid(&args[0]) {
            json!({"version": 1, "look": args[0]})
        } else {
            Value::Null
        }),
        _ => Err(format!("Unknown look operation: {method}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn finished(sex: &str) -> Value {
        let mut look = normalize(&json!({"sex": sex}));
        look["callsign"] = json!("Сыч");
        look["tag"] = json!("K-417-093");
        look
    }
    #[test]
    fn a_finished_look_round_trips_through_storage() {
        for sex in ["male", "female"] {
            let look = finished(sex);
            assert!(valid(&look));
            let saved = dispatch("client.look.save", json!([look])).unwrap();
            let raw = serde_json::to_string(&saved).unwrap();
            assert_eq!(dispatch("client.look.load", json!([raw])).unwrap(), look);
        }
    }
    #[test]
    fn missing_or_broken_storage_means_no_persona_yet() {
        assert_eq!(dispatch("client.look.load", json!([null])).unwrap(), Value::Null);
        assert_eq!(dispatch("client.look.load", json!(["{"])).unwrap(), Value::Null);
        let mut look = finished("male");
        look["callsign"] = json!("");
        let raw = json!({"version": 1, "look": look}).to_string();
        assert_eq!(dispatch("client.look.load", json!([raw])).unwrap(), Value::Null);
    }
    #[test]
    fn switching_persona_drops_picks_the_other_one_lacks() {
        let mut look = finished("female");
        look["hair"] = json!("short");
        look["sex"] = json!("male");
        let fixed = normalize(&look);
        assert_eq!(fixed["hair"], "buzz");
        assert_eq!(fixed["face"], "m1");
        assert_eq!(fixed["callsign"], "Сыч");
    }
    #[test]
    fn callsigns_are_trimmed_and_checked() {
        assert_eq!(callsign("  Старый   Сыч ").unwrap(), "Старый Сыч");
        assert!(callsign("Я").is_err());
        assert!(callsign("Сыч!").is_err());
        assert!(callsign(&"а".repeat(17)).is_err());
        assert_eq!(callsign("Ворон-7").unwrap(), "Ворон-7");
    }
    #[test]
    fn default_picks_keep_the_scanned_colours() {
        let p = paint(&finished("female"));
        assert_eq!(p["hairCards"], json!([1.0, 1.0, 1.0]));
        assert_eq!(p["suit"], json!([1.0, 1.0, 1.0]));
        assert_eq!(p["skin"], json!([1.0, 1.0, 1.0]));
        assert_eq!(p["iris"], json!(round(lin(srgb(EYES[0].2)))));
        assert_eq!(p["cards"], true);
        assert_eq!(paint(&finished("male"))["model"], "persona-male");
    }
}
