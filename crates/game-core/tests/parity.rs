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
                assert_same(a, &b[k], &format!("{path}.{k}"));
            }
        }
        _ => assert_eq!(a, b, "{path}"),
    }
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
