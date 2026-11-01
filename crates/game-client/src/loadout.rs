use crate::{core, text};
use serde_json::{json, Map, Value};

const GEAR: [&str; 8] = [
    "weapon1", "weapon2", "head", "body", "suit", "boots", "knife", "grenade",
];
const MODS: [&str; 5] = ["optic", "muzzle", "rail", "mag", "stock"];

fn weapon_shape(v: &Value) -> bool {
    let Some(o) = v.as_object() else {
        return false;
    };
    o.get("item").is_some_and(Value::is_string)
        && o.get("skin").is_none_or(Value::is_string)
        && o.get("mods").is_none_or(|m| {
            m.as_object().is_some_and(|m| {
                m.iter()
                    .all(|(k, v)| MODS.contains(&k.as_str()) && v.is_string())
            })
        })
}
fn valid(v: &Value) -> bool {
    let Some(o) = v.as_object() else {
        return false;
    };
    weapon_shape(&v["weapon1"])
        && o.get("weapon2")
            .is_some_and(|w| w.is_null() || weapon_shape(w))
        && GEAR[2..].iter().all(|slot| v[*slot].is_string())
        && o.get("colors").is_none_or(|c| {
            c.as_object()
                .is_some_and(|c| c.values().all(|v| v.as_str().is_some_and(crate::look::gear_color)))
        })
        && core("validateLoadout", json!([v])).is_ok_and(|e| e.is_null())
}
fn clone_weapon(v: &Value) -> Value {
    let mut o = Map::new();
    o.insert("item".into(), v["item"].clone());
    for k in ["mods", "skin"] {
        if let Some(value) = v.get(k) {
            o.insert(k.into(), value.clone());
        }
    }
    Value::Object(o)
}
fn clone_kit(v: &Value) -> Value {
    let mut o = Map::new();
    o.insert("weapon1".into(), clone_weapon(&v["weapon1"]));
    o.insert(
        "weapon2".into(),
        if v["weapon2"].is_null() {
            Value::Null
        } else {
            clone_weapon(&v["weapon2"])
        },
    );
    for k in &GEAR[2..] {
        o.insert((*k).into(), v[*k].clone());
    }
    // each worn piece's own colour, by item id
    if v["colors"].as_object().is_some_and(|c| !c.is_empty()) {
        o.insert("colors".into(), v["colors"].clone());
    }
    Value::Object(o)
}
fn items(slot: &str) -> Result<Value, String> {
    let d = game_core::constants();
    let catalog = d["ITEMS"].as_array().ok_or("Missing item catalogue")?;
    let result: Vec<_> = catalog
        .iter()
        .filter(|item| {
            if slot == "weapon1" || slot == "weapon2" {
                text(&item["kind"]) == "weapon"
                    && (slot == "weapon1" || d["WEAPONS"][text(&item["weapon"])]["hands"] == 1)
            } else {
                item["kind"] == d["KIND_FOR_SLOT"][slot]
            }
        })
        .cloned()
        .collect();
    Ok(json!(result))
}
fn equipped(kit: &Value, slot: &str) -> Value {
    if slot == "weapon1" || slot == "weapon2" {
        kit[slot]["item"].clone()
    } else {
        kit[slot].clone()
    }
}
fn weapon_for_mod(kit: &Value, slot: &str) -> Result<Value, String> {
    if !valid(kit) {
        return Err("Некорректный комплект снаряжения".into());
    }
    if !MODS.contains(&slot) {
        return Err("Неизвестное гнездо оружия".into());
    }
    let weapon = core("getItem", json!([kit["weapon1"]["item"]]))?;
    if weapon["kind"] != "weapon" {
        return Err("В первом слоте нет оружия".into());
    }
    Ok(weapon)
}
/// Worn gear that left the catalogue (concept items without a model) goes back to the starter piece.
fn gear_left_catalogue(kit: &Value) -> Value {
    let mut kit = kit.clone();
    let Ok(starter) = core("starterLoadout", json!(["rifle"])) else {
        return kit;
    };
    for slot in &GEAR[2..] {
        if kit[*slot].as_str().is_some_and(|id| core("getItem", json!([id])).is_err()) {
            kit[*slot] = starter[*slot].clone();
        }
    }
    kit
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    let kit = &args[0];
    let slot = text(&args[1]);
    let id = &args[2];
    match method {
        "client.isValidLoadout" => Ok(json!(valid(kit))),
        "client.load" => {
            if let Some(raw) = args[0].as_str() {
                if let Ok(saved) = serde_json::from_str::<Value>(raw) {
                    if saved["version"] == 1 {
                        let kit = gear_left_catalogue(&saved["loadout"]);
                        if valid(&kit) {
                            return Ok(clone_kit(&kit));
                        }
                    }
                }
            }
            core("starterLoadout", json!(["rifle"]))
        }
        "client.save" => Ok(if valid(kit) {
            json!({"version":1,"loadout":clone_kit(kit)})
        } else {
            Value::Null
        }),
        "client.itemsForSlot" => items(text(&args[0])),
        "client.equippedItemId" => Ok(equipped(kit, slot)),
        "client.equipItem" => {
            if !valid(kit) {
                return Err("Некорректный комплект снаряжения".into());
            }
            if !GEAR.contains(&slot) {
                return Err("Неизвестный слот снаряжения".into());
            }
            if id.is_null() && slot != "weapon2" {
                return Err("Этот слот нельзя оставить пустым".into());
            }
            if !id.is_null()
                && !items(slot)?
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|x| &x["id"] == id)
            {
                return Err("Предмет не подходит для этого слота".into());
            }
            let mut next = clone_kit(kit);
            if slot == "weapon2" && id.is_null() {
                next[slot] = Value::Null;
            } else if slot == "weapon1" || slot == "weapon2" {
                if equipped(&next, slot) != *id {
                    next[slot] = json!({"item":id});
                }
            } else {
                next[slot] = id.clone();
            }
            if !core("validateLoadout", json!([next]))?.is_null() {
                return Err("Предмет несовместим с комплектом".into());
            }
            Ok(next)
        }
        "client.gearColors" => Ok(crate::look::gear_colors()),
        "client.itemColor" => {
            // unset, cloth wears the suit colour and hard pieces stay as made (look::gear_tints)
            let item = text(&args[1]);
            let cloth = item == "head-cap" || item.starts_with("suit-") || item.starts_with("boots-");
            Ok(kit["colors"][item]
                .as_str()
                .map_or(json!(if cloth { "suit" } else { "own" }), |c| json!(c)))
        }
        "client.paintItem" => {
            if !valid(kit) {
                return Err("Некорректный комплект снаряжения".into());
            }
            let item = text(&args[1]);
            if !GEAR[2..].iter().any(|s| kit[*s] == item) {
                return Err("Предмет не надет".into());
            }
            let color = text(&args[2]);
            if !crate::look::gear_color(color) {
                return Err("Нет такого цвета".into());
            }
            let mut next = clone_kit(kit);
            let mut colors = next["colors"].as_object().cloned().unwrap_or_default();
            colors.insert(item.into(), json!(color));
            next["colors"] = Value::Object(colors);
            Ok(next)
        }
        "client.weaponModsForSlot" => {
            let weapon = weapon_for_mod(kit, slot)?;
            core("modsFor", json!([weapon["weapon"], slot]))
        }
        "client.effectiveWeaponModId" => {
            let weapon = weapon_for_mod(kit, slot)?;
            Ok(kit["weapon1"]["mods"]
                .get(slot)
                .unwrap_or(&weapon["stock"][slot])
                .clone())
        }
        "client.equipWeaponMod" => {
            let weapon = weapon_for_mod(kit, slot)?;
            if !id.is_null()
                && !core("modsFor", json!([weapon["weapon"], slot]))?
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|m| &m["id"] == id)
            {
                return Err("Модификация не подходит к этому гнезду оружия".into());
            }
            let mut next = clone_kit(kit);
            let mut mods = next["weapon1"]["mods"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            if id.is_null() || id == &weapon["stock"][slot] {
                mods.remove(slot);
            } else {
                mods.insert(slot.into(), id.clone());
            }
            if mods.is_empty() {
                next["weapon1"].as_object_mut().unwrap().remove("mods");
            } else {
                next["weapon1"]["mods"] = Value::Object(mods);
            }
            if !core("validateLoadout", json!([next]))?.is_null() {
                return Err("Модификация несовместима с комплектом".into());
            }
            Ok(next)
        }
        _ => Err(format!("Unknown loadout operation: {method}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_saved_kit_keeps_its_guns_when_worn_gear_left_the_catalogue() {
        let mut kit = core("starterLoadout", json!(["sniper"])).unwrap();
        kit["body"] = json!("body-light");
        let saved = json!({"version": 1, "loadout": kit}).to_string();
        let back = dispatch("client.load", json!([saved])).unwrap();
        assert!(back["weapon1"]["item"].as_str().unwrap().starts_with("sniper"));
        assert_eq!(back["body"], core("starterLoadout", json!(["rifle"])).unwrap()["body"]);
    }
}
