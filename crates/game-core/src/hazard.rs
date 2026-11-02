//! The living world's rules, on when the match config says `events`: each map's own weather
//! (names and numbers from `MAP_WEATHER`), what a roof changes, and the hazards a turn brings —
//! lightning, a stray kamikaze drone, shelling, a roof collapse, a blackout. A hazard is warned
//! a turn ahead, so dodging it is a choice. Lightning, shells and collapses strike a stretch of
//! the lane, so each fighter keeps a lane position (`lane`, fighter 0 faces +x).
use crate::rng::{self, State};
use crate::*;

pub fn on(m: &Value) -> bool {
    b(&m["config"]["events"])
}

/// "open", "roof" (hall, depot) or "gallery" (a roof with one open side).
pub fn cover(m: &Value) -> &'static str {
    match s(&constants()["MAP_WEATHER"][s(&m["config"]["mapId"])]["cover"]) {
        "roof" => "roof",
        "gallery" => "gallery",
        _ => "open",
    }
}

/// Under a roof there is no heat shimmer, and a shot without a suppressor echoes and gives the
/// shooter away at any hour, as the muzzle flash does at night.
pub fn covered(m: &Value) -> bool {
    on(m) && cover(m) != "open"
}

/// The weather's rules on this map: the catalogue weather and the map's own overrides.
pub fn weather(m: &Value) -> Value {
    let kind = s(&m["config"]["weather"]);
    let mut w = constants()["WEATHER"][kind].clone();
    if on(m) {
        merge(
            &mut w,
            &constants()["MAP_WEATHER"][s(&m["config"]["mapId"])]["weather"][kind],
        );
    }
    w
}

/// Lightning struck the roof last turn: the lights are out for this one, as at night.
pub fn dark(m: &Value) -> bool {
    on(m) && s(&m["hazard"]["kind"]) == "blackout" && n(&m["hazard"]["turn"]) == n(&m["turn"])
}

pub fn lane(m: &Value) -> [f64; 2] {
    [n(&m["lane"][0]), n(&m["lane"][1])]
}

/// How far each fighter really went when the range was clamped: running into the
/// opponent cuts the advances, the distance limit cuts the retreats, and when the
/// limit shrinks under both of them the rest closes in from both sides.
pub fn travel(before: f64, moves: [f64; 2], after: f64) -> [f64; 2] {
    let mut t = moves;
    let wanted = before - t[0] - t[1];
    if wanted < after {
        let excess = after - wanted;
        let ahead: f64 = t.iter().filter(|m| **m > 0.0).sum();
        for m in t.iter_mut().filter(|m| **m > 0.0) {
            *m -= excess * *m / ahead;
        }
    } else if wanted > after {
        let excess = wanted - after;
        let back: f64 = t.iter().filter(|m| **m < 0.0).map(|m| -m).sum();
        let cut = excess.min(back);
        if back > 0.0 {
            for m in t.iter_mut().filter(|m| **m < 0.0) {
                *m += cut * -*m / back;
            }
        }
        let rest = (excess - cut) / 2.0;
        t[0] += rest;
        t[1] += rest;
    }
    t
}

fn rng_for(m: &Value, turn: f64, what: &str) -> State {
    let seed = &m["config"]["seed"];
    let seed = seed
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| seed.to_string());
    rng::seed(&format!("{seed}:hazard:{what}:{turn}"))
}

/// Whether a lane position is inside a warned stretch. Stretches cover whole steps: radius 0
/// is one step, radius 1 the step and both neighbours.
pub fn inside(h: &Value, x: f64) -> bool {
    !h["at"].is_null() && (x - n(&h["at"])).abs() < n(&h["radius"]) + 0.5
}

/// The warning for next turn, or null. At most one hazard a turn; shelling comes on top of the
/// weather's own odds and is far likelier while the map is under fire.
pub fn warn(m: &Value) -> Value {
    let turn = n(&m["turn"]) + 1.0;
    let mut rng = rng_for(m, turn, "warn");
    let mut odds: Vec<(String, f64)> = weather(m)["hazards"]
        .as_object()
        .map(|o| o.iter().map(|(k, v)| (k.clone(), n(v))).collect())
        .unwrap_or_default();
    let shelling = &constants()["SHELLING"];
    let fire = n(&shelling[if b(&m["config"]["shelled"]) {
        "underFire"
    } else {
        "chance"
    }]);
    // A shell that lands on a roof brings the ceiling down instead.
    let shell = if cover(m) == "roof" { "collapse" } else { "shell" };
    match odds.iter_mut().find(|(k, _)| k == shell) {
        Some(o) => o.1 += fire,
        None => odds.push((shell.to_string(), fire)),
    }
    let mut r = rng::next(&mut rng);
    let Some(kind) = odds.iter().find_map(|(k, p)| {
        r -= p;
        (r < 0.0).then(|| k.clone())
    }) else {
        return Value::Null;
    };
    let spec = &constants()["HAZARDS"][&kind];
    let [x0, x1] = lane(m);
    let at = match kind.as_str() {
        // Anywhere along the stretch the fighters hold, a step past either of them included.
        "lightning" => Some((x0 - 1.0 + (rng::next(&mut rng) * (x1 - x0 + 3.0)).floor()).round()),
        // Artillery brackets one of the fighters.
        "shell" => Some(lane(m)[(rng::next(&mut rng) * 2.0) as usize].round()),
        // The ceiling gives way somewhere over the stretch between them.
        "collapse" => Some((x0 + (rng::next(&mut rng) * (x1 - x0 + 1.0)).floor()).round()),
        _ => None,
    };
    json!({"kind":kind,"name":spec["name"],"turn":turn,"at":at,"radius":spec["radius"]})
}

