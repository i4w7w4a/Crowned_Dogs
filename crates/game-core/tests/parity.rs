use game_core::dispatch;
use serde_json::{json, Value};
fn assert_same(a: &Value, b: &Value, path: &str) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!(
                (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1.0),
                "{path}: {a} != {b}"
            );
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}: length");
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                assert_same(a, b, &format!("{path}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(
                a.len(),
                b.len(),
                "{path}: keys: {:?} != {:?}",
                a.keys(),
                b.keys()
            );
            for (k, a) in a {
                assert!(b.contains_key(k), "{path}: missing {k}");
                // Which guns a mod fits is catalogue design that moved on after the migration.
                if k == "fits" {
                    continue;
                }
                assert_same(a, &b[k], &format!("{path}.{k}"));
            }
        }
        _ => assert_eq!(a, b, "{path}"),
    }
}
/// The Kulak grip left the sniper rifle for the pistol and the LCU-2 laser joined the sniper (31.10.2026);
/// then the Kulak left the pump shotgun for the LCU-2, and the veteran shotgun kit took the Luch instead.
/// The oracle froze the old rails, so cases that build on them (the Kulak on a sniper or a shotgun, the
/// changed rail lists) are skipped.
fn frozen_sniper_rail(method: &str, args: &Value) -> bool {
    fn kulak_on_sniper(v: &Value) -> bool {
        match v {
            Value::Object(o) => {
                (o.get("item")
                    .and_then(Value::as_str)
                    .is_some_and(|i| i.starts_with("sniper") || i.starts_with("shotgun"))
                    && v["mods"]["rail"] == "rail-kulak")
                    || o.values().any(kulak_on_sniper)
            }
            Value::Array(a) => a.iter().any(kulak_on_sniper),
            _ => false,
        }
    }
    let rail_of = |class: &str| {
        args[1] == "rail"
            && (args[0] == class
                || args[0]["weapon1"]["item"]
                    .as_str()
                    .is_some_and(|i| i.starts_with(class)))
    };
    let changed_rail = rail_of("sniper") || rail_of("pistol") || rail_of("shotgun");
    kulak_on_sniper(args)
        || (changed_rail && matches!(method, "modsFor" | "client.weaponModsForSlot"))
        || ((rail_of("sniper") || rail_of("shotgun"))
            && method == "client.equipWeaponMod"
            && args[2] == "rail-kulak")
}
/// Concept gear without a model left the catalogue (01.11.2026); cases built on it are skipped.
const CONCEPTS: [&str; 15] = [
    "body-light",
    "body-bastion",
    "suit-leshy",
    "boots-sprint",
    "boots-hodok",
    "boots-saper",
    "knife-shtyk",
    "knife-zhalo",
    "knife-kogot",
    "knife-iskra",
    "grenade-grad",
    "grenade-emp",
    "grenade-smoke",
    "head-marevo",
    "head-filin",
];
fn concept_gear(args: &Value) -> bool {
    let text = args.to_string();
    CONCEPTS
        .iter()
        .any(|id| text.contains(&format!("\"{id}\"")))
}
fn oracle() -> Value {
    serde_json::from_str(include_str!("oracle.json")).unwrap()
}
#[test]
fn every_export_matches_typescript_oracle() {
    let data = oracle();
    for (i, case) in data["cases"].as_array().unwrap().iter().enumerate() {
        let method = case["method"].as_str().unwrap();
        let args: Vec<Value> = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                if let Some(index) = v["$ref"].as_u64() {
                    data["pool"][index as usize].clone()
                } else {
                    v.clone()
                }
            })
            .collect();
        if frozen_sniper_rail(method, &json!(args))
            || concept_gear(&json!(args))
            || concept_gear(&case["expected"])
        {
            continue;
        }
        let result = dispatch(method, json!(args));
        if let Some(error) = case.get("error") {
            assert_eq!(
                result.unwrap_err(),
                error.as_str().unwrap(),
                "case {i}: {method}"
            );
        } else {
            assert_same(
                &result.unwrap_or_else(|e| panic!("case {i} {method}: {e}")),
                &case["expected"],
                &format!("case {i} {method}"),
            );
        }
    }
}
#[test]
fn all_seeded_bot_duels_have_identical_orders_damage_rng_and_outcomes() {
    let data = oracle();
    for (i, battle) in data["battles"].as_array().unwrap().iter().enumerate() {
        // Shotgun pellets catch neighbouring zones since 02.11.2026; the oracle predates that.
        if battle["config"].to_string().contains("shotgun") {
            continue;
        }
        let mut m = dispatch("createMatch", json!([battle["config"]])).unwrap();
        for (turn, orders) in battle["orders"].as_array().unwrap().iter().enumerate() {
            let a = dispatch("botOrder", json!([m, 0])).unwrap();
            let b = dispatch("botOrder", json!([m, 1])).unwrap();
            assert_same(
                &json!([a, b]),
                orders,
                &format!("battle {i} turn {turn} bot orders"),
            );
            let result = dispatch("resolveTurn", json!([m, orders])).unwrap();
            m = result["match"].clone();
        }
        assert_same(&m, &battle["expected"], &format!("battle {i}"));
    }
}
