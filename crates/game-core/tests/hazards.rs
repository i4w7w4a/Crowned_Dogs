use game_core::dispatch;
use serde_json::{json, Value};
use std::collections::HashMap;

fn config(map: &str, weather: &str, time: &str, seed: &str) -> Value {
    json!({"seed":seed,"mapId":map,"weather":weather,"time":time,"events":true,
        "fighters":[{"id":"a","name":"A","weapon":"rifle"},{"id":"b","name":"B","weapon":"shotgun"}]})
}

/// Bots play whole matches; returns every turn's report.
fn play(cfg: Value) -> Vec<Value> {
    let mut m = dispatch("createMatch", json!([cfg])).unwrap();
    let mut reports = vec![];
    while m["result"].is_null() {
        let orders = json!([
            dispatch("botOrder", json!([m, 0])).unwrap(),
            dispatch("botOrder", json!([m, 1])).unwrap()
        ]);
        let out = dispatch("resolveTurn", json!([m, orders])).unwrap();
        m = out["match"].clone();
        reports.push(out["report"].clone());
    }
    reports
}

fn kinds(map: &str, weather: &str, shelled: bool) -> HashMap<String, usize> {
    let mut seen = HashMap::new();
    for i in 0..25 {
        let mut cfg = config(map, weather, "day", &format!("hz{i}"));
        cfg["shelled"] = json!(shelled);
        for r in play(cfg) {
            if let Some(k) = r["hazard"]["kind"].as_str() {
                *seen.entry(k.to_string()).or_default() += 1;
            }
        }
    }
    seen
}

#[test]
fn storms_bring_lightning_outside_and_collapses_and_blackouts_under_a_roof() {
    let open = kinds("dam", "storm", false);
    assert!(open["lightning"] > 10, "{open:?}");
    assert!(open.contains_key("drone"), "{open:?}");
    assert!(!open.contains_key("collapse") && !open.contains_key("blackout"));
    let hall = kinds("saltHall", "storm", false);
    assert!(!hall.contains_key("lightning"), "{hall:?}");
    assert!(hall["collapse"] > 3 && hall["blackout"] > 3, "{hall:?}");
}

#[test]
fn no_drones_in_a_dust_storm_and_shelling_follows_the_front() {
    let dust = kinds("blocks", "dust", false);
    assert!(!dust.contains_key("drone"), "{dust:?}");
    let quiet = kinds("blocks", "clear", false).get("shell").copied().unwrap_or(0);
    let front = kinds("blocks", "clear", true).get("shell").copied().unwrap_or(0);
    assert!(front > quiet * 3, "{quiet} vs {front}");
}

#[test]
fn a_hazard_is_warned_a_turn_ahead_and_strikes_only_its_stretch() {
    let mut warned = 0;
    for i in 0..40 {
        let mut m = dispatch("createMatch", json!([config("dam", "storm", "day", &format!("w{i}"))])).unwrap();
        assert_eq!(m["lane"], json!([0.0, 7.0]));
        while m["result"].is_null() {
            let orders = json!([
                dispatch("botOrder", json!([m, 0])).unwrap(),
                dispatch("botOrder", json!([m, 1])).unwrap()
            ]);
            let warning = m["hazard"].clone();
            let out = dispatch("resolveTurn", json!([m.clone(), orders])).unwrap();
            let r = &out["report"];
            let lane = &r["lane"];
            let gap = lane[1].as_f64().unwrap() - lane[0].as_f64().unwrap();
            assert!((gap - r["distanceAfter"].as_f64().unwrap()).abs() < 1e-9, "the lane keeps the range");
            if warning["turn"] == m["turn"] {
                warned += 1;
                assert_eq!(r["hazard"]["kind"], warning["kind"]);
                if let Some(at) = warning["at"].as_f64() {
                    let reach = warning["radius"].as_f64().unwrap() + 0.5;
                    for side in 0..2 {
                        let x = lane[side].as_f64().unwrap();
                        let hit = r["hazard"]["hits"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|h| h["side"] == side);
                        assert_eq!(hit, (x - at).abs() < reach, "turn {} side {side}", m["turn"]);
                    }
                }
            } else {
                assert!(r["hazard"].is_null() || r["hazard"]["kind"].is_null());
            }
            m = out["match"].clone();
        }
    }
    assert!(warned > 20, "{warned}");
}

