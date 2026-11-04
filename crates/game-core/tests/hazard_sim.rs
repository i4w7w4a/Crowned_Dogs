//! Map-event pace and fairness over many bot battles (stage 9, Art 4 Nov). Run:
//! `cargo test -p game-core --test hazard_sim -- --ignored --nocapture`
//! Side 0 plays as a player who ignores the warnings (the worst case); side 1 is the bot.
use game_core::dispatch;
use serde_json::{json, Value};

fn play(cfg: Value, blind: bool) -> (usize, usize, [usize; 2], [f64; 2], usize) {
    let mut m = dispatch("createMatch", json!([cfg])).unwrap();
    let (mut turns, mut events, mut hits, mut dmg, mut back) = (0, 0, [0; 2], [0.0; 2], 0);
    let mut last: Option<usize> = None;
    while m["result"].is_null() {
        let mut seen = m.clone();
        if blind {
            seen["hazard"] = Value::Null;
        }
        let orders = json!([
            dispatch("botOrder", json!([seen, 0])).unwrap(),
            dispatch("botOrder", json!([m, 1])).unwrap()
        ]);
        let out = dispatch("resolveTurn", json!([m, orders])).unwrap();
        m = out["match"].clone();
        turns += 1;
        let h = &out["report"]["hazard"];
        if h["kind"].is_string() {
            events += 1;
            if last.is_some_and(|t| turns - t <= 2) {
                back += 1;
            }
            last = Some(turns);
            for hit in h["hits"].as_array().unwrap() {
                let side = hit["side"].as_u64().unwrap() as usize;
                hits[side] += 1;
                dmg[side] += hit["damage"].as_f64().unwrap();
            }
        }
    }
    (turns, events, hits, dmg, back)
}

#[test]
#[ignore]
fn pace_and_fairness() {
    let maps = ["saltHall", "blocks", "dam", "dryBed", "intake", "backwater", "railyard", "sludgePonds"];
    for blind in [true, false] {
        for weather in ["clear", "storm", "fog", "dust"] {
            let (mut b, mut t, mut e, mut h, mut d, mut back) = (0, 0, 0, [0; 2], [0.0; 2], 0);
            for (mi, map) in maps.iter().enumerate() {
                for i in 0..130 {
                    let (ga, gb) = (["rifle","pistol","shotgun","sniper"][i%4], ["shotgun","sniper","rifle","pistol"][i%4]);
                    let cfg = json!({"seed":format!("sim{mi}-{i}"),"mapId":map,"weather":weather,"time":"day","events":true,
                        "shelled": i % 8 == 0,
                        "fighters":[{"id":"a","name":"A","weapon":ga},
                                    {"id":"b","name":"B","weapon":gb}]});
                    let (tt, ee, hh, dd, bb) = play(cfg, blind);
                    b += 1; t += tt; e += ee; back += bb;
                    for s in 0..2 { h[s] += hh[s]; d[s] += dd[s]; }
                }
            }
            println!(
                "{} {weather:5}: {b} battles, {:.1} turns, {:.2} events/battle ({:.0}% of turns), back-to-back {:.0}%, hits player {} bot {}, damage/battle player {:.1} bot {:.1}",
                if blind { "blind" } else { "both " },
                t as f64 / b as f64, e as f64 / b as f64, 100. * e as f64 / t as f64,
                100. * back as f64 / e.max(1) as f64, h[0], h[1], d[0] / b as f64, d[1] / b as f64
            );
        }
    }
}
