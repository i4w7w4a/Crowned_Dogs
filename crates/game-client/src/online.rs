//! Durable PvP policy. The host supplies trusted accounts, entropy and time; it
//! commits this private state atomically and exposes only `project` to clients.
use crate::{core, num, text};
use serde_json::{json, Value};

const RULESET: &str = "online-v1";
fn pick(v: &Value, keys: &[&str]) -> Value {
    let mut out = json!({});
    for k in keys {
        if let Some(value) = v.get(k) {
            out[*k] = value.clone();
        }
    }
    out
}
fn check(s: &Value) -> Result<(), String> {
    if s["schemaVersion"] != 1 || s["ruleset"] != RULESET {
        return Err("incompatible_ruleset".into());
    }
    Ok(())
}
fn side(s: &Value, id: &str) -> Result<usize, String> {
    (0..2)
        .find(|i| s["players"][*i]["id"] == id)
        .ok_or("not_found".into())
}
fn player(p: &Value) -> Value {
    let callsign = text(&p["appearance"]["callsign"]);
    json!({"id":p["id"],"tag":p["tag"],"name":if callsign.is_empty(){text(&p["telegram"]["firstName"])}else{callsign},
        "telegram":pick(&p["telegram"],&["firstName","lastName","username"]),"level":p["profile"]["level"],
        "appearance":crate::look::normalize(&p["appearance"]),"loadout":p["loadout"]})
}
fn create(opts: &Value) -> Result<Value, String> {
    if opts["players"].as_array().is_none_or(|p| p.len() != 2)
        || opts["players"][0]["id"] == opts["players"][1]["id"]
    {
        return Err("invalid_players".into());
    }
    let mut fighters = vec![];
    for p in opts["players"].as_array().unwrap() {
        if crate::loadout::dispatch("client.isValidLoadout", json!([p["loadout"]]))? != true {
            return Err("invalid_loadout".into());
        }
        let mut f = core(
            "kitFighter",
            json!([p["id"], player(p)["name"], p["loadout"]]),
        )?;
        f["kit"] = core("syncApply", json!([f["kit"], p["profile"]]))?;
        fighters.push(f);
    }
    let setting = if opts["setting"].is_object() {
        opts["setting"].clone()
    } else {
        let world = crate::world::at(num(&opts["now"]), "saltHall");
        json!({"mapId":"saltHall","weather":world["weather"],"time":world["time"],"events":true})
    };
    let mut config = pick(&setting, &["mapId", "weather", "time", "events", "shelled"]);
    config["fighters"] = json!(fighters);
    config["seed"] = opts["seed"].clone();
    let m = core("createMatch", json!([config]))?;
    Ok(
        json!({"schemaVersion":1,"ruleset":RULESET,"id":opts["id"],"version":1,"phase":"loading",
        "players":opts["players"],"ready":[false,false],"loadingDeadline":num(&opts["now"])+300000.0,
        "deadline":null,"match":m,"drafts":[core("freshDraft",json!([m,0]))?,core("freshDraft",json!([m,1]))?],
        "orders":[null,null],"cancelReason":null}),
    )
}
fn arm(s: &mut Value, now: f64) -> Result<(), String> {
    s["deadline"] = json!(now + num(&core("turnSeconds", json!([s["match"]]))?) * 1000.0);
    Ok(())
}
fn advance(s: &mut Value, now: f64) -> Result<(), String> {
    let resolved = core("resolveTurn", json!([s["match"], s["orders"]]))?;
    s["match"] = resolved["match"].clone();
    s["orders"] = json!([null, null]);
    s["drafts"] = json!([
        core("freshDraft", json!([s["match"], 0]))?,
        core("freshDraft", json!([s["match"], 1]))?
    ]);
    if s["match"]["result"].is_null() {
        arm(s, now)?;
    } else {
        s["phase"] = json!("finished");
        s["deadline"] = Value::Null;
    }
    Ok(())
}
fn due(s: &Value, now: f64) -> bool {
    let key = if s["phase"] == "loading" {
        "loadingDeadline"
    } else if s["phase"] == "active" {
        "deadline"
    } else {
        return false;
    };
    now >= num(&s[key])
}
fn bump(s: &mut Value) {
    s["version"] = json!(s["version"].as_u64().unwrap() + 1);
}
fn timeout(mut s: Value, now: f64) -> Result<Value, String> {
    check(&s)?;
    if !due(&s, now) {
        return Err("not_due".into());
    }
    if s["phase"] == "loading" {
        s["phase"] = json!("cancelled");
        s["cancelReason"] = json!("loading_timeout");
    } else {
        advance(&mut s, now)?;
    }
    bump(&mut s);
    Ok(s)
}
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn command(c: &Value) -> bool {
    match text(&c["type"]) {
        "zone" | "guard" => {
            exact(c, &["type", "zone"])
                && ["head", "body", "groin", "legs"].contains(&text(&c["zone"]))
        }
        "step" => {
            exact(c, &["type", "move"]) && c["move"].as_i64().is_some_and(|n| (-1..=1).contains(&n))
        }
        "run" => {
            exact(c, &["type", "move"]) && c["move"].as_i64().is_some_and(|n| n == 2 || n == -2)
        }
        "dash" | "knife" | "grenade" | "swap" | "flee" => exact(c, &["type"]),
        _ => false,
    }
}
fn transition(mut s: Value, id: &str, e: &Value, now: f64) -> Result<Value, String> {
    check(&s)?;
    let i = side(&s, id)?;
    if due(&s, now) {
        return Err("deadline_passed".into());
    }
    if e["expectedVersion"] != s["version"] {
        return Err("stale_version".into());
    }
    if !e["turn"].is_number() || num(&e["turn"]) != num(&s["match"]["turn"]) {
        return Err("stale_turn".into());
    }
    let a = &e["action"];
    match text(&a["type"]) {
        "ready" if s["phase"] == "loading" && exact(a, &["type"]) => {
            s["ready"][i] = json!(true);
            if s["ready"] == json!([true, true]) {
                s["phase"] = json!("active");
                arm(&mut s, now)?;
            }
        }
        "press" | "submit" if s["phase"] == "active" => {
            if !s["orders"][i].is_null() {
                return Err("already_submitted".into());
            }
            if a["type"] == "press" {
                if !exact(a, &["type", "command"]) || !command(&a["command"]) {
                    return Err("invalid_command".into());
                }
                let d = core(
                    "applyCommand",
                    json!([s["match"], i, s["drafts"][i], a["command"]]),
                )?;
                if d.is_string() {
                    return Err("invalid_command".into());
                }
                s["drafts"][i] = d;
            } else {
                if !exact(a, &["type"]) {
                    return Err("invalid_command".into());
                }
                let o = core("toOrder", json!([s["match"], i, s["drafts"][i]]))?;
                if o.is_string() || o.is_null() {
                    return Err("incomplete_order".into());
                }
                s["orders"][i] = o;
                if !s["orders"][0].is_null() && !s["orders"][1].is_null() {
                    advance(&mut s, now)?;
                }
            }
        }
        "surrender" if s["phase"] == "active" && exact(a, &["type"]) => {
            s["phase"] = json!("finished");
            s["deadline"] = Value::Null;
            s["match"]["result"] = json!({"winner":1-i,"reason":"surrender"});
            s["orders"] = json!([null, null]);
        }
        _ => return Err("invalid_command".into()),
    }
    bump(&mut s);
    Ok(s)
}
fn project(s: &Value, id: &str) -> Result<Value, String> {
    check(s)?;
    let i = side(s, id)?;
    let mut m = pick(
        &s["match"],
        &[
            "version", "map", "turn", "distance", "fighters", "result", "history", "lane", "hazard",
        ],
    );
    m["config"] = pick(
        &s["match"]["config"],
        &["mapId", "weather", "time", "events", "shelled", "fighters"],
    );
    let mut out = pick(
        s,
        &[
            "schemaVersion",
            "ruleset",
            "id",
            "version",
            "phase",
            "ready",
            "loadingDeadline",
            "deadline",
            "cancelReason",
        ],
    );
    out["side"] = json!(i);
    out["players"] = json!([player(&s["players"][0]), player(&s["players"][1])]);
    out["eventCursor"] = json!(m["history"].as_array().map_or(0, Vec::len));
    out["result"] = m["result"].clone();
    out["match"] = m;
    out["own"] = json!({"draft":s["drafts"][i],"order":s["orders"][i],"submitted":!s["orders"][i].is_null()});
    out["submitted"] = json!([!s["orders"][0].is_null(), !s["orders"][1].is_null()]);
    out["reward"] = Value::Null;
    Ok(out)
}
fn profile(mut p: Value, a: &Value) -> Result<Value, String> {
    match text(&a["type"]) {
        "spend" if exact(a, &["type", "stat"]) => {
            if !["power", "cooling", "instinct", "reflexes"].contains(&text(&a["stat"]))
                || num(&p["profile"]["free"]) <= 0.0
            {
                return Err("invalid_command".into());
            }
            p["profile"] = core("syncSpend", json!([p["profile"], a["stat"]]))?;
        }
        "appearance" if exact(a, &["type", "value"]) => {
            let mut v = a["value"].clone();
            if let Some(tag) = v.get("tag") {
                if tag != &p["tag"] {
                    return Err("invalid_command".into());
                }
            }
            v["tag"] = p["tag"].clone();
            if !crate::look::valid(&v) {
                return Err("invalid_command".into());
            }
            p["appearance"] = v;
        }
        "equip" if exact(a, &["type", "slot", "itemId"]) => {
            p["loadout"] = crate::loadout::dispatch(
                "client.equipItem",
                json!([p["loadout"], a["slot"], a["itemId"]]),
            )
            .map_err(|_| "invalid_command")?
        }
        "mod" if exact(a, &["type", "slot", "itemId"]) => {
            p["loadout"] = crate::loadout::dispatch(
                "client.equipWeaponMod",
                json!([p["loadout"], a["slot"], a["itemId"]]),
            )
            .map_err(|_| "invalid_command")?
        }
        "paint" if exact(a, &["type", "itemId", "color"]) => {
            p["loadout"] = crate::loadout::dispatch(
                "client.paintItem",
                json!([p["loadout"], a["itemId"], a["color"]]),
            )
            .map_err(|_| "invalid_command")?
        }
        _ => return Err("invalid_command".into()),
    }
    Ok(p)
}
fn invite(inv: &Value, id: &str, action: &str, version: &Value, now: f64) -> Result<Value, String> {
    if inv["status"] != "open" {
        return Err("invite_closed".into());
    }
    if inv["version"] != *version {
        return Err("stale_version".into());
    }
    if now >= num(&inv["expiresAt"]) {
        return if action == "expire" {
            Ok(json!("expired"))
        } else {
            Err("deadline_passed".into())
        };
    }
    match action {
        "accept" if inv["hostId"] != id => Ok(json!("accepted")),
        "cancel" if inv["hostId"] == id => Ok(json!("cancelled")),
        "retry" if inv["hostId"] == id => Ok(json!("open")),
        _ => Err("invalid_command".into()),
    }
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    match method {
        "server.online.create" => create(&args[0]),
        "server.online.transition" => {
            transition(args[0].clone(), text(&args[1]), &args[2], num(&args[3]))
        }
        "server.online.timeout" => timeout(args[0].clone(), num(&args[1])),
        "server.online.project" => project(&args[0], text(&args[1])),
        "server.online.player" => Ok(player(&args[0])),
        "server.online.profile" => profile(args[0].clone(), &args[1]),
        "server.online.invite" => invite(
            &args[0],
            text(&args[1]),
            text(&args[2]),
            &args[3],
            num(&args[4]),
        ),
        "server.online.rewards" => {
            check(&args[0])?;
            if args[0]["phase"] != "finished" || args[0]["match"]["result"].is_null() {
                return Err("not_finished".into());
            }
            Ok(json!([
                core(
                    "syncGain",
                    json!([args[0]["players"][0]["profile"], args[0]["match"], 0])
                )?,
                core(
                    "syncGain",
                    json!([args[0]["players"][1]["profile"], args[0]["match"], 1])
                )?
            ]))
        }
        _ => Err("invalid_command".into()),
    }
}
