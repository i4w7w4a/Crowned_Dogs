use crate::{core, num, text};
use serde_json::{json, Value};

fn move_command(n: f64) -> Value {
    if n >= 3.0 {
        json!({"type":"dash"})
    } else if n.abs() >= 2.0 {
        json!({"type":"run","move":if n > 0.0 {2} else {-2}})
    } else {
        json!({"type":"step","move":if n > 0.0 {1} else if n < 0.0 {-1} else {0}})
    }
}
fn setup(id: &str, f: &Value) -> Result<Value, String> {
    let kit = f.get("kit").filter(|k| !k.is_null());
    let skills = f.get("skills").filter(|p| !p.is_null());
    if kit.is_none() && skills.is_none() {
        return Ok(json!({"id":id,"name":f["name"],"weapon":f["weapon"]}));
    }
    // Synchronisation sits on a compiled kit; without gear that is the starter kit, which plays as the plain rules.
    let loadout = match kit {
        Some(k) => k.clone(),
        None => core("starterLoadout", json!([f["weapon"]]))?,
    };
    let mut fighter = core("kitFighter", json!([id, f["name"], loadout]))?;
    if let Some(p) = skills {
        fighter["kit"] = core("syncApply", json!([fighter["kit"], p]))?;
    }
    Ok(fighter)
}
fn create(opts: &Value) -> Result<Value, String> {
    let seed = text(&opts["seed"]);
    // JS hashes UTF-16 code units, including both halves of astral characters.
    let hash = seed
        .encode_utf16()
        .fold(2166136261u32, |h, c| (h ^ c as u32).wrapping_mul(16777619));
    let d = game_core::constants();
    let pick = |name: &str, salt: u64| -> Value {
        let list = d[name].as_array().expect("rule list");
        list[(hash as u64 + salt) as usize % list.len()].clone()
    };
    let player = setup("me", &opts["player"])?;
    let bot = setup("bot", &opts["bot"])?;
    let mut fighters = vec![opts["player"].clone(), opts["bot"].clone()];
    fighters[0]["weapon"] = player["weapon"].clone();
    fighters[1]["weapon"] = bot["weapon"].clone();
    let mut config = json!({"seed":seed,"mapId":opts.get("mapId").unwrap_or(&json!("saltHall")),"weather":opts.get("weather").cloned().unwrap_or_else(||pick("WEATHERS",1)),"time":opts.get("time").cloned().unwrap_or_else(||pick("TIMES",2)),"fighters":[player,bot]});
    if num(&opts["startDistance"]) != 0.0 {
        config["startDistance"] = opts["startDistance"].clone();
    }
    let m = core("createMatch", json!([config]))?;
    let draft = core("freshDraft", json!([m, 0]))?;
    Ok(json!({"match":m,"draft":draft,"fighters":fighters}))
}
fn press(state: &mut Value, command: &Value) -> Result<Value, String> {
    let next = core(
        "applyCommand",
        json!([state["match"], 0, state["draft"], command]),
    )?;
    if next.is_string() {
        Ok(next)
    } else {
        state["draft"] = next;
        Ok(Value::Null)
    }
}
fn play(state: &mut Value, order: Value) -> Result<Value, String> {
    let bot = core("botOrder", json!([state["match"], 1]))?;
    let outcome = core("resolveTurn", json!([state["match"], [order, bot]]))?;
    state["match"] = outcome["match"].clone();
    state["draft"] = core("freshDraft", json!([state["match"], 0]))?;
    Ok(outcome["report"].clone())
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    if method == "client.moveCommand" {
        return Ok(move_command(num(&args[0])));
    }
    if method == "client.duel.create" {
        return create(&args[0]);
    }
    let mut state = args[0].clone();
    if method == "client.duel.query" {
        return match text(&args[1]) {
            "missing" => core("missing", json!([state["draft"]])),
            "chance" => core("draftChance", json!([state["match"], 0, state["draft"]])),
            "plannedDistance" => Ok(json!((num(&state["match"]["distance"])
                - num(&state["draft"]["move"]))
            .min(num(&core("distanceLimit", json!([state["match"]]))?))
            .max(1.0))),
            "spare" => core("swapWeapon", json!([state["match"], 0])),
            "over" => Ok(json!(!state["match"]["result"].is_null())),
            name => Err(format!("Unknown duel property: {name}")),
        };
    }
    let result = match method {
        "client.duel.press" => press(&mut state, &args[1])?,
        "client.duel.setMove" => {
            // JS Math.round rounds ties towards positive infinity.
            let target = (num(&args[1]) + 0.5).floor().clamp(-2.0, 3.0);
            if target == num(&state["draft"]["move"]) && state["draft"]["kind"] != "flee" {
                Value::Null
            } else {
                let command = move_command(target);
                if ["run", "dash"].contains(&text(&state["draft"]["kind"]))
                    && command["type"] != "step"
                {
                    press(&mut state, &json!({"type":"step","move":0}))?;
                }
                press(&mut state, &command)?
            }
        }
        "client.duel.submit" => {
            if !state["match"]["result"].is_null() {
                json!("Бой окончен")
            } else {
                let order = core("toOrder", json!([state["match"], 0, state["draft"]]))?;
                if order.is_string() {
                    order
                } else {
                    play(&mut state, order)?
                }
            }
        }
        "client.duel.timeout" => {
            let order = core("toOrder", json!([state["match"], 0, state["draft"]]))?;
            play(
                &mut state,
                if order.is_string() {
                    Value::Null
                } else {
                    order
                },
            )?
        }
        _ => return Err(format!("Unknown duel operation: {method}")),
    };
    Ok(json!({"state":state,"result":result}))
}
