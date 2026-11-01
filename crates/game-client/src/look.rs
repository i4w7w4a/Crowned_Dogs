//! Persona appearance: what the player picks at creation and how each pick tints the model.
use crate::text;
use serde_json::{json, Value};

/// (id, name, sRGB colour). Colours are targets or multipliers, see `paint`.
type Choice = (&'static str, &'static str, u32);

const SEXES: [(&str, &str); 2] = [("male", "Мужская"), ("female", "Женская")];
/// Each face is its own head mesh in the Persona's model (material `M_Head_<id>`); the first is the default.
const FACES: [(&str, &str, &str); 8] = [
    ("male", "m1", "Лицо 1"),
    ("male", "m2", "Лицо 2"),
    ("male", "m3", "Лицо 3"),
    ("male", "m4", "Лицо 4"),
    ("female", "f1", "Лицо 1"),
    ("female", "f2", "Лицо 2"),
    ("female", "f3", "Лицо 3"),
    ("female", "f4", "Лицо 4"),
];
/// Multipliers over the baked skin texture; white keeps the scan as it is.
const SKINS: [Choice; 6] = [
    ("light", "Светлая", 0xffffff),
    ("warm", "Тёплая", 0xecd2bc),
    ("tan", "Смуглая", 0xd2aa88),
    ("olive", "Оливковая", 0xbea27e),
    ("brown", "Тёмная", 0x9c7458),
    ("deep", "Очень тёмная", 0x6e4c38),
];
/// Eye glow colours, in hue order, taken from neon palettes. The first six ids are the old iris
/// list's, so saved looks still load.
const EYES: [Choice; 14] = [
    ("grey", "Лёд", 0xa8ecff),
    ("cyan", "Циан", 0x0affef),
    ("teal", "Бирюза", 0x00ffb4),
    ("green", "Изумруд", 0x1dff87),
    ("acid", "Кислота", 0x9dff14),
    ("gold", "Золото", 0xffe14a),
    ("hazel", "Янтарь", 0xffb703),
    ("ember", "Угли", 0xff6a1f),
    ("brown", "Алые", 0xff2a3d),
    ("rose", "Розовые", 0xff2079),
    ("magenta", "Маджента", 0xff2daa),
    ("dark", "Фиолетовые", 0xb026ff),
    ("indigo", "Индиго", 0x6a4dff),
    ("blue", "Синие", 0x00a3ff),
];
/// (sex, id, name, hair cap, hair cards). The male scan has its buzz cut baked into the head.
/// The others (female pixie, bob, braid; male crop, slick, medium) are their own meshes in the
/// Persona's hair pack (`Hair_<id>_*`), not the long hair's cap and cards; the renderer shows them
/// by the paint's `hair` id.
const HAIRS: [(&str, &str, &str, bool, bool); 9] = [
    ("male", "buzz", "Ёжик", false, false),
    ("male", "crop", "Короткая", false, false),
    ("male", "slick", "Зачёс назад", false, false),
    ("male", "medium", "Средняя", false, false),
    ("female", "long", "Длинные", true, true),
    ("female", "pixie", "Пикси", false, false),
    ("female", "bob", "Боб", false, false),
    ("female", "braid", "Коса", false, false),
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
const BEARDS: [(&str, &str); 5] = [
    ("stubble", "Щетина"),
    ("boxed", "Короткая"),
    ("goatee", "Эспаньолка"),
    ("full", "Полная"),
    ("tapered", "Клиновидная"),
];
const BEARD_COLORS: [Choice; 6] = [
    ("black", "Чёрный", 0x1e1b1a),
    ("dark", "Тёмно-русый", 0x3e2f25),
    ("brown", "Каштановый", 0x674d3a),
    ("light", "Русый", 0x9a7d5c),
    ("blond", "Светлый", 0xc2a57a),
    ("grey", "Седой", 0xa9a6a2),
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
/// Beard atlas colour contract; independent of the existing female hair atlas.
const BEARD_BASE: [f64; 3] = [0.406, 0.304, 0.229];
const SUIT_BASE: [f64; 3] = [0.322, 0.295, 0.270];
/// Suit fabrics: their colour is the suit colour itself.
const SUIT_MATERIALS: [&str; 4] = ["M_Suit", "M_Solonchak", "M_Ten", "M_Ryvok"];
/// Other worn pieces; their textures are neutral greys like the suit's, so a colour shifts them by the suit's factor.
const GEAR_MATERIALS: [&str; 7] = ["M_Cap", "M_Buff", "M_Boots", "M_Helmet", "M_Sfera", "M_Plita4", "M_Vest"];
/// Starter cloth pieces wear the suit's colour unless given one of their own.
const FOLLOW_SUIT: [&str; 3] = ["M_Cap", "M_Buff", "M_Boots"];
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
        "beard": if female { "none" } else { "stubble" },
        "beardColor": "brown",
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
    if sex == "male" {
        if let Some(v) = pick("beard", &|v| BEARDS.iter().any(|b| b.0 == v)) {
            out["beard"] = json!(v);
        }
    }
    for (key, list) in [
        ("skin", &SKINS[..]),
        ("eyes", &EYES[..]),
        ("hairColor", &HAIR_COLORS[..]),
        ("beardColor", &BEARD_COLORS[..]),
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
    let face = text(&look["face"]);
    let cap = lin(srgb(hair_color.2)).map(|v| v * 0.75);
    json!({
        "sex": sex,
        "model": format!("persona-{sex}"),
        "face": face,
        "skin": round(lin(srgb(find(&SKINS, text(&look["skin"])).unwrap_or(SKINS[0]).2))),
        "suit": round(scale(find(&SUITS, text(&look["suit"])).unwrap_or(SUITS[0]).2, SUIT_BASE, SUITS[0].2)),
        "hairCards": round(scale(hair_color.2, HAIR_BASE, HAIR_COLORS[2].2)),
        "hairCap": round(cap),
        "beard": look["beard"],
        "beardColor": round(scale(find(&BEARD_COLORS, text(&look["beardColor"])).unwrap_or(BEARD_COLORS[2]).2, BEARD_BASE, BEARD_COLORS[2].2)),
        "hair": hair.1,
        "cap": hair.3,
        "cards": hair.4,
        "iris": round(lin(srgb(eyes.2))),
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
            "hairColor": true,
            "beards": if sex == "male" { BEARDS.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>() } else { vec![] },
        })
    };
    json!({
        "sexes": SEXES.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
        "male": per_sex("male"),
        "female": per_sex("female"),
        "skins": choices(&SKINS),
        "eyes": choices(&EYES),
        "hairColors": choices(&HAIR_COLORS),
        "beardColors": choices(&BEARD_COLORS),
        "suits": choices(&SUITS),
        "callsignMax": CALLSIGN_MAX,
    })
}

/// Colours a worn piece can take: as the suit, as made, or one of the suit colours.
pub fn gear_colors() -> Value {
    let mut list = vec![
        json!({"id": "suit", "name": "Как костюм"}),
        json!({"id": "own", "name": "Родной"}),
    ];
    list.extend(choices(&SUITS).as_array().cloned().unwrap_or_default());
    json!(list)
}
pub fn gear_color(id: &str) -> bool {
    id == "suit" || id == "own" || find(&SUITS, id).is_some()
}

/// Linear factors for every worn material: `colors` maps a material to `suit`, `own` or a suit colour id.
/// Unset, the suit fabrics and the starter cloth wear the suit colour; hard pieces stay as made.
pub fn gear_tints(look: &Value, colors: &Value) -> Value {
    let look = normalize(look);
    let pick = |m: &str| colors[m].as_str().filter(|c| gear_color(c));
    let persona = text(&look["suit"]).to_string();
    // the suit actually worn sets "as the suit" for the rest
    let worn = SUIT_MATERIALS
        .iter()
        .find_map(|m| pick(m).filter(|c| *c != "suit" && *c != "own"))
        .unwrap_or(&persona)
        .to_string();
    let suit = |id: &str| scale(find(&SUITS, id).unwrap_or(SUITS[0]).2, SUIT_BASE, SUITS[0].2);
    let mut out = serde_json::Map::new();
    for m in SUIT_MATERIALS {
        let factor = match pick(m) {
            Some("own") => [1.; 3],
            Some("suit") | None => suit(&persona),
            Some(id) => suit(id),
        };
        out.insert(m.into(), round(factor));
    }
    for m in GEAR_MATERIALS {
        let id = match pick(m) {
            Some("own") => None,
            Some("suit") => Some(worn.as_str()),
            None if FOLLOW_SUIT.contains(&m) => Some(worn.as_str()),
            None => None,
            Some(id) => Some(id),
        };
        out.insert(m.into(), round(id.map_or([1.; 3], suit)));
    }
    Value::Object(out)
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    match method {
        "client.look.options" => Ok(options()),
        "client.look.gearTints" => Ok(gear_tints(&args[0], &args[1])),
        "client.look.gearColors" => Ok(gear_colors()),
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
            let mut look = saved["look"].clone();
            if saved["version"] == 1 {
                // Keep the existing storage key and identity. Only fill the new fields;
                // malformed legacy data still fails the same strict validation below.
                if let Some(fields) = look.as_object_mut() {
                    fields.entry("beard").or_insert(json!("none"));
                    fields.entry("beardColor").or_insert(json!("brown"));
                }
            }
            // Light stubble replaced the clean-shaven choice: a saved shaven man wears it now.
            if look["sex"] != "female" && look["beard"] == "none" {
                look["beard"] = json!("stubble");
            }
            Ok(if (saved["version"] == 1 || saved["version"] == 2) && valid(&look) {
                look
            } else {
                Value::Null
            })
        }
        "client.look.save" => Ok(if valid(&args[0]) {
            json!({"version": 2, "look": args[0]})
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
    fn starter_cloth_follows_the_suit_and_hard_pieces_keep_their_own_colour() {
        let mut look = finished("male");
        look["suit"] = json!("olive");
        let olive = paint(&look)["suit"].clone();
        let tints = gear_tints(&look, &json!({}));
        for m in ["M_Suit", "M_Cap", "M_Buff", "M_Boots"] {
            assert_eq!(tints[m], olive, "{m}");
        }
        assert_eq!(tints["M_Plita4"], json!([1.0, 1.0, 1.0]));
        // a suit painted in the kit takes the cloth with it; a piece of its own keeps its pick
        let tints = gear_tints(&look, &json!({"M_Suit": "navy", "M_Boots": "own", "M_Plita4": "suit"}));
        assert_eq!(tints["M_Cap"], tints["M_Suit"]);
        assert_eq!(tints["M_Plita4"], tints["M_Suit"]);
        assert_eq!(tints["M_Boots"], json!([1.0, 1.0, 1.0]));
        assert_ne!(tints["M_Suit"], olive);
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
    fn legacy_identity_and_all_picks_survive_beard_migration() {
        for sex in ["male", "female"] {
            let mut old = finished(sex);
            old["face"] = json!(if sex == "male" { "m3" } else { "f4" });
            old["skin"] = json!("olive");
            old["eyes"] = json!("ember");
            old["hairColor"] = json!("grey");
            old["suit"] = json!("navy");
            let expected = old.clone();
            old.as_object_mut().unwrap().remove("beard");
            old.as_object_mut().unwrap().remove("beardColor");
            let loaded = dispatch("client.look.load", json!([json!({"version": 1, "look": old}).to_string()])).unwrap();
            assert_eq!(loaded, expected);
            let saved = dispatch("client.look.save", json!([loaded])).unwrap();
            assert_eq!(saved["version"], 2);
            assert_eq!(dispatch("client.look.load", json!([saved.to_string()])).unwrap(), expected);
        }
    }
    #[test]
    fn beard_is_independent_of_face_and_hair_and_only_exists_on_male() {
        for (style, _) in BEARDS {
            for face in ["m1", "m2", "m3", "m4"] {
                let mut look = finished("male");
                look["face"] = json!(face);
                look["beard"] = json!(style);
                look["beardColor"] = json!("grey");
                assert!(valid(&look));
                let painted = paint(&look);
                assert_eq!(painted["beard"], style);
                assert_eq!(painted["face"], face);
                assert_eq!(look["hairColor"], "brown");
                look["sex"] = json!("female");
                assert_eq!(normalize(&look)["beard"], "none");
            }
        }
        let mut look = finished("male");
        look["beard"] = json!("not-a-style");
        assert!(!valid(&look));
        assert_eq!(normalize(&look)["beard"], "stubble");
        assert!(dispatch("client.look.load", json!([json!({"version": 99, "look": finished("male")}).to_string()])).unwrap().is_null());
    }
    #[test]
    fn a_saved_clean_shaven_man_loads_with_stubble() {
        let mut old = finished("male");
        old["beard"] = json!("none");
        for version in [1, 2] {
            let loaded = dispatch("client.look.load", json!([json!({"version": version, "look": old}).to_string()])).unwrap();
            assert_eq!(loaded["beard"], "stubble");
            assert_eq!(loaded["callsign"], "Сыч");
        }
        assert_eq!(finished("male")["beard"], "stubble");
        assert_eq!(finished("female")["beard"], "none");
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
        look["hair"] = json!("shaved");
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
    #[test]
    fn new_hairstyles_drop_the_long_hair_and_stay_female() {
        for id in ["pixie", "bob", "braid"] {
            let mut look = finished("female");
            look["hair"] = json!(id);
            let p = paint(&normalize(&look));
            assert_eq!(p["hair"], id);
            assert_eq!(p["cards"], false);
            assert_eq!(p["cap"], false);
            look["sex"] = json!("male");
            assert_eq!(normalize(&look)["hair"], "buzz");
        }
    }
    #[test]
    fn male_hairstyles_stay_male() {
        for id in ["crop", "slick", "medium"] {
            let mut look = finished("male");
            look["hair"] = json!(id);
            let p = paint(&normalize(&look));
            assert_eq!(p["hair"], id);
            assert_eq!(p["cards"], false);
            assert_eq!(p["cap"], false);
            look["sex"] = json!("female");
            assert_eq!(normalize(&look)["hair"], "long");
        }
    }
    #[test]
    fn each_face_is_painted_by_its_own_id() {
        for (sex, id, _) in FACES {
            let mut look = finished(sex);
            look["face"] = json!(id);
            assert_eq!(paint(&look)["face"], id);
        }
        let mut look = finished("female");
        look["face"] = json!("m2");
        assert_eq!(paint(&look)["face"], "f1");
    }
}
