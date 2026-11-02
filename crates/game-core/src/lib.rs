//! Deterministic, platform-independent Crowned Dogs rules. JSON is the versioned
//! boundary shared by the browser WASM build and the native Windows host.
use serde_json::{json, Value};
use std::sync::OnceLock;
mod ai;
mod combat;
mod draft;
mod loadout;
pub mod rng;
mod sync;
mod tag;

pub(crate) const ZONES: [&str; 4] = ["head", "body", "groin", "legs"];
pub(crate) const BANDS: [&str; 3] = ["near", "mid", "far"];
pub(crate) const MODS: [&str; 5] = ["optic", "muzzle", "rail", "mag", "stock"];
pub(crate) const GEAR: [&str; 8] = [
    "weapon1", "weapon2", "head", "body", "suit", "boots", "knife", "grenade",
];
pub fn constants() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/catalog.json")).expect("embedded catalog")
    })
}
pub(crate) fn n(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}
pub(crate) fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}
pub(crate) fn b(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}
pub(crate) fn a(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub(crate) fn num(v: f64) -> Value {
    json!(v)
}
pub(crate) fn round(v: f64) -> f64 {
    (v + 0.5).floor()
}
pub(crate) fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}
pub(crate) fn band(d: f64) -> &'static str {
    if d <= 4.0 {
        "near"
    } else if d <= 8.0 {
        "mid"
    } else {
        "far"
    }
}
pub(crate) fn contains(v: &Value, t: &str) -> bool {
    a(v).iter().any(|x| s(x) == t)
}
pub(crate) fn merge(dst: &mut Value, src: &Value) {
    if let Some(o) = src.as_object() {
        for (k, v) in o {
            dst[k] = v.clone();
        }
    }
}
pub(crate) fn armor_factor(armor: f64, pen: f64, flat: f64, anti: f64) -> f64 {
    let p = clamp(pen - anti, 0.0, 0.74);
    let armor = (armor * (1.0 - p) - flat).max(0.0);
    1.0 - (0.74 * armor) / (47.0 + armor)
}
pub(crate) fn base_damage(w: &Value, d: f64) -> f64 {
    let idx = d.min(16.0) as isize - 1;
    let fall = if idx >= 0 {
        w["falloff"]
            .get(idx as usize)
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
    } else {
        1.0
    };
    (27.0 + n(&w["power"]) * 0.74) * fall
}
pub(crate) fn grenade_damage(armor: f64, mv: f64, g: &Value) -> f64 {
    let full = n(&g["base"]) * armor_factor(armor, n(&g["armorPen"]), 0.0, 0.04);
    (if mv.abs() == 0.0 {
        full
    } else if mv.abs() == 1.0 {
        full / 2.0
    } else {
        0.0
    })
    .floor()
}
pub(crate) fn item(id: &str) -> Result<Value, String> {
    a(&constants()["ITEMS"])
        .iter()
        .find(|v| s(&v["id"]) == id)
        .cloned()
        .ok_or_else(|| format!("Unknown item: {id}"))
}
pub(crate) fn weapon_item(w: &str) -> Result<Value, String> {
    a(&constants()["ITEMS"])
        .iter()
        .find(|v| s(&v["kind"]) == "weapon" && s(&v["weapon"]) == w)
        .cloned()
        .ok_or_else(|| format!("No weapon item for {w}"))
}

