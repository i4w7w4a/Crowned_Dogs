use serde_json::{json, Value};

#[test]
fn earned_experience_stops_at_four_with_a_valid_point_budget() {
    let mut profile = game_core::dispatch("syncFresh", json!([])).unwrap();
    profile = game_core::dispatch("syncSpend", json!([profile, "power"])).unwrap();
    // A large stored XP balance and repeated completed wins must never overrun the cap.
    profile["xp"] = json!(1_000_000);
    let fight = json!({"result":{"winner":0},"history":[],
        "config":{"fighters":[{"weapon":"pistol"}]}});
    for _ in 0..100 {
        let gain = game_core::dispatch("syncGain", json!([profile, fight, 0])).unwrap();
        profile = gain["profile"].clone();
        assert!(profile["level"].as_f64().unwrap() <= 4.0);
    }
    assert_eq!(profile["level"], json!(4.0));
    assert_eq!(profile["xp"], json!(0.0));
    let spent: f64 = profile["stats"].as_object().unwrap().values()
        .map(|v: &Value| v.as_f64().unwrap()).sum();
    assert_eq!(spent, 1.0);
    assert_eq!(spent + profile["free"].as_f64().unwrap(), 12.0);
}
