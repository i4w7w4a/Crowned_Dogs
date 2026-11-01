use crate::rng::{roll, State};
use crate::*;

pub fn create(config: &Value) -> Result<Value, String> {
    let map = constants()["MAPS"][s(&config["mapId"])].clone();
    if map.is_null() {
        return Err(format!("Unknown map: {}", s(&config["mapId"])));
    }
    let mut rng = rng::seed_value(&config["seed"]);
    let sun = if s(&config["time"]) == "evening" {
        (rng::next(&mut rng) * 2.0).floor() as i32
    } else {
        -1
    };
    let fighters:Vec<_>=a(&config["fighters"]).iter().enumerate().map(|(i,f)|{
        let hp=f["hp"].as_f64().unwrap_or(100.0);let kit=&f["kit"];
        let mut armor=if kit.is_null(){constants()["DEFAULT_ARMOR"].clone()}else{kit["armor"].clone()};
        if kit.is_null(){merge(&mut armor,&f["armor"]);}
        let mut gear=if kit.is_null(){json!({})}else{json!({"nvg":kit["nvg"],"thermal":kit["thermal"],"heatProof":kit["heatProof"]})};
        merge(&mut gear,&f["gear"]);
        json!({"id":f["id"],"name":f["name"],"weapon":if kit.is_null(){f["weapon"].clone()}else{kit["weapon"]["id"].clone()},
            "armor":armor,"gear":gear,"hp":hp,"maxHp":hp,"still":1,
            "grenades":if kit.is_null(){json!(1)}else{kit["grenades"].clone()},"lastDefend":[],"afkStreak":0,
            "flash":false,"winded":false,"facesSun":sun==i as i32,"shieldOff":0,"smoke":0,"sidearm":false})
    }).collect();
    Ok(
        json!({"version":1,"config":config,"map":map,"turn":1,"distance":config["startDistance"].as_f64().unwrap_or(7.0).min(n(&map["maxDistance"])),
        "fighters":fighters,"rng":rng,"result":null,"history":[]}),
    )
}
pub fn weapon(m: &Value, side: usize, w: &str) -> Value {
    let kit = &m["config"]["fighters"][side]["kit"];
    if !kit.is_null() {
        if w == "knife" {
            return kit["knife"].clone();
        }
        if b(&m["fighters"][side]["sidearm"]) && s(&kit["sidearm"]["id"]) == w {
            return kit["sidearm"].clone();
        }
        if s(&kit["weapon"]["id"]) == w {
            return kit["weapon"].clone();
        }
        if s(&kit["sidearm"]["id"]) == w {
            return kit["sidearm"].clone();
        }
    }
    let mut out = constants()["WEAPONS"][w].clone();
    merge(
        &mut out,
        &json!({"doubleShot":0,"nightAcc":0,"tracking":0,"stanceTurns":2,"noStancePenalty":0.5,"guardBonus":0,"suppressor":false}),
    );
    out
}
pub fn swap(m: &Value, side: usize) -> Value {
    let kit = &m["config"]["fighters"][side]["kit"];
    if kit["sidearm"].is_null() {
        Value::Null
    } else if b(&m["fighters"][side]["sidearm"]) {
        kit["weapon"]["id"].clone()
    } else {
        kit["sidearm"]["id"].clone()
    }
}
pub fn grenade(m: &Value, side: usize) -> Value {
    let g = &m["config"]["fighters"][side]["kit"]["grenade"];
    if g.is_null() {
        let mut g = json!({"type":"frag"});
        merge(&mut g, &constants()["GRENADE"]);
        g
    } else {
        g.clone()
    }
}
pub fn grenade_hit(m: &Value, side: usize, mv: f64) -> f64 {
    grenade_damage(
        n(&m["fighters"][1 - side]["armor"]["body"]),
        mv,
        &grenade(m, side),
    )
}
pub fn guard(m: &Value, side: usize, w: &str) -> f64 {
    if n(&m["fighters"][1 - side]["shieldOff"]) > 0.0 {
        return 1.0;
    }
    (if w == "knife" { 0.5 } else { 0.26 }) + n(&weapon(m, side, w)["guardBonus"])
}
pub fn weather(m: &Value) -> &Value {
    &constants()["WEATHER"][s(&m["config"]["weather"])]
}
pub fn limit(m: &Value, turn: f64) -> f64 {
    let max = n(&m["map"]["maxDistance"]);
    4.0f64.min(max).max(max - (turn - 6.0).max(0.0))
}
pub fn move_of(action: &Value) -> f64 {
    match s(&action["type"]) {
        "shoot" | "knife" | "grenade" | "swap" | "run" => n(&action["move"]),
        "dash" => 3.0,
        _ => 0.0,
    }
}
pub fn defend_count(action: &Value) -> usize {
    if s(&action["type"]) == "dash" {
        1
    } else {
        2
    }
}
pub fn planned(m: &Value, mv: f64) -> f64 {
    clamp(n(&m["distance"]) - mv, 1.0, limit(m, n(&m["turn"])))
}
pub fn can_flee(m: &Value) -> bool {
    n(&m["distance"]) >= limit(m, n(&m["turn"]))
}
pub fn can_grenade(m: &Value, side: usize) -> bool {
    let g = grenade(m, side);
    let d = n(&m["distance"]);
    n(&m["fighters"][side]["grenades"]) > 0.0
        && d >= n(&g["minDistance"])
        && d <= n(&g["maxDistance"])
}
pub fn can_knife(m: &Value, mv: f64) -> bool {
    planned(m, mv) <= 2.0
}
pub fn validate(m: &Value, side: usize, order: &Value) -> Value {
    if !m["result"].is_null() {
        return json!("finished");
    }
    let action = &order["action"];
    let mv = n(&action["move"]);
    match s(&action["type"]) {
        "shoot" | "knife" | "grenade" | "swap" => {
            if ![-1.0, 0.0, 1.0].contains(&mv) || action["move"].is_null() {
                return json!("bad_move");
            }
            if let Some(zone) = action.get("zone") {
                if !ZONES.contains(&s(zone)) {
                    return json!("bad_action");
                }
            }
            if s(&action["type"]) == "knife" && !can_knife(m, mv) {
                return json!("knife_out_of_reach");
            }
            if s(&action["type"]) == "grenade" && !can_grenade(m, side) {
                return json!("grenade_unavailable");
            }
            if s(&action["type"]) == "swap" && swap(m, side).is_null() {
                return json!("swap_unavailable");
            }
        }
        "run" => {
            if mv.abs() != 2.0 {
                return json!("bad_move");
            }
        }
        "dash" => {}
        "flee" => {
            if !can_flee(m) {
                return json!("flee_unavailable");
            }
        }
        _ => return json!("bad_action"),
    }
    let defend = a(&order["defend"]);
    if defend
        .iter()
        .enumerate()
        .any(|(i, z)| !ZONES.contains(&s(z)) || defend[..i].contains(z))
        || defend.len() != defend_count(action)
    {
        return json!("bad_defence");
    }
    Value::Null
}
pub fn condition(m: &Value, side: usize, d: f64) -> f64 {
    let me = &m["fighters"][side];
    let band = band(d);
    let mut mult = n(&weather(m)["accuracy"][band]);
    if s(&m["config"]["weather"]) == "dust" && b(&me["gear"]["thermal"]) {
        mult = 1.0 - (1.0 - mult) / 2.0;
    }
    match s(&m["config"]["time"]) {
        "day" => {
            if band == "far" {
                mult *= 0.9;
            }
        }
        "evening" => {
            if b(&me["facesSun"]) {
                mult *= 0.9;
            }
        }
        "night" => {
            if d > 4.0 && !b(&me["gear"]["nvg"]) {
                mult *= 0.75;
            }
        }
        _ => {}
    };
    mult
}
pub fn hit_chance(m: &Value, side: usize, id: &str, d: f64, still: f64, dashing: bool) -> f64 {
    let w = weapon(m, side, id);
    let target = &m["config"]["fighters"][1 - side]["kit"];
    let idx = d.min(16.0) as isize - 1;
    let mut acc = if idx >= 0 {
        n(&w["accuracy"][idx as usize])
    } else {
        0.0
    };
    if acc <= 0.0 {
        return 0.0;
    }
    if s(&m["config"]["time"]) == "night" {
        acc += n(&w["nightAcc"]);
    }
    if id != "knife" {
        acc *= condition(m, side, d);
    }
    if b(&w["needsStance"]) && still < n(&w["stanceTurns"]) {
        acc *= n(&w["noStancePenalty"]);
    }
    let mut p = acc - 0.07;
    if !target.is_null() {
        p -= n(&target["dodge"])
            + if id == "knife" {
                0.0
            } else {
                n(&target["camo"][band(d)])
            };
    }
    if dashing {
        p -= (0.1 + n(&target["dashEvade"]) - n(&w["tracking"])).max(0.0);
    }
    if b(&m["fighters"][1 - side]["flash"]) {
        p += 0.1;
    }
    if b(&m["fighters"][side]["winded"]) {
        p -= 0.1;
    }
    if n(&m["fighters"][1 - side]["smoke"]) > 0.0 && id != "knife" {
        p -= if b(&m["fighters"][side]["gear"]["thermal"]) {
            0.175
        } else {
            0.35
        };
    }
    clamp(p, 0.05, 0.95)
}
pub fn crit(m: &Value, id: &str, side: Option<usize>) -> f64 {
    let w = side
        .map(|s| weapon(m, s, id))
        .unwrap_or_else(|| constants()["WEAPONS"][id].clone());
    clamp(0.07 + n(&w["crit"]) + n(&weather(m)["crit"]), 0.0, 0.9)
}
pub fn expected(m: &Value, side: usize, id: &str, zone: &str, d: f64) -> f64 {
    let w = weapon(m, side, id);
    let target = &m["fighters"][1 - side];
    let crit = 1.0 + crit(m, id, Some(side)) * (1.47 + n(&w["critBonus"]) - 1.0);
    let armor = armor_factor(n(&target["armor"][zone]), 0.04, pen(m, side, &w), 0.04);
    base_damage(&w, d)
        * n(&constants()["ZONE_MULT"][zone])
        * crit
        * armor
        * (1.0 + n(&w["doubleShot"]))
}
/// Armour piercing left after the target's Cooling (synchronisation) takes its share.
fn pen(m: &Value, side: usize, w: &Value) -> f64 {
    let pen = n(&w["flatPen"]);
    let anti = n(&m["config"]["fighters"][1 - side]["kit"]["antiPen"]);
    if pen > 0.0 && anti > 0.0 {
        (pen - anti).max(0.0)
    } else {
        pen
    }
}
fn strike(
    m: &Value,
    side: usize,
    id: &str,
    zone: &str,
    d: f64,
    defend: &Value,
    rng: &mut State,
) -> Value {
    let w = weapon(m, side, id);
    let target = &m["fighters"][1 - side];
    let mut damage = base_damage(&w, d) * n(&constants()["ZONE_MULT"][zone]);
    let critical = roll(rng, crit(m, id, Some(side)));
    if critical {
        damage *= 1.47 + n(&w["critBonus"]);
    }
    damage *= armor_factor(n(&target["armor"][zone]), 0.04, pen(m, side, &w), 0.04);
    let blocked = contains(defend, zone) && n(&target["shieldOff"]) <= 0.0;
    let mut broken = false;
    if blocked {
        let mut share = guard(m, side, id);
        if s(&m["config"]["time"]) == "day" && !b(&target["gear"]["heatProof"]) {
            share += 0.04 * (1.0 - n(&m["config"]["fighters"][1 - side]["kit"]["heatResist"]));
        }
        if roll(rng, 0.07) {
            share += (1.0 - 0.26) * 0.27;
            broken = true;
        }
        damage *= share;
    }
    json!({"damage":damage.floor(),"crit":critical,"blocked":blocked,"shieldBroken":broken})
}
/// Shotgun pellets that leave the aimed zone can still catch the zones next to it (head–body–groin–legs)
/// for part of the damage: no crit, the neighbour's armour and a shield on it still count.
fn spread(
    m: &Value,
    side: usize,
    id: &str,
    zone: &str,
    d: f64,
    defend: &Value,
    rng: &mut State,
) -> Vec<Value> {
    let sp = &constants()["SPREAD"][id];
    let Some(i) = ZONES.iter().position(|z| *z == zone) else {
        return vec![];
    };
    if sp.is_null() {
        return vec![];
    }
    let chance = (n(&sp["chance"]) + n(&sp["perStep"]) * d).min(n(&sp["max"]));
    let w = weapon(m, side, id);
    let target = &m["fighters"][1 - side];
    let mut out = vec![];
    for near in [i.checked_sub(1), Some(i + 1)]
        .into_iter()
        .flatten()
        .filter_map(|j| ZONES.get(j))
    {
        if !roll(rng, chance) {
            continue;
        }
        let mut damage = base_damage(&w, d)
            * n(&constants()["ZONE_MULT"][*near])
            * n(&sp["share"])
            * armor_factor(n(&target["armor"][*near]), 0.04, pen(m, side, &w), 0.04);
        let blocked = contains(defend, near) && n(&target["shieldOff"]) <= 0.0;
        if blocked {
            damage *= guard(m, side, id);
        }
        if damage.floor() > 0.0 {
            out.push(json!({"zone":near,"damage":damage.floor(),"blocked":blocked}));
        }
    }
    out
}
fn attack(
    m: &Value,
    side: usize,
    action: &Value,
    d: f64,
    still: f64,
    defend: &Value,
    dashing: bool,
    mv: f64,
    rng: &mut State,
) -> Value {
    let kind = s(&action["type"]);
    if kind == "grenade" {
        let g = grenade(m, side);
        let damage = if n(&g["base"]) > 0.0 {
            grenade_hit(m, side, mv)
        } else {
            0.0
        };
        let mut report = json!({"weapon":"grenade","zone":null,"chance":1,"hit":damage>0.0,"damage":damage,"crit":false,"blocked":false,"shieldBroken":false});
        if s(&g["type"]) == "smoke" {
            report["hit"] = json!(true);
            report["effect"] = json!("smoke");
        }
        if s(&g["type"]) == "emp" && mv.abs() < 2.0 {
            report["hit"] = json!(true);
            report["effect"] = json!("emp");
        }
        return report;
    }
    if kind != "shoot" && kind != "knife" {
        return Value::Null;
    }
    let id = if kind == "knife" {
        "knife"
    } else {
        s(&m["fighters"][side]["weapon"])
    };
    let zone = s(&action["zone"]);
    let chance = hit_chance(m, side, id, d, still, dashing);
    let mut report = json!({"weapon":id,"zone":zone,"chance":chance,"hit":false,"damage":0,"crit":false,"blocked":false,"shieldBroken":false});
    if chance <= 0.0 || !roll(rng, chance) {
        return report;
    }
    report["hit"] = json!(true);
    merge(&mut report, &strike(m, side, id, zone, d, defend, rng));
    let twice = n(&weapon(m, side, id)["doubleShot"]);
    if twice > 0.0 && roll(rng, twice) {
        let second = strike(m, side, id, zone, d, defend, rng);
        report["damage"] = num(n(&report["damage"]) + n(&second["damage"]));
        report["crit"] = json!(b(&report["crit"]) || b(&second["crit"]));
        report["shieldBroken"] = json!(b(&report["shieldBroken"]) || b(&second["shieldBroken"]));
        report["double"] = json!(true);
    }
    let splash = spread(m, side, id, zone, d, defend, rng);
    if !splash.is_empty() {
        let extra: f64 = splash.iter().map(|h| n(&h["damage"])).sum();
        report["damage"] = num(n(&report["damage"]) + extra);
        report["splash"] = json!(splash);
    }
    report
}
pub fn resolve(input: &Value, orders: &Value) -> Result<Value, String> {
    if !input["result"].is_null() {
        return Err("The match is already over".into());
    }
    let mut m = input.clone();
    let afk = [orders[0].is_null(), orders[1].is_null()];
    let mut final_orders = vec![];
    for side in 0..2 {
        if afk[side] {
            final_orders
                .push(json!({"action":{"type":"wait"},"defend":m["fighters"][side]["lastDefend"]}));
        } else {
            let err = validate(&m, side, &orders[side]);
            if !err.is_null() {
                return Err(format!("Invalid order for side {side}: {}", s(&err)));
            }
            final_orders.push(orders[side].clone());
        }
    }
    let moves = [
        move_of(&final_orders[0]["action"]),
        move_of(&final_orders[1]["action"]),
    ];
    let before = n(&m["distance"]);
    let lim = limit(&m, n(&m["turn"]));
    let after = clamp(before - moves[0] - moves[1], 1.0, lim);
    let still: [f64; 2] = std::array::from_fn(|i| {
        if moves[i] == 0.0 {
            (n(&m["fighters"][i]["still"]) + 1.0).min(2.0)
        } else {
            0.0
        }
    });
    let dashing: [bool; 2] =
        std::array::from_fn(|i| s(&final_orders[i]["action"]["type"]) == "dash");
    for side in 0..2 {
        if s(&final_orders[side]["action"]["type"]) == "grenade"
            && s(&grenade(&m, side)["type"]) == "smoke"
        {
            m["fighters"][side]["smoke"] = json!(2);
        }
    }
    let mut rng = rng::state(&m["rng"]);
    let mut attacks = vec![];
    for side in 0..2 {
        let t = 1 - side;
        attacks.push(attack(
            &m,
            side,
            &final_orders[side]["action"],
            after,
            still[side],
            &final_orders[t]["defend"],
            dashing[t],
            moves[t],
            &mut rng,
        ));
    }
    m["rng"] = json!(rng);
    m["distance"] = num(after);
    for side in 0..2 {
        let incoming = &attacks[1 - side];
        let mut f = m["fighters"][side].clone();
        let kind = s(&final_orders[side]["action"]["type"]);
        if !incoming.is_null() {
            f["hp"] = num((n(&f["hp"]) - n(&incoming["damage"])).max(0.0));
        }
        // Cooling (synchronisation) mends a little every turn the Persona is still standing.
        let regen = n(&m["config"]["fighters"][side]["kit"]["regen"]);
        if regen > 0.0 && n(&f["hp"]) > 0.0 {
            f["hp"] = num((n(&f["hp"]) + regen).min(n(&f["maxHp"])));
        }
        f["shieldOff"] = num(if s(&incoming["effect"]) == "emp" {
            1.0
        } else {
            (n(&f["shieldOff"]) - 1.0).max(0.0)
        });
        f["smoke"] = num((n(&f["smoke"]) - 1.0).max(0.0));
        f["still"] = num(still[side]);
        if kind == "grenade" {
            f["grenades"] = num(n(&f["grenades"]) - 1.0);
        }
        if kind == "swap" {
            f["weapon"] = swap(&m, side);
            f["sidearm"] = json!(!b(&f["sidearm"]));
        }
        f["lastDefend"] = final_orders[side]["defend"].clone();
        f["afkStreak"] = num(if afk[side] {
            n(&f["afkStreak"]) + 1.0
        } else {
            0.0
        });
        // The weapon lookup after swapping observes the updated in-hand flag.
        m["fighters"][side] = f.clone();
        let silenced =
            b(&f["gear"]["suppressor"]) || b(&weapon(&m, side, s(&f["weapon"]))["suppressor"]);
        f["flash"] = json!(s(&m["config"]["time"]) == "night" && kind == "shoot" && !silenced);
        f["winded"] = json!(kind == "dash");
        m["fighters"][side] = f;
    }
    let report = json!({"turn":m["turn"],"distanceBefore":before,"distanceAfter":after,"limit":lim,
        "actions":[final_orders[0]["action"],final_orders[1]["action"]],"defend":[final_orders[0]["defend"],final_orders[1]["defend"]],
        "moves":moves,"attacks":attacks,"hp":[m["fighters"][0]["hp"],m["fighters"][1]["hp"]],"afk":afk});
    m["history"]
        .as_array_mut()
        .ok_or("Invalid match history")?
        .push(report.clone());
    m["result"] = judge(&m, &final_orders);
    if m["result"].is_null() {
        m["turn"] = num(n(&m["turn"]) + 1.0);
    }
    Ok(json!({"match":m,"report":report}))
}
fn outcome(a: bool, b: bool, reason: &str) -> Value {
    json!({"winner":if a&&b{Value::Null}else if a{json!(1)}else{json!(0)},"reason":reason})
}
fn judge(m: &Value, orders: &[Value]) -> Value {
    let a = &m["fighters"][0];
    let bb = &m["fighters"][1];
    let downa = n(&a["hp"]) <= 0.0;
    let downb = n(&bb["hp"]) <= 0.0;
    if downa || downb {
        return outcome(
            downa,
            downb,
            if downa && downb { "double_ko" } else { "ko" },
        );
    }
    let fleea = s(&orders[0]["action"]["type"]) == "flee";
    let fleeb = s(&orders[1]["action"]["type"]) == "flee";
    if fleea || fleeb {
        return outcome(fleea, fleeb, "fled");
    }
    let afka = n(&a["afkStreak"]) >= 3.0;
    let afkb = n(&bb["afkStreak"]) >= 3.0;
    if afka || afkb {
        return outcome(afka, afkb, "afk");
    }
    if n(&m["turn"]) >= n(&weather(m)["turnLimit"]) {
        let pa = n(&a["hp"]) / n(&a["maxHp"]);
        let pb = n(&bb["hp"]) / n(&bb["maxHp"]);
        return outcome(pa <= pb, pb <= pa, "turn_limit");
    }
    Value::Null
}
