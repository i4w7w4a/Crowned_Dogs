//! Clip choreography emits renderer commands; AnimationMixer remains only playback I/O.
use serde_json::{Value, json};
fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}
fn n(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}
fn ready_name(stance: &str) -> &str {
    match stance {
        "rifle" => "ready_rifle",
        "shotgun" | "sniper" => "ready_shotgun",
        _ => "ready_pistol",
    }
}
fn spec(kind: &str) -> Value {
    match kind {
        "pistol" => json!({"clip":"fire_pistol","speed":1.15,"mark":0.625}),
        "rifle" => json!({"clip":"fire_rifle","speed":1,"mark":0.333}),
        "shotgun" => json!({"clip":"fire_shotgun","speed":1.25,"mark":0.333}),
        "sniper" => json!({"clip":"fire_shotgun","speed":1,"mark":0.333}),
        "knife" => json!({"clip":"attack_knife","speed":1.25,"mark":0.958,"hold":"knife"}),
        "grenade" => {
            json!({"clip":"grenade_throw","speed":1.4,"mark":1.208,"hold":"grenade","release":true,"end":1.875})
        }
        _ => Value::Null,
    }
}
fn ready(state: &Value) -> String {
    let name = ready_name(s(&state["stance"]));
    if state["durations"].get(name).is_some() {
        name.into()
    } else {
        "idle".into()
    }
}
fn to(
    state: &mut Value,
    commands: &mut Vec<Value>,
    layer: &str,
    name: &str,
    fade: f64,
    speed: f64,
    once: bool,
) -> bool {
    if state["durations"].get(name).is_none() {
        return false;
    }
    let reset = once || s(&state[layer]) != name;
    commands.push(json!({"op":"to","layer":layer,"name":name,"fade":fade,"speed":speed,"once":once,"reset":reset}));
    state[layer] = json!(name);
    true
}
fn sync(state: &Value, commands: &mut Vec<Value>) {
    if state["body"] == state["legs"] {
        commands.push(json!({"op":"sync"}));
    }
}
fn start(state: &mut Value, commands: &mut Vec<Value>, kind: &str, at: f64) {
    let a = spec(kind);
    let clip = s(&a["clip"]);
    if !to(state, commands, "body", clip, 0.12, n(&a["speed"]), true) {
        return;
    }
    commands.push(json!({"op":"time","layer":"body","value":at.min(n(&state["durations"][clip]))}));
    state["kind"] = json!(kind);
    state["released"] = json!(false);
    state["marked"] = json!(false);
    if !b(&state["walking"]) {
        to(state, commands, "legs", clip, 0.12, n(&a["speed"]), true);
        sync(state, commands);
    }
    if !a["hold"].is_null() {
        commands.push(json!({"op":"hold","value":a["hold"]}));
    }
}
fn done(state: &mut Value, commands: &mut Vec<Value>) {
    state["kind"] = Value::Null;
    let r = ready(state);
    to(state, commands, "body", &r, 0.35, 1.0, false);
    if !b(&state["walking"]) {
        to(state, commands, "legs", &r, 0.35, 1.0, false);
        sync(state, commands);
    }
    commands.push(json!({"op":"hold","value":state["stance"]}));
}
pub fn event(args: &Value) -> Value {
    let mut state = args["state"].clone();
    let event = s(&args["event"]);
    let mut commands = vec![];
    if event == "create" {
        state = json!({"stance":"pistol","walking":false,"pending":null,"fallen":false,"marked":false,"kind":null,"released":false,"body":null,"legs":null,"durations":args["durations"]});
    } else {
        match event {
            "arm" => {
                let stance = s(&args["stance"]);
                if stance != s(&state["stance"]) {
                    state["stance"] = json!(stance);
                    let r = ready(&state);
                    if state["kind"].is_null() {
                        to(&mut state, &mut commands, "body", &r, 0.3, 1.0, false);
                        if !b(&state["walking"]) {
                            to(&mut state, &mut commands, "legs", &r, 0.3, 1.0, false);
                        }
                    }
                    commands.push(json!({"op":"hold","value":stance}));
                }
            }
            "walk" => {
                let clip = s(&args["clip"]);
                let was = b(&state["walking"]) && s(&state["legs"]) == clip;
                state["walking"] = json!(true);
                if to(&mut state, &mut commands, "legs", clip, 0.15, 0.0, false) && !was {
                    let duration = n(&state["durations"][clip]);
                    commands.push(json!({"op":"time","layer":"legs","value":n(&args["at"]).rem_euclid(duration)}));
                }
            }
            "pace" => {
                if b(&state["walking"]) {
                    commands.push(json!({"op":"speed","layer":"legs","value":args["speed"]}));
                }
            }
            "stand" => {
                state["walking"] = json!(false);
                let clip = if state["kind"].is_null() {
                    ready(&state)
                } else {
                    s(&spec(s(&state["kind"]))["clip"]).into()
                };
                to(
                    &mut state,
                    &mut commands,
                    "legs",
                    &clip,
                    n(&args["fade"]),
                    1.0,
                    false,
                );
                sync(&state, &mut commands);
            }
            "attack" => {
                let kind = s(&args["kind"]);
                let a = spec(kind);
                if state["durations"].get(s(&a["clip"])).is_some() {
                    let wait = n(&args["inSeconds"]) - n(&a["mark"]) / n(&a["speed"]);
                    if wait > 0.0 {
                        state["pending"] = json!({"in":wait,"kind":kind});
                    } else {
                        start(&mut state, &mut commands, kind, -wait * n(&a["speed"]));
                    }
                }
            }
            "fall" => {
                state["pending"] = Value::Null;
                state["fallen"] = json!(true);
                for layer in ["body", "legs"] {
                    commands.push(json!({"op":"speed","layer":layer,"value":0.2}));
                }
            }
            "reset" => {
                state["pending"] = Value::Null;
                state["kind"] = Value::Null;
                state["walking"] = json!(false);
                state["fallen"] = json!(false);
                commands.push(json!({"op":"stopAll"}));
                let r = ready(&state);
                for layer in ["body", "legs"] {
                    state[layer] = Value::Null;
                    if to(&mut state, &mut commands, layer, &r, 0.0, 1.0, false) {
                        commands.push(json!({"op":"time","layer":layer,"value":n(&args["phase"])*n(&state["durations"][&r])}));
                    }
                }
                commands.push(json!({"op":"hold","value":state["stance"]}));
            }
            "update" => {
                if !state["pending"].is_null() {
                    state["pending"]["in"] = json!(n(&state["pending"]["in"]) - n(&args["dt"]));
                    if n(&state["pending"]["in"]) <= 0.0 {
                        let kind = s(&state["pending"]["kind"]).to_owned();
                        let at = -n(&state["pending"]["in"]) * n(&spec(&kind)["speed"]);
                        state["pending"] = Value::Null;
                        start(&mut state, &mut commands, &kind, at);
                    }
                }
                if !state["kind"].is_null() {
                    let a = spec(s(&state["kind"]));
                    let time = commands
                        .iter()
                        .rev()
                        .find(|c| s(&c["op"]) == "time" && s(&c["layer"]) == "body")
                        .map(|c| n(&c["value"]))
                        .unwrap_or(n(&args["time"]));
                    if !b(&state["marked"]) && time >= n(&a["mark"]) {
                        state["marked"] = json!(true);
                        commands.push(json!({"op":"mark","value":state["kind"]}));
                    }
                    if b(&a["release"]) && !b(&state["released"]) && time >= n(&a["mark"]) {
                        state["released"] = json!(true);
                        commands.push(json!({"op":"hold","value":null}));
                    }
                    if !a["end"].is_null() && time >= n(&a["end"]) {
                        done(&mut state, &mut commands);
                    }
                }
            }
            "done" => done(&mut state, &mut commands),
            _ => {}
        }
    }
    state["planted"] = if b(&state["walking"])
        || b(&state["fallen"])
        || (!state["kind"].is_null() && ["knife", "grenade"].contains(&s(&state["kind"])))
    {
        Value::Null
    } else {
        json!(ready_name(s(&state["stance"])))
    };
    json!({"state":state,"commands":commands})
}
