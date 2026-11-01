use game_core::dispatch;
use serde_json::{json, Value};

fn call(method: &str, args: Value) -> Value {
    dispatch(method, args).unwrap_or_else(|error| panic!("{method}: {error}"))
}

fn duel(time: &str, seed: &str) -> Value {
    call(
        "createMatch",
        json!([{
            "seed":seed,"mapId":"saltHall","weather":"clear","time":time,
            "fighters":[
                {"id":"a","name":"Тень","weapon":"rifle"},
                {"id":"b","name":"Искра","weapon":"rifle"}
            ]
        }]),
    )
}

fn number(value: Value) -> f64 {
    value.as_f64().unwrap()
}

fn accuracy(m: &Value, side: usize, distance: u32) -> f64 {
    number(call("conditionAccuracy", json!([m, side, distance])))
}

fn chance(m: &Value, side: usize, distance: u32) -> f64 {
    number(call(
        "hitChance",
        json!([m, side, "rifle", distance, 2, false]),
    ))
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
}

fn shot() -> Value {
    json!({"action":{"type":"shoot","move":0,"zone":"body"},"defend":["head","body"]})
}

fn move_back() -> Value {
    json!({"action":{"type":"run","move":-2},"defend":["head","body"]})
}

#[test]
fn four_times_use_the_designed_distance_boundaries_and_keep_weather() {
    let morning = duel("morning", "time-boundaries");
    let day = duel("day", "time-boundaries");
    let night = duel("night", "time-boundaries");
    for distance in 1..=16 {
        assert_eq!(accuracy(&morning, 0, distance), 1.0);
        assert_eq!(
            accuracy(&day, 0, distance),
            if distance > 8 { 0.9 } else { 1.0 }
        );
        assert_eq!(
            accuracy(&night, 0, distance),
            if distance > 4 { 0.75 } else { 1.0 }
        );
    }
    let mut nvg = night.clone();
    nvg["fighters"][0]["gear"]["nvg"] = json!(true);
    for distance in 1..=16 {
        assert_eq!(chance(&nvg, 0, distance), chance(&morning, 0, distance));
    }
    nvg["config"]["weather"] = json!("dust");
    assert_close(accuracy(&nvg, 0, 9), 0.7);
    let mut dusty_night = nvg.clone();
    dusty_night["fighters"][0]["gear"]["nvg"] = json!(false);
    assert_close(accuracy(&dusty_night, 0, 9), 0.7 * 0.75);
    for time in ["day", "evening", "night"] {
        let m = duel(time, "knife-is-not-a-shot");
        assert_eq!(
            call("hitChance", json!([m, 0, "knife", 2, 2, false])),
            call("hitChance", json!([morning, 0, "knife", 2, 2, false])),
            "{time} does not penalise a knife",
        );
    }
}

#[test]
fn evening_sun_selects_exactly_one_side_reproducibly() {
    let mut seen_sides = [false; 2];
    for i in 0..32 {
        let seed = format!("sun-{i}");
        let m = duel("evening", &seed);
        assert_eq!(
            m,
            duel("evening", &seed),
            "the same contract has the same sun side"
        );
        let faces: Vec<_> = (0..2)
            .filter(|&side| m["fighters"][side]["facesSun"] == true)
            .collect();
        assert_eq!(faces.len(), 1);
        let side = faces[0];
        seen_sides[side] = true;
        for distance in [2, 4, 5, 8, 9, 16] {
            assert_eq!(accuracy(&m, side, distance), 0.9);
            assert_eq!(accuracy(&m, 1 - side, distance), 1.0);
        }
        let next = call("resolveTurn", json!([m, [move_back(), move_back()]]));
        assert_eq!(next["match"]["fighters"][side]["facesSun"], true);
        for time in ["morning", "day", "night"] {
            let other = duel(time, &seed);
            assert!(other["fighters"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["facesSun"] == false));
        }
    }
    assert_eq!(
        seen_sides,
        [true, true],
        "the sun side is not permanently assigned to the player"
    );
}