#[test]
fn lightning_stuns_for_a_turn_and_takes_the_shield() {
    let mut cfg = config("dam", "storm", "day", "stun");
    cfg["startDistance"] = json!(5);
    let mut m = dispatch("createMatch", json!([cfg])).unwrap();
    m["turn"] = json!(3);
    m["hazard"] = json!({"kind":"lightning","name":"молния","turn":3,"at":0.0,"radius":1});
    let stand = json!({"action":{"type":"shoot","move":0,"zone":"body"},"defend":["head","body"]});
    let out = dispatch("resolveTurn", json!([m, [stand, stand]])).unwrap();
    let hits = out["report"]["hazard"]["hits"].as_array().unwrap().clone();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["side"], 0);
    let m = out["match"].clone();
    assert_eq!(m["fighters"][0]["stunned"], 1);
    assert_eq!(m["fighters"][0]["shieldOff"], 1.0);
    // Next turn the stunned side stands with its old guard, and that is not going missing.
    let run = json!({"action":{"type":"run","move":2},"defend":["legs","groin"]});
    let out = dispatch("resolveTurn", json!([m, [run, stand]])).unwrap();
    assert_eq!(out["report"]["moves"][0], 0.0);
    assert_eq!(out["report"]["afk"][0], false);
    assert_eq!(out["report"]["defend"][0], json!(["head", "body"]));
    assert_eq!(out["match"]["fighters"][0]["stunned"], 0);
    assert_eq!(out["match"]["fighters"][0]["afkStreak"], 0.0);
}

#[test]
fn the_drone_dives_on_whoever_stands_still() {
    let mut m = dispatch("createMatch", json!([config("blocks", "clear", "day", "drone")])).unwrap();
    m["turn"] = json!(2);
    m["hazard"] = json!({"kind":"drone","name":"дрон-камикадзе","turn":2,"at":null,"radius":null});
    let stand = json!({"action":{"type":"shoot","move":0,"zone":"body"},"defend":["head","body"]});
    let step = json!({"action":{"type":"shoot","move":1,"zone":"body"},"defend":["head","body"]});
    let out = dispatch("resolveTurn", json!([m, [step, stand]])).unwrap();
    let hits = out["report"]["hazard"]["hits"].as_array().unwrap().clone();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["side"], 1);
    assert_eq!(hits[0]["damage"], 16.0);
}

#[test]
fn roofs_change_the_weather_and_the_hour() {
    let chance = |map: &str, weather: &str, time: &str, events: bool| {
        let mut cfg = config(map, weather, time, "roof");
        cfg["events"] = json!(events);
        let m = dispatch("createMatch", json!([cfg])).unwrap();
        dispatch("conditionAccuracy", json!([m, 0, 10])).unwrap().as_f64().unwrap()
    };
    // Rain penalises the open dam, not the salt hall's roof.
    assert!(chance("dam", "storm", "morning", true) < 1.0);
    assert_eq!(chance("saltHall", "storm", "morning", true), 1.0);
    // Heat shimmer is outside only; without events the hall keeps the plain rules.
    assert!(chance("dam", "clear", "day", true) < 1.0);
    assert_eq!(chance("saltHall", "clear", "day", true), 1.0);
    assert!(chance("saltHall", "clear", "day", false) < 1.0);
    // Each map's own dust: the dry bed's salt storm is the worst.
    assert!(chance("dryBed", "dust", "morning", true) < chance("blocks", "dust", "morning", true));
}
