//! Matchmaking, invitation and operator-registry policy. Host supplies entropy,
//! time, storage and event delivery; replaying a request for entropy has no effects.
use crate::{core, num, text, truth};
use game_core::rng;
use serde_json::{json, Value};

const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const BOT_NAMES: [&str; 5] = ["Искра", "Сварка", "Глушь", "Ржавый", "Соль"];
fn fresh() -> Value {
    json!({"duels":{},"byPlayer":{},"invites":{}})
}
fn side(duel: &Value, id: &str) -> Option<usize> {
    (0..2).find(|i| text(&duel["players"][*i]["id"]) == id)
}
fn free(state: &Value, id: &str) -> Result<(), String> {
    if state["byPlayer"].get(id).is_some() {
        Err("Ты уже в бою. Сначала закончи его".into())
    } else {
        Ok(())
    }
}
fn remove(object: &mut Value, key: &str) {
    if let Some(o) = object.as_object_mut() {
        o.remove(key);
    }
}
fn locate(state: &Value, id: &str, duel_id: &str, turn: &Value) -> Result<(Value, usize), String> {
    let duel = state["duels"]
        .get(duel_id)
        .filter(|d| d["match"]["result"].is_null())
        .ok_or("Этот бой уже закончился")?;
    let side = side(duel, id).ok_or("Это не твой бой")?;
    if num(&duel["match"]["turn"]) != num(turn) {
        return Err("Этот ход уже прошёл".into());
    }
    Ok((duel.clone(), side))
}
fn invite(state: &mut Value, code: &str, now: f64) -> Value {
    let inv = state["invites"][code].clone();
    if !inv.is_null() && num(&inv["expires"]) < now {
        remove(&mut state["invites"], code);
        Value::Null
    } else {
        inv
    }
}
struct Request {
    seeds: Vec<Value>,
    cursor: usize,
    now: f64,
    setting: Value,
    events: Vec<Value>,
}
impl Request {
    fn seed(&mut self) -> Result<String, String> {
        let i = self.cursor;
        self.cursor += 1;
        self.seeds
            .get(i)
            .map(|v| text(v).to_owned())
            .ok_or_else(|| format!("__need_seed:{i}"))
    }
    fn unique(&mut self, taken: &Value, lower: bool) -> Result<String, String> {
        let mut rng = rng::seed(&self.seed()?);
        loop {
            let s: String = (0..6)
                .map(|_| {
                    ALPHABET[(rng::next(&mut rng) * ALPHABET.len() as f64).floor() as usize] as char
                })
                .collect();
            let id = if lower { s.to_lowercase() } else { s };
            if taken.get(&id).is_none() {
                return Ok(id);
            }
        }
    }
    fn arm(&self, duel: &mut Value) -> Result<(), String> {
        duel["deadline"] =
            json!(self.now + num(&core("turnSeconds", json!([duel["match"]]))?) * 1000.0);
        Ok(())
    }
    fn start(&mut self, state: &mut Value, a: Value, b: Value) -> Result<Value, String> {
        let seed = self.seed()?;
        let mut random = rng::seed(&format!("{seed}:setting"));
        let constants = game_core::constants();
        let setting = if self.setting.is_null() {
            let maps = constants["MAPS"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            let map = &maps[(rng::next(&mut random) * maps.len() as f64).floor() as usize];
            let weathers = constants["WEATHERS"].as_array().unwrap();
            let odds = &constants["REGION_EVENTS"]["calm"]["odds"];
            let mut roll =
                rng::next(&mut random) * weathers.iter().map(|w| num(&odds[text(w)])).sum::<f64>();
            let mut weather = weathers.last().unwrap().clone();
            for w in weathers {
                roll -= num(&odds[text(w)]);
                if roll < 0.0 {
                    weather = w.clone();
                    break;
                }
            }
            let times = constants["TIMES"].as_array().unwrap();
            let time = &times[(rng::next(&mut random) * times.len() as f64).floor() as usize];
            json!({"mapId":map,"weather":weather,"time":time})
        } else {
            self.setting.clone()
        };
        let fighter = |p: &Value| -> Result<Value, String> {
            let mut f = json!({"id":p["id"],"name":p["name"],"weapon":p["weapon"]});
            if p.get("kit").is_some_and(|k| !k.is_null()) {
                f["kit"] = core("compileKit", json!([p["kit"]]))?;
            }
            Ok(f)
        };
        let mut config = setting;
        config["seed"] = json!(seed);
        config["fighters"] = json!([fighter(&a)?, fighter(&b)?]);
        let m = core("createMatch", json!([config]))?;
        let id = self.unique(&state["duels"], true)?;
        let mut duel = json!({"id":id,"players":[a,b],"match":m,"drafts":[core("freshDraft",json!([m,0]))?,core("freshDraft",json!([m,1]))?],"orders":[null,null],"deadline":0});
        self.arm(&mut duel)?;
        state["duels"][&id] = duel.clone();
        for i in 0..2 {
            if !truth(&duel["players"][i]["bot"]) {
                state["byPlayer"][text(&duel["players"][i]["id"])] = json!(id);
            }
        }
        self.events.push(json!({"type":"start","duel":duel}));
        Ok(duel)
    }
    fn play(&mut self, state: &mut Value, mut duel: Value) -> Result<Value, String> {
        let mut orders = duel["orders"].clone();
        for i in 0..2 {
            if truth(&duel["players"][i]["bot"]) {
                orders[i] = core("botOrder", json!([duel["match"], i]))?;
            }
        }
        let result = core("resolveTurn", json!([duel["match"], orders]))?;
        duel["match"] = result["match"].clone();
        duel["orders"] = json!([null, null]);
        duel["drafts"] = json!([
            core("freshDraft", json!([duel["match"], 0]))?,
            core("freshDraft", json!([duel["match"], 1]))?
        ]);
        let id = text(&duel["id"]).to_owned();
        if !duel["match"]["result"].is_null() {
            remove(&mut state["duels"], &id);
            for i in 0..2 {
                let p = text(&duel["players"][i]["id"]);
                if state["byPlayer"][p] == id {
                    remove(&mut state["byPlayer"], p);
                }
            }
        } else {
            self.arm(&mut duel)?;
            state["duels"][&id] = duel.clone();
        }
        self.events
            .push(json!({"type":"turn","duel":duel,"report":result["report"]}));
        Ok(duel)
    }
    fn act(&mut self, state: &mut Value, method: &str, args: &Value) -> Result<Value, String> {
        match method {
            "duelOf" => {
                let id = text(&state["byPlayer"][text(&args[0])]);
                Ok(state["duels"][id].clone())
            }
            "get" => Ok(state["duels"][text(&args[0])].clone()),
            "inviteInfo" => Ok(invite(state, text(&args[0]), self.now)),
            "startVsBot" => {
                let mut player = args[0].clone();
                free(state, text(&player["id"]))?;
                let mut rng = rng::seed(&self.seed()?);
                let constants = game_core::constants();
                let weapons = constants["WEAPON_IDS"].as_array().unwrap();
                let weapon =
                    weapons[(rng::next(&mut rng) * weapons.len() as f64).floor() as usize].clone();
                let id = format!("bot:{}", (rng::next(&mut rng) * 1e9).floor());
                let name =
                    BOT_NAMES[(rng::next(&mut rng) * BOT_NAMES.len() as f64).floor() as usize];
                let mut bot = json!({"id":id,"name":name,"weapon":weapon,"bot":true});
                if player.get("kit").is_some_and(|v| !v.is_null()) {
                    bot["kit"] = constants["VETERAN_KITS"][text(&weapon)].clone();
                }
                player["bot"] = json!(false);
                self.start(state, player, bot)
            }
            "createInvite" => {
                let mut host = args[0].clone();
                free(state, text(&host["id"]))?;
                state["invites"]
                    .as_object_mut()
                    .unwrap()
                    .retain(|_, inv| inv["host"]["id"] != host["id"]);
                let code = self.unique(&state["invites"], false)?;
                host["bot"] = json!(false);
                let inv = json!({"code":code,"host":host,"expires":self.now+1800000.0});
                state["invites"][&code] = inv.clone();
                Ok(inv)
            }
            "acceptInvite" => {
                let code = text(&args[0]);
                let inv = invite(state, code, self.now);
                if inv.is_null() {
                    return Err("Приглашение не найдено или устарело".into());
                }
                let mut guest = args[1].clone();
                if inv["host"]["id"] == guest["id"] {
                    return Err("Это твоё приглашение. Перешли ссылку другу".into());
                }
                free(state, text(&guest["id"]))?;
                if state["byPlayer"].get(text(&inv["host"]["id"])).is_some() {
                    return Err("Соперник уже в другом бою".into());
                }
                remove(&mut state["invites"], code);
                guest["bot"] = json!(false);
                self.start(state, inv["host"].clone(), guest)
            }
            "press" | "submit" => {
                let id = text(&args[1]);
                let (mut duel, side) = locate(state, text(&args[0]), id, &args[2])?;
                if !duel["orders"][side].is_null() {
                    return Err("Приказ уже отдан, ждём соперника".into());
                }
                if method == "press" {
                    let next = core(
                        "applyCommand",
                        json!([duel["match"], side, duel["drafts"][side], args[3]]),
                    )?;
                    if let Some(error) = next.as_str() {
                        return Err(error.into());
                    }
                    duel["drafts"][side] = next;
                } else {
                    let order = core(
                        "toOrder",
                        json!([duel["match"], side, duel["drafts"][side]]),
                    )?;
                    if let Some(error) = order.as_str() {
                        return Err(error.into());
                    }
                    duel["orders"][side] = order;
                    if !(0..2)
                        .any(|i| !truth(&duel["players"][i]["bot"]) && duel["orders"][i].is_null())
                    {
                        return self.play(state, duel);
                    }
                }
                state["duels"][id] = duel.clone();
                Ok(duel)
            }
            "timeout" => {
                let duel = state["duels"][text(&args[0])].clone();
                if duel.is_null()
                    || !duel["match"]["result"].is_null()
                    || num(&duel["match"]["turn"]) != num(&args[1])
                {
                    Ok(Value::Null)
                } else {
                    self.play(state, duel)
                }
            }
            _ => Err(format!("Unknown duel service operation: {method}")),
        }
    }
}
fn encode(c: &Value) -> String {
    let kind = text(&c["type"]);
    match kind {
        "zone" | "guard" => format!(
            "{}{}",
            if kind == "zone" { 'z' } else { 'g' },
            text(&c["zone"]).chars().next().unwrap_or(' ')
        ),
        "step" | "run" => format!(
            "{}{}",
            if kind == "step" { 's' } else { 'r' },
            num(&c["move"])
        ),
        "dash" => "d".into(),
        "knife" => "k".into(),
        "grenade" => "n".into(),
        "swap" => "w".into(),
        "flee" => "f".into(),
        _ => String::new(),
    }
}
fn decode(s: &str) -> Value {
    let mut chars = s.chars();
    let first = chars.next().unwrap_or(' ');
    let rest = chars.as_str();
    match first {
        'z' | 'g' => {
            let zone = match rest {
                "h" => "head",
                "b" => "body",
                "g" => "groin",
                "l" => "legs",
                _ => return Value::Null,
            };
            json!({"type":if first=='z'{"zone"}else{"guard"},"zone":zone})
        }
        's' if ["-1", "0", "1"].contains(&rest) => {
            json!({"type":"step","move":rest.parse::<i32>().unwrap()})
        }
        'r' if ["-2", "2"].contains(&rest) => {
            json!({"type":"run","move":rest.parse::<i32>().unwrap()})
        }
        'd' | 'k' | 'n' | 'w' | 'f' if rest.is_empty() => {
            json!({"type":match first{'d'=>"dash",'k'=>"knife",'n'=>"grenade",'w'=>"swap",_=>"flee"}})
        }
        _ => Value::Null,
    }
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "server.encodeCommand" => json!(encode(&args[0])),
        "server.decodeCommand" => decode(text(&args[0])),
        "server.sideOf" => json!(side(&args[0], text(&args[1]))),
        "server.isReady" => json!(!args[0]["orders"][num(&args[1]) as usize].is_null()),
        "server.duels.new" => fresh(),
        "server.duels.step" => {
            let mut state = args[0].clone();
            let mut request = Request {
                seeds: args[4].as_array().cloned().unwrap_or_default(),
                cursor: 0,
                now: num(&args[3]),
                setting: args[5].clone(),
                events: vec![],
            };
            match request.act(&mut state, text(&args[1]), &args[2]) {
                Ok(value) => json!({"state":state,"value":value,"events":request.events}),
                Err(error) if error.starts_with("__need_seed:") => json!({"needsSeed":true}),
                Err(error) => json!({"state":state,"error":error,"events":[]}),
            }
        }
        "server.operators.sign" => {
            let list = args[0].as_array().ok_or("Invalid operator registry")?;
            let id = &args[1];
            if let Some(known) = list.iter().find(|op| &op["telegramId"] == id) {
                return Ok(json!({"operator":known,"operators":list,"existing":true}));
            }
            if list.len() >= 1_000_000 {
                return Err("all tags are taken".into());
            }
            let candidates = args[4].as_array().ok_or("Invalid tag candidates")?;
            let mut chosen = None;
            for pick in candidates {
                let tag = core("formatTag", json!([pick]))?;
                if !list.iter().any(|op| op["tag"] == tag) {
                    chosen = Some(tag);
                    break;
                }
            }
            if let Some(tag) = chosen {
                let op = json!({"tag":tag,"telegramId":id,"name":args[2],"signedAt":args[3]});
                let mut list = list.clone();
                list.push(op.clone());
                json!({"operator":op,"operators":list,"existing":false})
            } else {
                json!({"needsPick":true})
            }
        }
        _ => return Err(format!("Unknown server operation: {method}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_turn_cannot_activate_a_second_round() {
        let mut r = Request {
            seeds: vec![json!("a"), json!("b"), json!("c")],
            cursor: 0,
            now: 1000.0,
            setting: json!({"mapId":"saltHall","weather":"clear","time":"day"}),
            events: vec![],
        };
        let mut state = fresh();
        let d = r
            .act(
                &mut state,
                "startVsBot",
                &json!([{"id":"u","name":"Тень","weapon":"rifle"}]),
            )
            .unwrap();
        let first = r.act(&mut state, "timeout", &json!([d["id"], 1])).unwrap();
        assert_eq!(first["match"]["turn"], 2.0);
        assert!(r
            .act(&mut state, "timeout", &json!([d["id"], 1]))
            .unwrap()
            .is_null());
    }
    #[test]
    fn malformed_callbacks_are_rejected() {
        for s in ["", "z", "ghh", "s10", "r3", "do", "wx"] {
            assert!(decode(s).is_null());
        }
    }
}
