use crate::combat as c;
use crate::*;
pub fn fresh(m: &Value, side: usize) -> Value {
    json!({"kind":"shoot","move":0,"zone":null,"defend":a(&m["fighters"][side]["lastDefend"]).iter().take(2).cloned().collect::<Vec<_>>()})
}
pub fn grenade_refusal(m: &Value, side: usize) -> String {
    if n(&m["fighters"][side]["grenades"]) <= 0.0 {
        return "Гранаты больше нет".into();
    }
    let g = c::grenade(m, side);
    format!(
        "Граната летит на {}–{} шагов",
        n(&g["minDistance"]),
        n(&g["maxDistance"])
    )
}
pub fn guards(d: &Value) -> usize {
    if s(&d["kind"]) == "dash" {
        1
    } else {
        2
    }
}
pub fn apply(m: &Value, side: usize, d: &Value, command: &Value) -> Value {
    let mut next = d.clone();
    let kind = s(&d["kind"]);
    match s(&command["type"]) {
        "zone" => {
            next["zone"] = if d["zone"] == command["zone"] && (kind == "shoot" || kind == "knife") {
                Value::Null
            } else {
                command["zone"].clone()
            };
            if kind != "knife" {
                next["kind"] = json!("shoot");
            }
            if n(&next["move"]).abs() > 1.0 {
                next["move"] = json!(0);
            }
        }
        "guard" => {
            let mut defend = a(&next["defend"]).to_vec();
            if let Some(i) = defend.iter().position(|v| v == &command["zone"]) {
                defend.remove(i);
            } else {
                defend.push(command["zone"].clone());
            }
            next["defend"] = json!(defend);
        }
        "step" => {
            if ["run", "dash", "flee"].contains(&kind) {
                next["kind"] = json!("shoot");
            }
            next["move"] = command["move"].clone();
            if s(&next["kind"]) == "knife" && !c::can_knife(m, n(&next["move"])) {
                return json!("С такого шага нож не достанет");
            }
        }
        "run" => {
            let run = !(kind == "run" && n(&d["move"]) == n(&command["move"]));
            next["kind"] = json!(if run { "run" } else { "shoot" });
            next["move"] = if run {
                command["move"].clone()
            } else {
                json!(0)
            };
        }
        "dash" => {
            let dash = kind != "dash";
            next["kind"] = json!(if dash { "dash" } else { "shoot" });
            next["move"] = json!(if dash { 3 } else { 0 });
        }
        "knife" => {
            if kind == "knife" {
                next["kind"] = json!("shoot");
            } else {
                if n(&next["move"]).abs() > 1.0 {
                    next["move"] = json!(0);
                }
                if !c::can_knife(m, n(&next["move"])) {
                    if c::can_knife(m, 1.0) {
                        next["move"] = json!(1);
                    } else {
                        return json!("Нож бьёт только с 2 шагов");
                    }
                }
                next["kind"] = json!("knife");
            }
        }
        "grenade" => {
            if kind == "grenade" {
                next["kind"] = json!("shoot");
            } else {
                if !c::can_grenade(m, side) {
                    return json!(grenade_refusal(m, side));
                }
                next["kind"] = json!("grenade");
                if n(&next["move"]).abs() > 1.0 {
                    next["move"] = json!(0);
                }
            }
        }
        "swap" => {
            if kind == "swap" {
                next["kind"] = json!("shoot");
            } else {
                if c::swap(m, side).is_null() {
                    return json!("Второго оружия нет");
                }
                next["kind"] = json!("swap");
                if n(&next["move"]).abs() > 1.0 {
                    next["move"] = json!(0);
                }
            }
        }
        "flee" => {
            if kind == "flee" {
                next["kind"] = json!("shoot");
                next["move"] = json!(0);
            } else {
                if !c::can_flee(m) {
                    return json!("Уйти можно только с края карты");
                }
                next["kind"] = json!("flee");
                next["move"] = json!(0);
            }
        }
        _ => {}
    }
    let count = guards(&next);
    let mut defend = a(&next["defend"]).to_vec();
    if defend.len() > count {
        defend.drain(..defend.len() - count);
    }
    next["defend"] = json!(defend);
    next
}
pub fn action(d: &Value) -> Value {
    match s(&d["kind"]) {
        "shoot" | "knife" => {
            if d["zone"].is_null() {
                Value::Null
            } else {
                json!({"type":d["kind"],"move":d["move"],"zone":d["zone"]})
            }
        }
        "grenade" | "swap" => json!({"type":d["kind"],"move":d["move"]}),
        "run" => json!({"type":"run","move":if n(&d["move"])==2.0{2}else{-2}}),
        "dash" => json!({"type":"dash"}),
        "flee" => json!({"type":"flee"}),
        _ => Value::Null,
    }
}
pub fn missing(d: &Value) -> Value {
    if action(d).is_null() {
        return json!(if s(&d["kind"]) == "knife" {
            "Выбери зону для ножа"
        } else {
            "Выбери зону атаки"
        });
    }
    let need = guards(d) as isize - a(&d["defend"]).len() as isize;
    if need > 0 {
        json!(if need == 2 {
            "Выбери 2 зоны защиты"
        } else {
            "Выбери зону защиты"
        })
    } else {
        Value::Null
    }
}
pub fn order(m: &Value, side: usize, d: &Value) -> Value {
    let gap = missing(d);
    if !gap.is_null() {
        return gap;
    }
    let order = json!({"action":action(d),"defend":d["defend"]});
    let err = c::validate(m, side, &order);
    if err.is_null() {
        order
    } else {
        json!(format!("Так нельзя: {}", s(&err)))
    }
}
pub fn chance(m: &Value, side: usize, d: &Value) -> Value {
    let kind = s(&d["kind"]);
    if kind != "shoot" && kind != "knife" {
        return Value::Null;
    }
    let me = &m["fighters"][side];
    let dist = c::planned(m, n(&d["move"])).min(16.0);
    let still = if n(&d["move"]) == 0.0 {
        (n(&me["still"]) + 1.0).min(2.0)
    } else {
        0.0
    };
    num(c::hit_chance(
        m,
        side,
        if kind == "knife" {
            "knife"
        } else {
            s(&me["weapon"])
        },
        dist,
        still,
        false,
    ))
}
