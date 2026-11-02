use crate::{core, data, num, text, truth};
use serde_json::{json, Value};

fn plural<'a>(n: f64, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    let a = n.abs() % 100.0;
    let b = a % 10.0;
    if a > 10.0 && a < 20.0 {
        many
    } else if b == 1.0 {
        one
    } else if (2.0..=4.0).contains(&b) {
        few
    } else {
        many
    }
}
fn move_word(a: &Value) -> &'static str {
    match text(&a["type"]) {
        "dash" => "рывок",
        "run" => {
            if num(&a["move"]) > 0.0 {
                "перебежка вперёд"
            } else {
                "перебежка назад"
            }
        }
        "flee" => "уход из боя",
        "wait" => "",
        _ => {
            if num(&a["move"]) > 0.0 {
                "шаг вперёд"
            } else if num(&a["move"]) < 0.0 {
                "шаг назад"
            } else {
                ""
            }
        }
    }
}
fn zone_name(z: &str) -> &'static str {
    match z {
        "head" => "голова",
        "body" => "корпус",
        "groin" => "пах",
        "legs" => "ноги",
        _ => "",
    }
}
fn turn_line(r: &Value, side: usize, in_hand: &str) -> Value {
    let action = &r["actions"][side];
    let attack = &r["attacks"][side];
    let mv = move_word(action);
    let join = |s: String| {
        if mv.is_empty() {
            s
        } else {
            format!("{mv}, {s}")
        }
    };
    let outcome = if attack.is_null() {
        json!({"kind":"none","text":"без атаки"})
    } else if !truth(&attack["hit"]) {
        json!({"kind":"miss"})
    } else if truth(&attack["blocked"]) {
        json!({"kind":"blocked","damage":attack["damage"]})
    } else {
        json!({"kind":"hit","damage":attack["damage"],"crit":attack["crit"]})
    };
    let target = if attack["weapon"] == "grenade" {
        "граната"
    } else {
        zone_name(text(&attack["zone"]))
    };
    let mut line = json!({"back":false,"dash":false,"target":target,"outcome":outcome});
    match text(&action["type"]) {
        "shoot" | "knife" => {
            line["icon"] = json!(if action["type"] == "knife" {
                "knife"
            } else {
                "shot"
            });
            line["act"] = json!(join(format!(
                "{} {}",
                if action["type"] == "knife" {
                    "нож"
                } else {
                    "выстрел"
                },
                text(&data()["ZONE_TO"][text(&action["zone"])])
            )));
        }
        "grenade" => {
            line["icon"] = json!("grenade");
            line["act"] = json!(join(
                match text(&attack["effect"]) {
                    "emp" => "ЭМИ-граната",
                    "smoke" => "дымовая граната",
                    _ => "граната",
                }
                .into()
            ));
        }
        "swap" => {
            line["icon"] = json!("swap");
            line["act"] = json!(join(format!("взять {}", text(&data()["GUN_ACC"][in_hand]))));
            line["outcome"] = json!({"kind":"none","text":"без выстрела"});
        }
        "wait" => {
            line["icon"] = json!("move");
            line["act"] = json!("приказ не отдан");
            line["outcome"] = json!({"kind":"none","text":"пропуск"});
        }
        _ => {
            line["icon"] = json!("move");
            line["act"] = json!(mv);
            line["back"] = json!(action["type"] == "run" && num(&action["move"]) < 0.0);
            line["dash"] = json!(action["type"] == "dash");
        }
    }
    line
}
fn end_words(res: &Value, me: u64, foe: &str, turns: f64) -> Value {
    let winner = res["winner"].as_u64();
    let won = winner == Some(me);
    let lost = winner == Some(1 - me);
    let reason = text(&res["reason"]);
    let title = if lost {
        "Связь потеряна"
    } else if won {
        "Цель подавлена"
    } else {
        "Ничья"
    };
    let line = if winner.is_none() {
        match reason {
            "double_ko" => "Обе Персоны выведены из строя одновременно.",
            "fled" => "Обе Персоны ушли из боя.",
            _ => "Время вышло, здоровья поровну.",
        }
        .to_owned()
    } else if won {
        match reason {
            "fled" => format!("Персона «{foe}» ушла из боя."),
            "afk" => "Соперник три хода не отвечал.".into(),
            "turn_limit" => "Время вышло, у тебя больше здоровья.".into(),
            _ => format!(
                "Персона «{foe}» выведена из строя за {turns} {}. Добыча твоя.",
                plural(turns, "ход", "хода", "ходов")
            ),
        }
    } else {
        match reason {
            "fled" => "Персона ушла из боя и сохранила половину добычи.",
            "afk" => "Три хода подряд без приказа: связь разорвана.",
            "turn_limit" => "Время вышло, у соперника больше здоровья.",
            _ => "Персона уничтожена. Ты очнёшься в капсуле на базе «Соль».",
        }
        .into()
    };
    json!({"title":title,"text":line,"again":if lost{"Очнуться в капсуле"}else{"Новый бой"},"lost":lost})
}
/// Whether lane position `x` is inside a warned stretch (as `game_core::hazard::inside`).
fn inside(h: &Value, x: f64) -> bool {
    !h["at"].is_null() && (x - num(&h["at"])).abs() < num(&h["radius"]) + 0.5
}
/// Where a warned stretch lies for the player: under them, ahead or behind, in whole steps.
fn stretch_words(h: &Value, x: f64) -> String {
    if inside(h, x) {
        return "там, где ты стоишь".into();
    }
    let at = num(&h["at"]);
    let r = num(&h["radius"]);
    let (near, far) = ((at - x).abs() - r, (at - x).abs() + r);
    let near = near.ceil().max(1.0);
    let far = far.floor().max(near);
    let side = if at > x { "перед тобой" } else { "позади" };
    if near == far {
        format!("в {near} {} {side}", plural(near, "шаге", "шагах", "шагах"))
    } else {
        format!("в {near}–{far} шагах {side}")
    }
}
/// The warning for the turn being planned and the moves that end inside the danger, for
/// the player (fighter 0). Null when nothing threatens and the air is clean.
fn hazard_warning(m: &Value) -> Value {
    let me = &m["fighters"][0];
    let stunned = num(&me["stunned"]) > 0.0;
    let h = &m["hazard"];
    let spec = &game_core::constants()["HAZARDS"][text(&h["kind"])];
    let mut risky: Vec<f64> = vec![];
    let warned = !h["kind"].is_null() && h["turn"] == m["turn"];
    let line = if !warned {
        String::new()
    } else {
        let x = num(&m["lane"][0]);
        let foe = num(&m["lane"][1]);
        let distance = num(&m["distance"]);
        for mv in [3.0, 2.0, 1.0, 0.0, -1.0, -2.0] {
            let planned = num(&core("plannedDistance", json!([m, mv])).unwrap_or(Value::Null));
            let landing = x + (distance - planned);
            if (text(&h["kind"]) == "drone" && mv == 0.0) || inside(h, landing) {
                risky.push(mv);
            }
        }
        let foe_in = if inside(h, foe) { " Противник тоже под ударом." } else { "" };
        let damage = num(&spec["damage"]);
        match text(&h["kind"]) {
            "lightning" => format!(
                "Молния ударит {}: −{damage} и оглушение на ход.{foe_in}",
                stretch_words(h, x)
            ),
            "shell" => format!(
                "Снаряд ляжет {}: −{damage}, на краю −{}.{foe_in}",
                stretch_words(h, x),
                (damage * num(&spec["edge"])).floor()
            ),
            "collapse" => format!("Потолок рухнет {}: −{damage}.{foe_in}", stretch_words(h, x)),
            "drone" => format!("Дрон-камикадзе: ударит в того, кто останется на месте, −{damage}."),
            "blackout" => "Свет погаснет: этот ход пройдёт в темноте.".into(),
            _ => String::new(),
        }
    };
    let toxic = num(&core("conditionToxic", json!([m])).unwrap_or(Value::Null));
    let mut text_out = if stunned {
        "Оглушён молнией: этот ход стоишь на месте, щит не работает.".to_string()
    } else {
        String::new()
    };
    if !line.is_empty() {
        if !text_out.is_empty() {
            text_out.push(' ');
        }
        text_out.push_str(&line);
    }
    if text_out.is_empty() && toxic > 0.0 {
        text_out = format!("Ядовитый воздух: −{toxic} здоровья обоим каждый ход.");
    }
    if text_out.is_empty() {
        return Value::Null;
    }
    json!({"kind": if warned { h["kind"].clone() } else if stunned { json!("stun") } else { json!("toxic") },
        "text": text_out, "risky": risky, "stunned": stunned})
}
/// What a turn's hazard did, for the fight log: null when nothing happened.
fn hazard_line(r: &Value, names: &Value) -> Value {
    let h = &r["hazard"];
    if h.is_null() {
        return Value::Null;
    }
    let who = |side: u64| {
        if side == 0 {
            "Ты".to_string()
        } else {
            text(&names[1]).to_string()
        }
    };
    let mut parts: Vec<String> = h["hits"]
        .as_array()
        .map(|hits| {
            hits.iter()
                .map(|hit| {
                    format!(
                        "{} −{}{}",
                        who(hit["side"].as_u64().unwrap_or(0)),
                        hit["damage"],
                        if truth(&hit["stun"]) { ", оглушение" } else { "" }
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let kind = text(&h["kind"]);
    let mut out = String::new();
    if !kind.is_empty() {
        let name = text(&h["name"]);
        let mut chars = name.chars();
        let name = chars
            .next()
            .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
            .unwrap_or_default();
        if parts.is_empty() && kind != "blackout" {
            parts.push("мимо".into());
        }
        out = if parts.is_empty() {
            name
        } else {
            format!("{name}: {}", parts.join(", "))
        };
    }
    if num(&h["toxic"]) > 0.0 {
        if !out.is_empty() {
            out.push_str(" · ");
        }
        out.push_str(&format!("яд: оба −{}", h["toxic"]));
    }
    let icon = match kind {
        "lightning" | "blackout" => "storm",
        "drone" => "drone",
        "shell" | "collapse" => "blast",
        _ => "dust",
    };
    json!({"icon": icon, "text": out, "hurt": h["hits"].as_array().is_some_and(|a| a.iter().any(|x| x["side"] == 0))})
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "words.plural" => json!(plural(
            num(&args[0]),
            text(&args[1]),
            text(&args[2]),
            text(&args[3])
        )),
        "words.steps" => json!(plural(num(&args[0]), "шаг", "шага", "шагов")),
        "words.turnsWord" => json!(plural(num(&args[0]), "ход", "хода", "ходов")),
        "words.moveWord" => json!(move_word(&args[0])),
        "words.zoneName" => json!(zone_name(text(&args[0]))),
        "words.turnLine" => turn_line(
            &args[0],
            args[1].as_u64().unwrap_or(0) as usize,
            text(&args[2]),
        ),
        "words.hazardWarning" => hazard_warning(&args[0]),
        "words.hazardLine" => hazard_line(&args[0], &args[1]),
        "words.endWords" => end_words(
            &args[0],
            args[1].as_u64().unwrap_or(0),
            text(&args[2]),
            num(&args[3]),
        ),
        _ => return Err(format!("Unknown words operation: {method}")),
    })
}
