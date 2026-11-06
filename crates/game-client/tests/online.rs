use game_client::dispatch;
use serde_json::{json, Value};

fn call(method: &str, args: Value) -> Value {
    dispatch(&format!("server.online.{method}"), args).unwrap()
}
fn player(id: &str) -> Value {
    json!({"id":id,"tag":format!("K-001-{id}"),"telegram":{"firstName":id,"lastName":null,"username":null},
        "profile":game_core::dispatch("syncFresh",json!([])).unwrap(),
        "appearance":dispatch("client.look.normalize",json!([{}])).unwrap(),
        "loadout":game_core::dispatch("starterLoadout",json!(["pistol"])).unwrap()})
}
fn battle() -> Value {
    call(
        "create",
        json!([{"id":"match-a","players":[player("a"),player("b")],"seed":"private-seed","now":1000,
        "setting":{"mapId":"saltHall","weather":"clear","time":"day","events":false}}]),
    )
}
fn act(s: &Value, who: &str, now: u64, action: Value) -> Value {
    call(
        "transition",
        json!([s,who,{"expectedVersion":s["version"],"turn":s["match"]["turn"],"action":action},now]),
    )
}
fn active() -> Value {
    let s = battle();
    let s = act(&s, "a", 1100, json!({"type":"ready"}));
    act(&s, "b", 1200, json!({"type":"ready"}))
}
#[test]
fn loading_waits_for_both_players_and_expires_without_result() {
    let s = battle();
    assert_eq!(s["phase"], "loading");
    assert!(s["deadline"].is_null());
    let s = act(&s, "a", 2000, json!({"type":"ready"}));
    assert!(s["deadline"].is_null());
    let s = call("timeout", json!([s, 301000]));
    assert_eq!(s["phase"], "cancelled");
    assert!(s["match"]["result"].is_null());
    assert_eq!(s["cancelReason"], "loading_timeout");
    let s = active();
    assert_eq!(s["phase"], "active");
    assert!(s["deadline"].as_f64().unwrap() > 1200.0);
}
#[test]
fn snapshot_roundtrip_preserves_first_order_and_resolves_once() {
    let mut s = active();
    for command in [
        json!({"type":"zone","zone":"head"}),
        json!({"type":"guard","zone":"head"}),
        json!({"type":"guard","zone":"body"}),
    ] {
        s = act(&s, "a", 1300, json!({"type":"press","command":command}));
    }
    s = act(&s, "a", 1400, json!({"type":"submit"}));
    assert!(!s["orders"][0].is_null());
    let restored: Value = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    let resolved = call("timeout", json!([restored, 1000000]));
    assert_eq!(resolved["match"]["turn"].as_f64(), Some(2.0));
    assert_eq!(resolved["match"]["history"].as_array().unwrap().len(), 1);
    assert!(resolved["deadline"].as_f64().unwrap() > 1000000.0);
    assert!(dispatch("server.online.timeout", json!([resolved, 1000000])).is_err());
}
#[test]
fn projection_hides_entropy_and_opponent_intent_even_for_future_fields() {
    let mut s = active();
    s = act(
        &s,
        "b",
        1300,
        json!({"type":"press","command":{"type":"zone","zone":"head"}}),
    );
    s["internalSecret"] = json!("hidden");
    s["match"]["futurePrivate"] = json!("hidden");
    s["match"]["config"]["futurePrivate"] = json!("hidden");
    let view = call("project", json!([s, "a"]));
    let wire = serde_json::to_string(&view).unwrap();
    assert!(!wire.contains("private-seed"));
    assert!(!wire.contains("rng"));
    assert!(!wire.contains("hidden"));
    assert!(view.get("orders").is_none());
    assert!(view.get("drafts").is_none());
    assert!(view["own"]["draft"]["zone"].is_null());
    assert_eq!(view["side"], 0);
    assert_eq!(call("project", json!([s, "b"]))["side"], 1);
    assert!(dispatch("server.online.project", json!([s, "stranger"])).is_err());
}
#[test]
fn deadline_version_and_action_validation_precede_mutation() {
    let s = active();
    let envelope = json!({"expectedVersion":s["version"],"turn":1,"action":{"type":"press","command":{"type":"zone","zone":"brain"}}});
    assert!(dispatch("server.online.transition", json!([s, "a", envelope, 1300])).is_err());
    let envelope = json!({"expectedVersion":0,"turn":1,"action":{"type":"ready"}});
    assert!(dispatch("server.online.transition", json!([s, "a", envelope, 1300])).is_err());
    let envelope = json!({"expectedVersion":s["version"],"turn":1,"action":{"type":"submit"}});
    assert_eq!(
        dispatch(
            "server.online.transition",
            json!([s, "a", envelope, s["deadline"]])
        )
        .unwrap_err(),
        "deadline_passed"
    );
}
#[test]
fn both_stored_profiles_are_applied_to_compiled_gear() {
    let mut p = player("a");
    p["profile"] = game_core::dispatch("syncSpend", json!([p["profile"], "power"])).unwrap();
    let s = call(
        "create",
        json!([{"id":"m","players":[p,player("b")],"seed":"s","now":1,
        "setting":{"mapId":"saltHall","weather":"clear","time":"day"}}]),
    );
    assert!(
        s["match"]["config"]["fighters"][0]["kit"]["weapon"]["power"]
            .as_f64()
            .unwrap()
            > s["match"]["config"]["fighters"][1]["kit"]["weapon"]["power"]
                .as_f64()
                .unwrap()
    );
}
#[test]
fn legal_groin_target_and_integer_wire_turn_work_after_timeout() {
    let s = call("timeout", json!([active(), 100000]));
    let s = call(
        "transition",
        json!([s,"a",{"expectedVersion":s["version"],"turn":2,
        "action":{"type":"press","command":{"type":"zone","zone":"groin"}}},100001]),
    );
    assert_eq!(s["drafts"][0]["zone"], "groin");
}
#[test]
fn surrender_finishes_and_rewards_are_derived_only_for_finished_battles() {
    let s = active();
    assert!(dispatch("server.online.rewards", json!([s])).is_err());
    let s = act(&s, "a", 1300, json!({"type":"surrender"}));
    assert_eq!(s["phase"], "finished");
    assert_eq!(s["match"]["result"]["winner"], 1);
    let rewards = call("rewards", json!([s]));
    assert_eq!(rewards.as_array().unwrap().len(), 2);
    assert!(rewards[1]["xp"].as_f64().unwrap() > rewards[0]["xp"].as_f64().unwrap());
}
#[test]
fn profile_accepts_intentions_and_rejects_forged_stats_tag_and_items() {
    let p = player("a");
    let next = call("profile", json!([p,{"type":"spend","stat":"power"}]));
    assert_eq!(next["profile"]["stats"]["power"].as_f64(), Some(1.0));
    assert!(dispatch(
        "server.online.profile",
        json!([p,{"type":"spend","stat":"power","level":4}])
    )
    .is_err());
    assert!(dispatch(
        "server.online.profile",
        json!([p,{"type":"equip","slot":"weapon1","itemId":"forged"}])
    )
    .is_err());
    let mut appearance = p["appearance"].clone();
    appearance["callsign"] = json!("Игрок");
    appearance["tag"] = json!("stolen");
    assert!(dispatch(
        "server.online.profile",
        json!([p,{"type":"appearance","value":appearance}])
    )
    .is_err());
    appearance["tag"] = p["tag"].clone();
    assert_eq!(
        call(
            "profile",
            json!([p,{"type":"appearance","value":appearance}])
        )["appearance"]["callsign"],
        "Игрок"
    );
}
#[test]
fn invitation_owner_self_accept_and_deadline_are_policy() {
    let inv = json!({"status":"open","hostId":"a","version":1,"expiresAt":2000});
    assert!(dispatch("server.online.invite", json!([inv, "a", "accept", 1, 1000])).is_err());
    assert!(dispatch("server.online.invite", json!([inv, "b", "cancel", 1, 1000])).is_err());
    assert_eq!(
        call("invite", json!([inv, "b", "accept", 1, 1000])),
        "accepted"
    );
    assert_eq!(
        call("invite", json!([inv, "b", "expire", 1, 2000])),
        "expired"
    );
    assert!(dispatch("server.online.invite", json!([inv, "b", "accept", 1, 2000])).is_err());
}
