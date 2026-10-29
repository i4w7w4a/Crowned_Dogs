use crate::combat as c;
use crate::rng::State;
use crate::*;
const PRIOR: [f64; 4] = [3.0, 3.0, 2.0, 2.0];
pub fn bot_rng(m: &Value, side: usize) -> State {
    rng::seed(&format!(
        "{}:bot:{side}:{}",
        s(&m["config"]["seed"]),
        n(&m["turn"])
    ))
}
fn guard_factor(count: usize, guard: f64) -> f64 {
    1.0 - (count as f64 / 4.0) * (1.0 - guard)
}
fn guess_defence(m: &Value, enemy: usize) -> [f64; 4] {
    let history = a(&m["history"]);
    std::array::from_fn(|i| {
        let seen = history
            .iter()
            .filter(|t| contains(&t["defend"][enemy], ZONES[i]))
            .count();
        (seen + 1) as f64 / (history.len() + 2) as f64
    })
}
fn guess_aim(m: &Value, enemy: usize) -> [f64; 4] {
    let mut weights = PRIOR;
    for t in a(&m["history"]) {
        let action = &t["actions"][enemy];
        if ["shoot", "knife"].contains(&s(&action["type"])) {
            if let Some(i) = ZONES.iter().position(|z| *z == s(&action["zone"])) {
                weights[i] += 2.0;
            }
        }
    }
    weights
}
fn guess_move(m: &Value, enemy: usize) -> f64 {
    let history = a(&m["history"]);
    let count = history.len().min(2);
    if count == 0 {
        return 0.0;
    }
    let total = history[history.len() - count..]
        .iter()
        .map(|t| n(&t["moves"][enemy]))
        .sum::<f64>();
    clamp(round(total / count as f64), -1.0, 1.0)
}
fn zone_value(m: &Value, side: usize, w: &str, z: &str, d: f64, pdef: f64) -> f64 {
    c::expected(m, side, w, z, d) * (1.0 - pdef * (1.0 - c::guard(m, side, w)))
}
fn best_zone(m: &Value, side: usize, w: &str, d: f64, defence: &[f64; 4]) -> f64 {
    ZONES
        .iter()
        .enumerate()
        .map(|(i, z)| zone_value(m, side, w, z, d, defence[i]))
        .fold(f64::NEG_INFINITY, f64::max)
}
fn shot_threat(m: &Value, side: usize, d: f64, still: f64, dashing: bool, guards: usize) -> f64 {
    let weapon = s(&m["fighters"][side]["weapon"]);
    let p = c::hit_chance(m, side, weapon, d, still, dashing);
    if p <= 0.0 {
        return 0.0;
    }
    let mut total = 0.0;
    let mut weight = 0.0;
    for (i, z) in ZONES.iter().enumerate() {
        total += PRIOR[i] * c::expected(m, side, weapon, z, d);
        weight += PRIOR[i];
    }
    let mut threat = p * (total / weight) * guard_factor(guards, c::guard(m, side, weapon));
    if d <= 2.0 {
        let knife = c::hit_chance(m, side, "knife", d, still, dashing)
            * c::expected(m, side, "knife", "body", d)
            * guard_factor(guards, c::guard(m, side, "knife"));
        threat = threat.max(knife);
    }
    threat
}
fn advantage(m: &Value, side: usize, d: f64) -> f64 {
    let dd = clamp(d, 1.0, 16.0);
    shot_threat(
        m,
        side,
        dd,
        (n(&m["fighters"][side]["still"]) + 1.0).min(2.0),
        false,
        2,
    ) - shot_threat(
        m,
        1 - side,
        dd,
        (n(&m["fighters"][1 - side]["still"]) + 1.0).min(2.0),
        false,
        2,
    )
}
fn grenade_value(m: &Value, side: usize, pstand: f64) -> f64 {
    let g = c::grenade(m, side);
    let hurt = c::grenade_hit(m, side, 0.0) * pstand;
    match s(&g["type"]) {
        "emp" => hurt + 8.0 * pstand,
        "smoke" => 0.5 * shot_threat(m, 1 - side, n(&m["distance"]), 2.0, false, 2),
        _ => hurt,
    }
}
fn candidates(m: &Value, side: usize, style: &Value) -> Vec<(Value, f64)> {
    let me = &m["fighters"][side];
    let foe = &m["fighters"][1 - side];
    let defence = guess_defence(m, 1 - side);
    let foe_still = (n(&foe["still"]) + 1.0).min(2.0);
    let caution = n(
        &style[if n(&me["hp"]) / n(&me["maxHp"]) < n(&foe["hp"]) / n(&foe["maxHp"]) {
            "cautionBehind"
        } else {
            "caution"
        }],
    );
    let history = a(&m["history"]);
    let stood = history
        .iter()
        .filter(|t| n(&t["moves"][1 - side]) == 0.0)
        .count();
    let pstand = (stood + 1) as f64 / (history.len() + 2) as f64;
    let foe_move = guess_move(m, 1 - side);
    let limit = c::limit(m, n(&m["turn"]));
    let mut list = vec![];
    for mv in [-1, 0, 1] {
        list.push(json!({"type":"shoot","move":mv,"zone":"body"}));
        if c::can_knife(m, mv as f64) {
            list.push(json!({"type":"knife","move":mv,"zone":"body"}));
        }
        if c::can_grenade(m, side) {
            list.push(json!({"type":"grenade","move":mv}));
        }
        if !c::swap(m, side).is_null() {
            list.push(json!({"type":"swap","move":mv}));
        }
    }
    list.extend([
        json!({"type":"run","move":2}),
        json!({"type":"run","move":-2}),
        json!({"type":"dash"}),
    ]);
    list.into_iter()
        .filter(|a| s(&a["type"]) != "dash" || n(&m["distance"]) > 1.0)
        .map(|action| {
            let kind = s(&action["type"]);
            let mv = c::move_of(&action);
            let d = clamp(n(&m["distance"]) - mv - foe_move, 1.0, limit);
            let still = if mv == 0.0 {
                (n(&me["still"]) + 1.0).min(2.0)
            } else {
                0.0
            };
            let deal = match kind {
                "shoot" => {
                    c::hit_chance(m, side, s(&me["weapon"]), d, still, false)
                        * best_zone(m, side, s(&me["weapon"]), d, &defence)
                }
                "knife" => {
                    c::hit_chance(m, side, "knife", d, still, false)
                        * best_zone(m, side, "knife", d, &defence)
                }
                "grenade" => grenade_value(m, side, pstand),
                _ => 0.0,
            };
            let take = shot_threat(
                m,
                1 - side,
                d,
                foe_still,
                kind == "dash",
                c::defend_count(&action),
            );
            let mut next = m.clone();
            if kind == "swap" {
                next["fighters"][side]["weapon"] = c::swap(m, side);
                next["fighters"][side]["sidearm"] = json!(!b(&me["sidearm"]));
            }
            if kind == "dash" {
                next["fighters"][side]["winded"] = json!(true);
            }
            let future = advantage(&next, side, d - 1.0)
                .max(advantage(&next, side, d))
                .max(advantage(&next, side, d + 1.0));
            (
                action,
                deal - caution * take + n(&style["foresight"]) * future,
            )
        })
        .collect()
}
fn soft_pick(rng: &mut State, scores: &[f64], temperature: f64) -> usize {
    let top = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let weights: Vec<f64> = scores
        .iter()
        .map(|score| ((score - top) / temperature).exp())
        .collect();
    let mut r = rng::next(rng) * weights.iter().sum::<f64>();
    for (i, w) in weights.iter().enumerate() {
        r -= w;
        if r <= 0.0 {
            return i;
        }
    }
    scores.len() - 1
}
fn pick_zone(rng: &mut State, m: &Value, side: usize, w: &str, d: f64) -> &'static str {
    let defence = guess_defence(m, 1 - side);
    let scores: Vec<_> = ZONES
        .iter()
        .enumerate()
        .map(|(i, z)| zone_value(m, side, w, z, d, defence[i]))
        .collect();
    ZONES[soft_pick(rng, &scores, 4.0)]
}
fn pick_defence(rng: &mut State, m: &Value, side: usize, count: usize) -> Value {
    let aim = guess_aim(m, 1 - side);
    let me = &m["fighters"][side];
    let mut weights = [0.0; 4];
    for (i, z) in ZONES.iter().enumerate() {
        weights[i] = (aim[i]
            * c::expected(
                m,
                1 - side,
                s(&m["fighters"][1 - side]["weapon"]),
                z,
                n(&m["distance"]),
            )
            / (n(&me["armor"][*z]) + 20.0))
            .powf(1.5);
    }
    let mut out = vec![];
    while out.len() < count {
        let left: Vec<_> = (0..4).filter(|i| !out.contains(i)).collect();
        let mut r = rng::next(rng) * left.iter().map(|i| weights[*i]).sum::<f64>();
        let mut chosen = *left.last().unwrap();
        for i in left {
            r -= weights[i];
            if r <= 0.0 {
                chosen = i;
                break;
            }
        }
        out.push(chosen);
    }
    json!(out.into_iter().map(|i| ZONES[i]).collect::<Vec<_>>())
}
pub fn order(m: &Value, side: usize, rng: &mut State, style: &Value) -> Value {
    let me = &m["fighters"][side];
    let foe = &m["fighters"][1 - side];
    if c::can_flee(m)
        && n(&me["hp"]) / n(&me["maxHp"]) < 0.2
        && n(&foe["hp"]) / n(&foe["maxHp"]) > 0.6
        && rng::next(rng) < 0.5
    {
        return json!({"action":{"type":"flee"},"defend":pick_defence(rng,m,side,2)});
    }
    let choices = candidates(m, side, style);
    let scores: Vec<_> = choices.iter().map(|(_, s)| *s).collect();
    let mut action = choices[soft_pick(rng, &scores, n(&style["temperature"]))]
        .0
        .clone();
    if ["shoot", "knife"].contains(&s(&action["type"])) {
        let w = if s(&action["type"]) == "knife" {
            "knife"
        } else {
            s(&me["weapon"])
        };
        let d = clamp(
            c::planned(m, n(&action["move"])) - guess_move(m, 1 - side),
            1.0,
            c::limit(m, n(&m["turn"])),
        );
        action["zone"] = json!(pick_zone(rng, m, side, w, d));
    }
    let order = json!({"defend":pick_defence(rng,m,side,c::defend_count(&action)),"action":action});
    if !c::validate(m, side, &order).is_null() {
        return json!({"action":{"type":"shoot","move":0,"zone":"body"},"defend":pick_defence(rng,m,side,2)});
    }
    order
}
