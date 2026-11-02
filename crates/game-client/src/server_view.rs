//! Telegram presentation is plain Rust data; grammY remains only the API adapter.
use crate::{core, data, num, server, text, truth, words};
use serde_json::{json, Value};
const ZONES: [&str; 4] = ["head", "body", "groin", "legs"];
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn plural(n: f64, one: &str, few: &str, many: &str) -> String {
    format!(
        "{n} {}",
        text(&words::dispatch("words.plural", json!([n, one, few, many])).unwrap())
    )
}
fn steps(n: f64) -> String {
    plural(n, "шаг", "шага", "шагов")
}
fn zone(z: &str) -> &'static str {
    match z {
        "head" => "голову",
        "groin" => "пах",
        "legs" => "ноги",
        _ => "корпус",
    }
}
fn label(z: &str) -> &'static str {
    match z {
        "head" => "Голова",
        "groin" => "Пах",
        "legs" => "Ноги",
        _ => "Корпус",
    }
}
fn weapon_acc(w: &str) -> String {
    text(&data()["GUN_ACC"][w]).to_owned()
}
fn turn_data(d: &Value, p: &str) -> String {
    format!("t:{}:{}:{p}", text(&d["id"]), num(&d["match"]["turn"]))
}
fn btn(t: impl Into<String>, d: impl Into<String>) -> Value {
    json!({"text":t.into(),"data":d.into()})
}
fn cmd(d: &Value, t: impl Into<String>, c: Value) -> Value {
    btn(
        t,
        turn_data(
            d,
            text(&server::dispatch("server.encodeCommand", json!([c])).unwrap()),
        ),
    )
}
fn menu() -> Value {
    json!([[
        btn("⚔️ Бой с ботом", "m:bot"),
        btn("🤝 Дуэль с другом", "m:duel")
    ]])
}
fn move_words(m: f64) -> String {
    if m == 0.0 {
        "стоять".into()
    } else {
        format!(
            "{} на {}",
            if m > 0.0 {
                "вперёд"
            } else {
                "назад"
            },
            steps(m.abs())
        )
    }
}
fn plan(d: &Value, side: usize, draft: &Value) -> Result<String, String> {
    let m = num(&draft["move"]);
    let kind = text(&draft["kind"]);
    let chance = core("draftChance", json!([d["match"], side, draft]))?;
    let chance = if chance.is_null() {
        String::new()
    } else {
        format!(" · попадание ~{}%", (num(&chance) * 100.0 + 0.5).floor())
    };
    Ok(match kind {
        "shoot" | "knife" => {
            let name = if kind == "shoot" {
                "выстрел"
            } else {
                "нож"
            };
            if draft["zone"].is_null() {
                format!("{name}, {}", move_words(m))
            } else {
                format!(
                    "{name} в {}, {}{chance}",
                    zone(text(&draft["zone"])),
                    move_words(m)
                )
            }
        }
        "grenade" => format!("граната, {}", move_words(m)),
        "swap" => {
            let next = core("swapWeapon", json!([d["match"], side]))?;
            format!(
                "взять {}, без выстрела, {}",
                if next.is_null() {
                    "другое оружие".into()
                } else {
                    weapon_acc(text(&next))
                },
                move_words(m)
            )
        }
        "run" => format!(
            "перебежка {} на 2 шага, без выстрела",
            if m > 0.0 {
                "вперёд"
            } else {
                "назад"
            }
        ),
        "dash" => {
            "рывок вперёд на 3 шага: без выстрела, щит на одну зону, попасть по тебе труднее".into()
        }
        "flee" => "уйти из боя (половина добычи остаётся у тебя)".into(),
        _ => String::new(),
    })
}
fn turn_screen(d: &Value, side: usize) -> Result<Value, String> {
    let m = &d["match"];
    let me = &m["fighters"][side];
    let foe = &m["fighters"][1 - side];
    let draft = &d["drafts"][side];
    let constants = game_core::constants();
    let conditions = format!(
        "{} · {}, {}",
        text(&m["map"]["name"]),
        text(&constants["TIME_NAMES"][text(&m["config"]["time"])]),
        text(&constants["WEATHER"][text(&m["config"]["weather"])]["name"])
    );
    let hp = |s: usize| {
        format!(
            "{}/{}",
            num(&m["fighters"][s]["hp"]),
            num(&m["fighters"][s]["maxHp"])
        )
    };
    let defend = draft["defend"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let shields = if defend.is_empty() {
        "—".into()
    } else {
        defend
            .iter()
            .map(|z| text(&constants["ZONE_NAMES"][text(z)]))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut lines = vec![
        format!(
            "<b>Ход {} из {}</b> · {} до врага",
            num(&m["turn"]),
            num(&core("turnLimit", json!([m]))?),
            steps(num(&m["distance"]))
        ),
        format!("<i>{}</i>", esc(&conditions)),
        String::new(),
        format!(
            "<b>Ты</b> · {} · {}",
            text(&constants["WEAPONS"][text(&me["weapon"])]["name"]),
            hp(side)
        ),
        format!(
            "<b>{}</b> · {} · {}",
            esc(text(&foe["name"])),
            text(&constants["WEAPONS"][text(&foe["weapon"])]["name"]),
            hp(1 - side)
        ),
        String::new(),
        format!("Атака: {}", plan(d, side, draft)?),
        format!(
            "Щит: {shields} ({}/{})",
            defend.len(),
            num(&core("guardsNeeded", json!([draft]))?)
        ),
    ];
    if num(&m["distance"]) >= num(&core("distanceLimit", json!([m]))?) {
        lines.push(String::new());
        lines.push("<i>Ты у края карты: можно уйти из боя.</i>".into());
    }
    if !d["orders"][side].is_null() {
        lines.push(String::new());
        lines.push("✅ Приказ отдан. Ждём соперника.".into());
        return Ok(json!({"text":lines.join("\n"),"buttons":[]}));
    }
    let mut attack = vec![btn("🎯", turn_data(d, "x"))];
    let mut guard = vec![btn("🛡", turn_data(d, "x"))];
    let kind = text(&draft["kind"]);
    let mv = num(&draft["move"]);
    let mark = |on: bool, s: &str| if on { format!("● {s}") } else { s.to_owned() };
    for z in ZONES {
        attack.push(cmd(
            d,
            mark(
                draft["zone"] == z && ["shoot", "knife"].contains(&kind),
                label(z),
            ),
            json!({"type":"zone","zone":z}),
        ));
        guard.push(cmd(
            d,
            if defend.iter().any(|v| v == z) {
                format!("✓ {}", label(z))
            } else {
                label(z).into()
            },
            json!({"type":"guard","zone":z}),
        ));
    }
    let with_shot = ["shoot", "knife", "grenade", "swap"].contains(&kind);
    let step = [(-1, "← Шаг назад"), (0, "Стоять"), (1, "Шаг вперёд →")]
        .iter()
        .map(|(n, t)| {
            cmd(
                d,
                mark(with_shot && mv == *n as f64, t),
                json!({"type":"step","move":n}),
            )
        })
        .collect::<Vec<_>>();
    let run = vec![
        cmd(
            d,
            mark(kind == "run" && mv < 0.0, "⇇ Назад 2"),
            json!({"type":"run","move":-2}),
        ),
        cmd(
            d,
            mark(kind == "run" && mv > 0.0, "Вперёд 2 ⇉"),
            json!({"type":"run","move":2}),
        ),
        cmd(d, mark(kind == "dash", "Рывок 3 ⇶"), json!({"type":"dash"})),
    ];
    let mut extra = vec![];
    if num(&m["distance"]) <= 3.0 {
        extra.push(cmd(
            d,
            mark(kind == "knife", "🔪 Нож"),
            json!({"type":"knife"}),
        ));
    }
    if truth(&core("canThrowGrenade", json!([m, side]))?) {
        extra.push(cmd(
            d,
            mark(kind == "grenade", "💣 Граната"),
            json!({"type":"grenade"}),
        ));
    }
    let spare = core("swapWeapon", json!([m, side]))?;
    if !spare.is_null() {
        extra.push(cmd(
            d,
            mark(
                kind == "swap",
                &format!("🔁 Взять {}", weapon_acc(text(&spare))),
            ),
            json!({"type":"swap"}),
        ));
    }
    if truth(&core("canFlee", json!([m]))?) {
        extra.push(cmd(
            d,
            mark(kind == "flee", "🚪 Уйти из боя"),
            json!({"type":"flee"}),
        ));
    }
    let gap = core("missing", json!([draft]))?;
    let mut rows = vec![attack, guard, step, run];
    if !extra.is_empty() {
        rows.push(extra);
    }
    rows.push(vec![btn(
        if gap.is_null() {
            "✅ Готово"
        } else {
            text(&gap)
        },
        turn_data(d, "go"),
    )]);
    Ok(json!({"text":lines.join("\n"),"buttons":rows}))
}
fn attack_words(a: &Value, r: &Value, m: f64) -> String {
    let step = if m > 0.0 {
        ", шаг вперёд"
    } else if m < 0.0 {
        ", шаг назад"
    } else {
        ""
    };
    match text(&a["type"]) {
        "run" => {
            return format!(
                "перебежка {}",
                if m > 0.0 {
                    "вперёд"
                } else {
                    "назад"
                }
            )
        }
        "dash" => return "рывок вперёд".into(),
        "flee" => return "уходит из боя".into(),
        "swap" => return format!("смена оружия{step}"),
        "wait" => return "пропустил ход".into(),
        _ => {}
    }
    if r.is_null() {
        return "без атаки".into();
    }
    if r["weapon"] == "grenade" {
        return if truth(&r["hit"]) {
            format!("граната{step}: −{}", num(&r["damage"]))
        } else {
            format!("граната{step}: мимо, цель сдвинулась")
        };
    }
    let what = format!(
        "{} в {}{step}",
        if r["weapon"] == "knife" {
            "нож"
        } else {
            "выстрел"
        },
        zone(text(&r["zone"]))
    );
    if !truth(&r["hit"]) {
        return format!(
            "{what}: мимо ({}%)",
            (num(&r["chance"]) * 100.0 + 0.5).floor()
        );
    }
    let mut notes: Vec<String> = vec![];
    if truth(&r["crit"]) {
        notes.push("крит".into());
    }
    if truth(&r["blocked"]) {
        notes.push(
            if truth(&r["shieldBroken"]) {
                "щит пробит"
            } else {
                "щит принял"
            }
            .into(),
        );
    }
    for h in r["splash"].as_array().into_iter().flatten() {
        notes.push(format!(
            "дробь в {} −{}",
            zone(text(&h["zone"])),
            num(&h["damage"])
        ));
    }
    format!(
        "{what}: −{}{}",
        num(&r["damage"]),
        if notes.is_empty() {
            String::new()
        } else {
            format!(" ({})", notes.join(", "))
        }
    )
}
fn report(d: &Value, side: usize, r: &Value) -> String {
    let foe = 1 - side;
    let name = esc(text(&d["players"][foe]["name"]));
    let distance = if r["distanceBefore"] == r["distanceAfter"] {
        steps(num(&r["distanceAfter"]))
    } else {
        format!(
            "{} → {}",
            num(&r["distanceBefore"]),
            steps(num(&r["distanceAfter"]))
        )
    };
    format!(
        "<b>Ход {}</b> · {distance}\nТы: {}\n{name}: {}\nТы {} · {name} {}",
        num(&r["turn"]),
        attack_words(
            &r["actions"][side],
            &r["attacks"][side],
            num(&r["moves"][side])
        ),
        attack_words(
            &r["actions"][foe],
            &r["attacks"][foe],
            num(&r["moves"][foe])
        ),
        num(&r["hp"][side]),
        num(&r["hp"][foe])
    )
}
fn final_screen(d: &Value, side: usize) -> Value {
    let result = &d["match"]["result"];
    let reason = text(&result["reason"]);
    let foe = esc(text(&d["players"][1 - side]["name"]));
    let winner = result["winner"].as_u64();
    let line = if winner.is_none() {
        match reason {
            "double_ko" => "<b>Ничья.</b> Обе Персоны выведены из строя одновременно.",
            "fled" => "<b>Ничья.</b> Оба ушли из боя.",
            _ => "<b>Ничья.</b> Время вышло, здоровья поровну.",
        }
        .into()
    } else if winner == Some(side as u64) {
        let why = match reason {
            "fled" => format!("{foe} ушёл из боя."),
            "afk" => format!("{foe} три хода не отвечал."),
            "turn_limit" => "Время вышло, у тебя больше здоровья.".into(),
            _ => format!(
                "{foe} выведен из строя за {}.",
                plural(num(&d["match"]["turn"]), "ход", "хода", "ходов")
            ),
        };
        format!("<b>Цель подавлена.</b> {why} Добыча твоя.")
    } else {
        let why = match reason {
            "fled" => "Ты ушёл из боя и сохранил половину добычи.",
            "afk" => "Ты три хода не отдавал приказ.",
            "turn_limit" => "Время вышло, у соперника больше здоровья.",
            _ => "Персона уничтожена. Ты очнёшься в капсуле на базе «Соль».",
        };
        format!("<b>Связь потеряна.</b> {why}")
    };
    json!({"text":line,"buttons":menu()})
}
fn picker(mode: &str, veteran: bool) -> Value {
    let hint = |w: &str| match w {
        "pistol" => "3–5",
        "shotgun" => "1–3",
        "rifle" => "5–8",
        _ => "7+",
    };
    let row = |weapons: [&str; 2]| {
        weapons
            .iter()
            .map(|w| {
                btn(
                    format!(
                        "{} · {}",
                        text(&game_core::constants()["WEAPONS"][*w]["name"]),
                        hint(w)
                    ),
                    format!("w:{mode}:{w}{}", if veteran { ":v" } else { "" }),
                )
            })
            .collect::<Vec<_>>()
    };
    json!([
        row(["pistol", "shotgun"]),
        row(["rifle", "sniper"]),
        vec![btn(
            if veteran {
                "● 🎖 Набор ветерана"
            } else {
                "🎖 Набор ветерана"
            },
            format!("wk:{mode}:{}", if veteran { "s" } else { "v" })
        )]
    ])
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method{
    "server.view.data"=>data()["SERVER_TEXT"].clone(),"server.view.escapeHtml"=>json!(esc(text(&args[0]))),"server.view.steps"=>json!(steps(num(&args[0]))),"server.view.turnData"=>json!(turn_data(&args[0],text(&args[1]))),
    "server.view.turnScreen"=>return turn_screen(&args[0],num(&args[1])as usize),
    "server.view.turnReport"=>json!(report(&args[0],num(&args[1])as usize,&args[2])),"server.view.finalScreen"=>final_screen(&args[0],num(&args[1])as usize),
    "server.view.contractButtons"=>{let next=text(&args[0]);json!([[btn("✍️ Подписать",if next.is_empty(){"c".into()}else{format!("c:{next}")})]])},
    "server.view.signedText"=>json!(format!("Контракт подписан. Твой жетон: <b>{}</b>.\nЭто твой номер в игре: по нему тебя найдут и в боте, и в клиенте.",text(&args[0]))),
    "server.view.tagText"=>json!(format!("Жетон <b>{}</b>, позывной <b>{}</b>.",text(&args[0]),esc(text(&args[1])))),
    "server.view.nameWithTag"=>{let p=&args[0];json!(format!("<b>{}</b>{}",esc(text(&p["name"])),if text(&p["tag"]).is_empty(){String::new()}else{format!(" ({})",text(&p["tag"]))}))},
    "server.view.menuButtons"=>menu(),"server.view.weaponPicker"=>picker(text(&args[0]),truth(&args[1])),
    _=>return Err(format!("Unknown server view operation: {method}")),
})
}