#[test]
fn daylight_overheats_the_targets_shield_and_heat_protection_prevents_it() {
    let mut witnessed_heat_damage = false;
    for i in 0..64 {
        let mut cool = duel("morning", &format!("heat-{i}"));
        cool["distance"] = json!(4);
        let mut hot = cool.clone();
        hot["config"]["time"] = json!("day");
        let mut protected = hot.clone();
        protected["fighters"][1]["gear"]["heatProof"] = json!(true);
        let orders = json!([shot(), shot()]);
        let cold_report =
            call("resolveTurn", json!([cool, orders]))["report"]["attacks"][0].clone();
        let hot_report = call("resolveTurn", json!([hot, orders]))["report"]["attacks"][0].clone();
        let safe_report =
            call("resolveTurn", json!([protected, orders]))["report"]["attacks"][0].clone();
        assert_eq!(
            safe_report, cold_report,
            "the target's heat-proof gear restores the normal block"
        );
        assert_eq!(
            hot_report["chance"], cold_report["chance"],
            "heat does not penalise close shots"
        );
        assert_eq!(hot_report["hit"], cold_report["hit"]);
        if hot_report["hit"] == true {
            assert!(
                hot_report["damage"].as_f64().unwrap() >= cold_report["damage"].as_f64().unwrap()
            );
            witnessed_heat_damage |= hot_report["damage"] != cold_report["damage"];
        }
    }
    assert!(
        witnessed_heat_damage,
        "overheating changes the actual resolved damage"
    );
}

#[test]
fn night_flash_gives_away_a_shooter_for_the_next_turn_only() {
    let start = duel("night", "flash-lifetime");
    let baseline = chance(&start, 1, 6);
    let next = call("resolveTurn", json!([start, [shot(), move_back()]]))["match"].clone();
    assert_eq!(next["fighters"][0]["flash"], true);
    assert_eq!(next["fighters"][1]["flash"], false);
    assert_close(chance(&next, 1, 6), baseline + 0.1);
    assert_close(chance(&next, 0, 6), baseline);
    let rested = call("resolveTurn", json!([next, [move_back(), move_back()]]))["match"].clone();
    assert_eq!(rested["fighters"][0]["flash"], false);
    assert_close(chance(&rested, 1, 6), baseline);
    let day = duel("day", "flash-lifetime");
    let daytime = call("resolveTurn", json!([day, [shot(), shot()]]));
    assert!(daytime["match"]["fighters"]
        .as_array()
        .unwrap()
        .iter()
        .all(|f| f["flash"] == false));
}

#[test]
fn suppressors_follow_the_in_hand_gun_and_do_not_silence_a_sidearm() {
    let mut loadout = call("starterLoadout", json!(["rifle"]));
    loadout["weapon1"]["mods"] = json!({"muzzle":"muzzle-tishina"});
    loadout["weapon2"] = json!({"item":"pistol-pustelga"});
    let kitted = call("kitFighter", json!(["a", "Тень", loadout]));
    let mut m = duel("night", "active-suppressor");
    m["config"]["fighters"][0] = kitted;
    m = call("createMatch", json!([m["config"]]));
    m = call("resolveTurn", json!([m, [shot(), move_back()]]))["match"].clone();
    assert_eq!(m["fighters"][0]["flash"], false);
    let swap = json!({"action":{"type":"swap","move":0},"defend":["head","body"]});
    m = call("resolveTurn", json!([m, [swap, move_back()]]))["match"].clone();
    assert_eq!(m["fighters"][0]["weapon"], "pistol");
    assert_eq!(
        m["fighters"][0]["flash"], false,
        "swapping has no muzzle flash"
    );
    m = call("resolveTurn", json!([m, [shot(), move_back()]]))["match"].clone();
    assert_eq!(
        m["fighters"][0]["flash"], true,
        "the unsuppressed pistol gives the shooter away"
    );
}

#[test]
fn starter_gear_replays_plain_fights_at_every_time_of_day() {
    for time in ["morning", "day", "evening", "night"] {
        for seed in ["time-parity-a", "time-parity-b", "time-parity-c"] {
            let plain = duel(time, seed);
            let mut config = plain["config"].clone();
            for side in 0..2 {
                let setup = &plain["config"]["fighters"][side];
                let starter = call("starterLoadout", json!([setup["weapon"]]));
                config["fighters"][side] =
                    call("kitFighter", json!([setup["id"], setup["name"], starter]));
            }
            let mut p = plain;
            let mut k = call("createMatch", json!([config]));
            while p["result"].is_null() {
                let plain_orders = json!([
                    call("botOrder", json!([p, 0])),
                    call("botOrder", json!([p, 1]))
                ]);
                let kit_orders = json!([
                    call("botOrder", json!([k, 0])),
                    call("botOrder", json!([k, 1]))
                ]);
                assert_eq!(kit_orders, plain_orders, "{time} {seed}: bot decisions");
                let plain_result = call("resolveTurn", json!([p, plain_orders]));
                let kit_result = call("resolveTurn", json!([k, kit_orders]));
                assert_eq!(
                    kit_result["report"], plain_result["report"],
                    "{time} {seed}: resolved turn"
                );
                p = plain_result["match"].clone();
                k = kit_result["match"].clone();
                assert_eq!(
                    k["rng"], p["rng"],
                    "{time} {seed}: deterministic random stream"
                );
                assert_eq!(k["result"], p["result"], "{time} {seed}: match outcome");
            }
        }
    }
}