/// Calls use positional JSON argument arrays. Optional omitted arguments are null.
/// RNG-mutating functions return `{value,state}` so bridges can update caller arrays.
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    let x = |i: usize| args.get(i).unwrap_or(&Value::Null);
    let side = |i: usize| n(x(i)) as usize;
    Ok(match method {
        "constants" => constants().clone(),
        "seedRng" => json!(rng::seed_value(x(0))),
        "nextFloat" | "nextInt" | "roll" | "pickWeighted" => {
            let mut state = rng::state(x(0));
            let f = rng::next(&mut state);
            let value = match method {
                "nextInt" => num((f * n(x(1))).floor()),
                "roll" => json!(f < n(x(1))),
                "pickWeighted" => {
                    let list = a(x(1));
                    if list.is_empty() {
                        return Err("pickWeighted: empty list".into());
                    }
                    let mut r = f * list.iter().map(|e| n(&e[1])).sum::<f64>();
                    let mut chosen = list.last().unwrap()[0].clone();
                    for e in list {
                        r -= n(&e[1]);
                        if r < 0.0 {
                            chosen = e[0].clone();
                            break;
                        }
                    }
                    chosen
                }
                _ => num(f),
            };
            json!({"value":value,"state":state})
        }
        "band" => json!(band(n(x(0)))),
        "armorFactor" => num(armor_factor(n(x(0)), n(x(1)), n(x(2)), n(x(3)))),
        "baseDamage" => num(base_damage(x(0), n(x(1)))),
        "grenadeDamage" => num(grenade_damage(
            n(x(0)),
            n(x(1)),
            if x(2).is_null() {
                &constants()["GRENADE"]
            } else {
                x(2)
            },
        )),
        "getItem" => item(s(x(0)))?,
        "weaponItem" => weapon_item(s(x(0)))?,
        "modsFor" => {
            let mut list: Vec<Value> = a(&constants()["ITEMS"])
                .iter()
                .filter(|v| {
                    s(&v["kind"]) == "mod"
                        && s(&v["slot"]) == s(x(1))
                        && contains(&v["fits"], s(x(0)))
                })
                .cloned()
                .collect();
            list.sort_by(|u, v| {
                b(&v["isStock"])
                    .cmp(&b(&u["isStock"]))
                    .then(n(&u["tier"]).total_cmp(&n(&v["tier"])))
            });
            json!(list)
        }
        "starterLoadout" => {
            let mut l = constants()["STARTER"].clone();
            l["weapon1"] = json!({"item":weapon_item(s(x(0)))?["id"]});
            l
        }
        "validateLoadout" => loadout::validate(x(0)),
        "weaponParts" => loadout::parts(x(0))?,
        "applyWeaponEffects" => loadout::apply(x(0), a(x(1))),
        "compileKit" => loadout::compile(x(0))?,
        "kitFighter" => {
            let kit = loadout::compile(x(2))?;
            json!({"id":x(0),"name":x(1),"weapon":kit["weapon"]["id"],"kit":kit})
        }
        "syncFresh" => sync::fresh(),
        "syncClean" => sync::clean(x(0)),
        "syncPlays" => json!(sync::plays(x(0))),
        "syncApply" => sync::apply(x(0), x(1)),
        "syncSpend" => sync::spend(x(0), s(x(1))),
        "syncBot" => sync::bot(x(0), s(x(1))),
        "syncGain" => sync::gain(x(0), x(1), side(2)),
        "syncView" => sync::view(x(0)),
        "effectLines" => json!(loadout::effect_lines(x(0))),
        "describeEffects" => json!(loadout::effect_lines(x(0))
            .iter()
            .map(|l| l["text"].clone())
            .collect::<Vec<_>>()),
        "describeGrenade" => loadout::describe_grenade(x(0)),
        "describeItem" => {
            if s(&x(0)["kind"]) == "grenade" {
                loadout::describe_grenade(&x(0)["grenade"])
            } else {
                json!(loadout::effect_lines(&x(0)["effects"])
                    .iter()
                    .map(|l| l["text"].clone())
                    .collect::<Vec<_>>())
            }
        }
        "nextTag" => {
            let taken = x(1).as_array().ok_or("tags must be a list")?;
            json!(tag::next(n(x(0)), taken.iter().filter_map(Value::as_str))?)
        }
        "localTag" => json!(tag::local(n(x(0)))),
        "formatTag" => {
            let v = n(x(0));
            if v.fract() != 0.0 || !(0.0..1_000_000.0).contains(&v) {
                return Err(format!("no tag for {v}"));
            }
            let t = format!("{:06}", v as u32);
            json!(format!("K-{}-{}", &t[..3], &t[3..]))
        }
        "parseTag" => {
            let text = s(x(0)).trim();
            let mut chars = text.chars();
            let first = chars.next();
            if !matches!(first, Some('K' | 'k' | 'К' | 'к')) {
                Value::Null
            } else {
                let tail = chars.as_str();
                let tail = tail.trim_start_matches(|c: char| c == '-' || c.is_whitespace());
                let mut pos = 0;
                for c in tail.chars().take(3) {
                    if c.is_ascii_digit() {
                        pos += 1;
                    } else {
                        break;
                    }
                }
                if pos != 3 {
                    Value::Null
                } else {
                    let last =
                        tail[3..].trim_start_matches(|c: char| c == '-' || c.is_whitespace());
                    if last.len() == 3 && last.bytes().all(|c| c.is_ascii_digit()) {
                        json!(format!("K-{}-{last}", &tail[..3]))
                    } else {
                        Value::Null
                    }
                }
            }
        }
        "other" => json!(1 - side(0)),
        "createMatch" => combat::create(x(0))?,
        "weaponSpec" => combat::weapon(x(0), side(1), s(x(2))),
        "weaponSpecSource" => {
            let kit = &x(0)["config"]["fighters"][side(1)]["kit"];
            let w = s(x(2));
            if kit.is_null() {
                Value::Null
            } else if w == "knife" {
                json!("knife")
            } else if b(&x(0)["fighters"][side(1)]["sidearm"]) && s(&kit["sidearm"]["id"]) == w {
                json!("sidearm")
            } else if s(&kit["weapon"]["id"]) == w {
                json!("weapon")
            } else if s(&kit["sidearm"]["id"]) == w {
                json!("sidearm")
            } else {
                Value::Null
            }
        }
        "swapWeapon" => combat::swap(x(0), side(1)),
        "grenadeSpec" => combat::grenade(x(0), side(1)),
        "grenadeHit" => num(combat::grenade_hit(x(0), side(1), n(x(2)))),
        "guardShare" => num(combat::guard(x(0), side(1), s(x(2)))),
        "turnLimit" => combat::weather(x(0))["turnLimit"].clone(),
        "turnSeconds" => combat::weather(x(0))["turnSeconds"].clone(),
        "distanceLimit" => num(combat::limit(
            x(0),
            if x(1).is_null() {
                n(&x(0)["turn"])
            } else {
                n(x(1))
            },
        )),
        "moveOf" => num(combat::move_of(x(0))),
        "defendCount" => json!(combat::defend_count(x(0))),
        "plannedDistance" => num(combat::planned(x(0), n(x(1)))),
        "canFlee" => json!(combat::can_flee(x(0))),
        "canThrowGrenade" => json!(combat::can_grenade(x(0), side(1))),
        "canUseKnife" => json!(combat::can_knife(x(0), n(x(1)))),
        "canSwap" => json!(!combat::swap(x(0), side(1)).is_null()),
        "validateOrder" => combat::validate(x(0), side(1), x(2)),
        "conditionAccuracy" => num(combat::condition(x(0), side(1), n(x(2)))),
        "hitChance" => num(combat::hit_chance(
            x(0),
            side(1),
            s(x(2)),
            n(x(3)),
            n(x(4)),
            b(x(5)),
        )),
        "critChance" => num(combat::crit(
            x(0),
            s(x(1)),
            if x(2).is_null() { None } else { Some(side(2)) },
        )),
        "expectedHit" => num(combat::expected(x(0), side(1), s(x(2)), s(x(3)), n(x(4)))),
        "resolveTurn" => combat::resolve(x(0), x(1))?,
        "freshDraft" => draft::fresh(x(0), side(1)),
        "grenadeRefusal" => json!(draft::grenade_refusal(x(0), side(1))),
        "guardsNeeded" => json!(draft::guards(x(0))),
        "applyCommand" => draft::apply(x(0), side(1), x(2), x(3)),
        "actionOf" => draft::action(x(0)),
        "missing" => draft::missing(x(0)),
        "toOrder" => draft::order(x(0), side(1), x(2)),
        "draftChance" => draft::chance(x(0), side(1), x(2)),
        "botRng" => json!(ai::bot_rng(x(0), side(1))),
        "botOrder" => {
            let explicit = !x(2).is_null();
            let mut state = if explicit {
                rng::state(x(2))
            } else {
                ai::bot_rng(x(0), side(1))
            };
            let style = if x(3).is_null() {
                &constants()["DEFAULT_BOT"]
            } else {
                x(3)
            };
            let value = ai::order(x(0), side(1), &mut state, style);
            if explicit {
                json!({"value":value,"state":state})
            } else {
                value
            }
        }
        _ => return Err(format!("Unknown game-core method: {method}")),
    })
}
