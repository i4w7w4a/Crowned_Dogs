//! Synthetic public fixtures only; no HTTP login or production identity bypass.
use serde_json::{json, Value};
fn call(name: &str, args: Value) -> Value {
    game_client::dispatch(&format!("server.online.{name}"), args).unwrap()
}
fn main() {
    let now = 1793966400000u64;
    let accounts:Vec<_>=(0..2).map(|side|json!({"id":format!("00000000-0000-4000-8000-00000000000{}",side+1),"tag":format!("K-001-00{}",side+1),
        "telegram":{"firstName":if side==0{"Искра"}else{"Соль"},"lastName":null,"username":null},
        "profile":game_core::dispatch("syncFresh",json!([])).unwrap(),
        "appearance":game_client::dispatch("client.look.normalize",json!([{"callsign":if side==0{"Искра"}else{"Соль"},"sex":"male"}])).unwrap(),
        "loadout":game_core::dispatch("starterLoadout",json!(["pistol"])).unwrap()})).collect();
    let mut s = call(
        "create",
        json!([{"id":"10000000-0000-4000-8000-000000000001","players":accounts,"seed":"synthetic-private-entropy","now":now,
        "setting":{"mapId":"saltHall","weather":"clear","time":"day","events":true}}]),
    );
    let view = |s: &Value, clock: u64| {
        let mut v = call("project", json!([s, accounts[1]["id"]]));
        v["serverNow"] = json!(clock);
        v
    };
    let loading = view(&s, now);
    for i in 0..2 {
        s = call(
            "transition",
            json!([s,accounts[i]["id"],{"expectedVersion":s["version"],"turn":1,"action":{"type":"ready"}},now+1000]),
        );
    }
    let active = view(&s, now + 1000);
    s = call("timeout", json!([s, now + 100000]));
    s = call(
        "transition",
        json!([s,accounts[0]["id"],{"expectedVersion":s["version"],"turn":s["match"]["turn"],"action":{"type":"surrender"}},now+100001]),
    );
    let mut finished = view(&s, now + 100001);
    finished["reward"] = call("rewards", json!([s]))[1].clone();
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"loading":loading,"active":active,"finished":finished})
        )
        .unwrap()
    );
}
