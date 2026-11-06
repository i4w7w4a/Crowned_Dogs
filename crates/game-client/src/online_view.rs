//! Presentation only: canonical server side 1 becomes local side 0 for the existing arena.
//! No orders, randomness, damage or rewards are computed here.
use serde_json::{Value, json};

fn pair(value: &mut Value, key: &str) {
    if let Some(a) = value.get_mut(key).and_then(Value::as_array_mut) {
        if a.len() == 2 {
            a.swap(0, 1);
        }
    }
}
fn lane(value: &mut Value) {
    pair(value, "lane");
    if let Some(a) = value.get_mut("lane").and_then(Value::as_array_mut) {
        for x in a {
            if let Some(n) = x.as_f64() {
                *x = json!(-n);
            }
        }
    }
}
fn result(value: &mut Value) {
    if let Some(side) = value["winner"].as_u64().filter(|side| *side <= 1) {
        value["winner"] = json!(1 - side);
    }
}
fn hazard(value: &mut Value) {
    if let Some(n) = value["at"].as_f64() {
        value["at"] = json!(-n);
    }
    pair(value, "hpBefore");
    if let Some(hits) = value.get_mut("hits").and_then(Value::as_array_mut) {
        for hit in hits {
            if let Some(side) = hit["side"].as_u64().filter(|side| *side <= 1) {
                hit["side"] = json!(1 - side);
            }
        }
    }
}
fn report(value: &mut Value) {
    for key in [
        "moves", "actions", "attacks", "defend", "hp", "afk", "stunned",
    ] {
        pair(value, key);
    }
    lane(value);
    if let Some(h) = value.get_mut("hazard") {
        hazard(h);
    }
}
fn view(battle: &Value) -> Result<Value, String> {
    let side = battle["side"]
        .as_u64()
        .filter(|side| *side <= 1)
        .ok_or("Invalid online side")?;
    let mut m = battle["match"].clone();
    let mut fighters = battle["players"].clone();
    if side == 1 {
        pair(&mut m, "fighters");
        if let Some(c) = m.get_mut("config") {
            pair(c, "fighters");
        }
        lane(&mut m);
        if let Some(r) = m.get_mut("result") {
            result(r);
        }
        if let Some(h) = m.get_mut("hazard") {
            hazard(h);
        }
        if let Some(history) = m.get_mut("history").and_then(Value::as_array_mut) {
            history.iter_mut().for_each(report);
        }
        if let Some(f) = fighters.as_array_mut().filter(|f| f.len() == 2) {
            f.swap(0, 1);
        }
    }
    if let Some(f) = fighters.as_array_mut() {
        for (side, player) in f.iter_mut().enumerate() {
            player["kit"] = player["loadout"].clone();
            player["weapon"] = m["fighters"][side]["weapon"].clone();
        }
    }
    Ok(
        json!({"match":m,"fighters":fighters,"draft":battle["own"]["draft"],
        "over":battle["phase"]=="finished" || battle["phase"]=="cancelled"}),
    )
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    match method {
        "client.online.view" => view(&args[0]),
        "client.online.profileActions" => {
            let before = &args[0];
            let after = &args[1];
            let mut actions = Vec::new();
            for slot in [
                "weapon1", "weapon2", "head", "body", "suit", "boots", "knife", "grenade",
            ] {
                let item = |v: &Value| {
                    if slot.starts_with("weapon") {
                        v[slot]["item"].clone()
                    } else {
                        v[slot].clone()
                    }
                };
                if item(before) != item(after) {
                    actions.push(json!({"type":"equip","slot":slot,"itemId":item(after)}));
                }
            }
            let old_mods = if before["weapon1"]["item"] == after["weapon1"]["item"] {
                before["weapon1"]["mods"].clone()
            } else {
                Value::Null
            };
            for slot in ["optic", "muzzle", "rail", "mag", "stock"] {
                if old_mods[slot] != after["weapon1"]["mods"][slot] {
                    actions.push(
                        json!({"type":"mod","slot":slot,"itemId":after["weapon1"]["mods"][slot]}),
                    );
                }
            }
            if let Some(colors) = after["colors"].as_object() {
                for (item, color) in colors {
                    if &before["colors"][item] != color {
                        actions.push(json!({"type":"paint","itemId":item,"color":color}));
                    }
                }
            }
            Ok(json!(actions))
        }
        "client.online.moveCommands" => {
            let draft = &args[0];
            let n = (crate::num(&args[1]) + 0.5).floor().clamp(-2.0, 3.0);
            if n == crate::num(&draft["move"]) && draft["kind"] != "flee" {
                return Ok(json!([]));
            }
            let command = crate::duel::dispatch("client.moveCommand", json!([n]))?;
            Ok(
                if ["run", "dash"].contains(&crate::text(&draft["kind"]))
                    && command["type"] != "step"
                {
                    json!([{"type":"step","move":0},command])
                } else {
                    json!([command])
                },
            )
        }
        "client.online.advance" => {
            let old = &args[0];
            let new = &args[1];
            let same = old["id"] == new["id"] && !old.is_null();
            Ok(json!(
                if same && crate::num(&new["version"]) < crate::num(&old["version"]) {
                    "ignore"
                } else if !same || crate::truth(&args[2]) {
                    "snap"
                } else {
                    match (crate::num(&new["eventCursor"]) - crate::num(&old["eventCursor"])) as i64
                    {
                        0 => "update",
                        1 => "play",
                        _ => "snap",
                    }
                }
            ))
        }
        _ => Err(format!("Unknown online presentation: {method}")),
    }
}