/// The warned hazard strikes after this turn's moves and shots. Returns what happened and
/// leaves the damage, the stun and the shield on the fighters.
pub fn strike(m: &mut Value, moves: [f64; 2]) -> Value {
    let mut report = Value::Null;
    // The arena shows the shots' damage first and the hazard's after it.
    let before = [n(&m["fighters"][0]["hp"]), n(&m["fighters"][1]["hp"])];
    let h = m["hazard"].clone();
    if n(&h["turn"]) == n(&m["turn"]) && !h["kind"].is_null() {
        let kind = s(&h["kind"]).to_string();
        let spec = &constants()["HAZARDS"][&kind];
        let lane = lane(m);
        let mut hits = vec![];
        let mut hit = |side: usize, share: f64| {
            hits.push(json!({"side":side,"damage":(n(&spec["damage"])*share).floor(),
                "stun":n(&spec["stun"])>0.0 && share>=1.0,"shieldOff":n(&spec["shieldOff"])>0.0 && share>=1.0}));
        };
        match kind.as_str() {
            "drone" => {
                // The drone dives on whoever stands still; on both, it picks one.
                let still: Vec<usize> = (0..2).filter(|&i| moves[i] == 0.0).collect();
                if !still.is_empty() {
                    let mut rng = rng_for(m, n(&m["turn"]), "drone");
                    hit(still[(rng::next(&mut rng) * still.len() as f64) as usize], 1.0);
                }
            }
            "blackout" => {}
            _ => {
                for (side, x) in lane.iter().enumerate() {
                    if inside(&h, *x) {
                        // A shell's edge takes part of the blast.
                        let edge = n(&spec["edge"]);
                        let share = if edge > 0.0 && (x - n(&h["at"])).abs() >= 0.5 {
                            edge
                        } else {
                            1.0
                        };
                        hit(side, share);
                    }
                }
            }
        }
        for hh in &hits {
            let f = &mut m["fighters"][n(&hh["side"]) as usize];
            f["hp"] = num((n(&f["hp"]) - n(&hh["damage"])).max(0.0));
            if b(&hh["stun"]) {
                f["stunned"] = json!(1);
            }
            if b(&hh["shieldOff"]) {
                f["shieldOff"] = num(n(&f["shieldOff"]).max(1.0));
            }
        }
        report = json!({"kind":kind,"name":h["name"],"at":h["at"],"radius":h["radius"],"hits":hits});
    }
    // Poisoned dust or fumes eat at both Personas every turn.
    let toxic = n(&weather(m)["toxic"]);
    if toxic > 0.0 {
        for side in 0..2 {
            let f = &mut m["fighters"][side];
            f["hp"] = num((n(&f["hp"]) - toxic).max(0.0));
        }
        if report.is_null() {
            report = json!({"kind":null,"hits":[]});
        }
        report["toxic"] = num(toxic);
    }
    if !report.is_null() {
        report["hpBefore"] = json!([num(before[0]), num(before[1])]);
    }
    report
}

/// A bot that sees the warning steps out of it more often than not: it keeps its shot when a
/// step is enough and gives the shot up for a run only for lightning or the heart of a shell.
pub fn dodge(m: &Value, side: usize, action: &Value, rng: &mut State) -> Value {
    let h = &m["hazard"];
    if !on(m) || n(&h["turn"]) != n(&m["turn"]) || rng::next(rng) >= 0.75 {
        return action.clone();
    }
    let kind = s(&h["kind"]);
    let typed = s(&action["type"]);
    let shot = ["shoot", "knife", "grenade", "swap"].contains(&typed);
    let mv = crate::combat::move_of(action);
    if kind == "drone" {
        if mv == 0.0 && shot {
            let mut a = action.clone();
            a["move"] = json!(if n(&m["distance"]) > 4.0 { 1 } else { -1 });
            return a;
        }
        return action.clone();
    }
    if h["at"].is_null() {
        return action.clone();
    }
    let x = lane(m)[side];
    let dir = if side == 0 { 1.0 } else { -1.0 };
    let safe = |step: f64| !inside(h, x + dir * step);
    if safe(mv) {
        return action.clone();
    }
    if shot {
        for step in [1.0, -1.0, 0.0] {
            if safe(step) {
                let mut a = action.clone();
                a["move"] = json!(step);
                return a;
            }
        }
    }
    let heavy = kind == "lightning" || (kind == "shell" && (x - n(&h["at"])).abs() < 0.5);
    if heavy {
        for (step, a) in [
            (-2.0, json!({"type":"run","move":-2})),
            (2.0, json!({"type":"run","move":2})),
            (3.0, json!({"type":"dash"})),
        ] {
            if safe(step) {
                return a;
            }
        }
    }
    action.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamped_moves_split_like_the_camera() {
        // Two rushes into each other share the last gap.
        assert_eq!(travel(2.0, [3.0, 3.0], 1.0), [0.5, 0.5]);
        // A retreat is honoured; only the advance stops at the opponent.
        assert_eq!(travel(2.0, [3.0, -1.0], 1.0), [2.0, -1.0]);
        // Retreats stop at the limit.
        assert_eq!(travel(10.0, [-2.0, 0.0], 11.0), [-1.0, 0.0]);
        // A shrinking limit first eats the retreat, then closes in from both sides.
        assert_eq!(travel(8.0, [1.0, -1.0], 7.0), [1.0, 0.0]);
        assert_eq!(travel(8.0, [0.0, 0.0], 7.0), [0.5, 0.5]);
    }
}
