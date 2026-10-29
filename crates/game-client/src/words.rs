use crate::{data, num, text, truth};
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
        "words.endWords" => end_words(
            &args[0],
            args[1].as_u64().unwrap_or(0),
            text(&args[2]),
            num(&args[3]),
        ),
        _ => return Err(format!("Unknown words operation: {method}")),
    })
}
