use serde_json::{Value, json};

fn project(value: &Value) -> Value {
    game_client::dispatch("client.online.view", json!([value])).expect("public battle presentation")
}

fn battle(side: u8) -> Value {
    json!({"schemaVersion":1,"id":"match","version":8,"phase":"active","side":side,
    "players":[{"name":"Первый","tag":"A","loadout":{"weapon1":"rifle"}},{"name":"Второй","tag":"B","loadout":{"weapon1":"pistol"}}],
    "ready":[true,false],"submitted":[true,false],"own":{"draft":{"move":1},"submitted":false},
    "result":{"winner":1,"reason":"hp"},"match":{
      "config":{"fighters":[{"name":"Первый"},{"name":"Второй"}]},
      "fighters":[{"hp":10},{"hp":40}],"distance":7,"lane":[2,9],
      "result":{"winner":1,"reason":"hp"},"hazard":{"at":8,"radius":1},
      "history":[{"turn":1,"moves":[-1,2],"actions":[{"type":"wait"},{"type":"shoot","zone":"head"}],
        "attacks":[null,{"damage":17,"zone":"head"}],"defend":[["body"],["head"]],"hp":[10,40],"afk":[true,false],"stunned":[false,true],"lane":[2,9],
        "hazard":{"at":8,"hpBefore":[20,40],"hits":[{"side":0,"damage":10}]}}]
    }})
}

#[test]
fn side_one_is_local_zero_for_every_render_channel() {
    let original = battle(1);
    let v = project(&original);
    assert_eq!(original["match"]["lane"], json!([2, 9]));
    assert_eq!(v["fighters"][0]["name"], "Второй");
    assert_eq!(v["fighters"][0]["kit"], json!({"weapon1":"pistol"}));
    assert_eq!(v["match"]["fighters"], json!([{"hp":40},{"hp":10}]));
    assert_eq!(v["match"]["config"]["fighters"][0]["name"], "Второй");
    assert_eq!(v["match"]["lane"], json!([-9.0, -2.0]));
    assert_eq!(v["match"]["hazard"]["at"], json!(-8.0));
    assert_eq!(v["match"]["result"]["winner"], 0);
    assert_eq!(v["draft"], json!({"move":1})); // moves are towards the foe, never negated
    let r = &v["match"]["history"][0];
    assert_eq!(r["moves"], json!([2, -1]));
    assert_eq!(r["hp"], json!([40, 10]));
    assert_eq!(r["actions"][0]["zone"], "head");
    assert_eq!(r["attacks"][0]["damage"], 17);
    assert_eq!(r["defend"], json!([["head"], ["body"]]));
    assert_eq!(r["afk"], json!([false, true]));
    assert_eq!(r["stunned"], json!([true, false]));
    assert_eq!(r["lane"], json!([-9.0, -2.0]));
    assert_eq!(r["hazard"]["at"], json!(-8.0));
    assert_eq!(r["hazard"]["hits"][0]["side"], 1);
    assert_eq!(r["hazard"]["hpBefore"], json!([40, 20]));
}

#[test]
fn side_zero_and_draw_keep_canonical_values() {
    let mut b = battle(0);
    b["match"]["result"]["winner"] = Value::Null;
    let v = project(&b);
    assert_eq!(v["match"], b["match"]);
    assert!(v["match"]["result"]["winner"].is_null());
}

#[test]
fn unknown_side_never_silently_controls_the_first_player() {
    let mut b = battle(1);
    b["side"] = json!(2);
    assert!(game_client::dispatch("client.online.view", json!([b])).is_err());
}

#[test]
fn move_selection_does_not_toggle_existing_choice_off() {
    let ask = |draft: Value, n: i32| {
        game_client::dispatch("client.online.moveCommands", json!([draft, n])).unwrap()
    };
    assert_eq!(ask(json!({"kind":"run","move":2}), 2), json!([]));
    assert_eq!(
        ask(json!({"kind":"run","move":2}), 3),
        json!([{"type":"step","move":0},{"type":"dash"}])
    );
    assert_eq!(
        ask(json!({"kind":"shoot","move":0}), -1),
        json!([{"type":"step","move":-1}])
    );
}

#[test]
fn resume_and_missing_reports_snap_but_one_new_report_plays() {
    let ask = |version: i32, cursor: i32, resume: bool| {
        game_client::dispatch("client.online.advance",json!([{"id":"a","version":4,"eventCursor":2},{"id":"a","version":version,"eventCursor":cursor},resume])).unwrap()
    };
    assert_eq!(ask(3, 1, false), "ignore");
    assert_eq!(ask(5, 3, false), "play");
    assert_eq!(ask(6, 4, false), "snap");
    assert_eq!(ask(5, 3, true), "snap");
    assert_eq!(ask(5, 2, false), "update");
}

#[test]
fn equipment_changes_emit_only_intentions_never_calculated_stats() {
    let before = json!({"weapon1":{"item":"rifle-varyag"},"head":"head-cap","colors":{}});
    let after = json!({"weapon1":{"item":"rifle-varyag","mods":{"optic":"optic-gran"}},"head":"head-k6","colors":{"head-k6":"black"},"level":99,"damage":999});
    let actions =
        game_client::dispatch("client.online.profileActions", json!([before, after])).unwrap();
    assert_eq!(
        actions,
        json!([{"type":"equip","slot":"head","itemId":"head-k6"},{"type":"mod","slot":"optic","itemId":"optic-gran"},{"type":"paint","itemId":"head-k6","color":"black"}])
    );
}
