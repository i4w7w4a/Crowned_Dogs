use serde_json::Value;

fn same(actual: &Value, expected: &Value, path: &str) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => assert!(
            (a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 1e-10,
            "{path}: {a} != {b}"
        ),
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path} length");
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                same(a, b, &format!("{path}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(
                a.len(),
                b.len(),
                "{path} keys: {:?} != {:?}",
                a.keys().collect::<Vec<_>>(),
                b.keys().collect::<Vec<_>>()
            );
            for (k, a) in a {
                same(
                    a,
                    b.get(k).unwrap_or_else(|| panic!("{path}.{k} missing")),
                    &format!("{path}.{k}"),
                );
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

#[test]
fn client_behavior_matches_the_pre_migration_game() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("../fixtures/oracle.json")).unwrap();
    for (index, case) in cases.iter().enumerate() {
        let method = case["method"].as_str().unwrap();
        let actual = game_client::dispatch(method, case["args"].clone())
            .unwrap_or_else(|e| panic!("case {index} {method}: {e}"));
        same(&actual, &case["value"], &format!("case {index} {method}"));
    }
}

#[test]
fn arena_math_matches_the_pre_migration_three_scene() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/arena-oracle.json")).unwrap();
    for (index, case) in cases.iter().enumerate() {
        let kind = case["kind"].as_str().unwrap();
        let input: Vec<f64> = case["input"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let actual = game_client::arena_compute(kind, &input).unwrap();
        same(
            &serde_json::json!(actual),
            &case["value"],
            &format!("arena case {index} {kind}"),
        );
    }
}
