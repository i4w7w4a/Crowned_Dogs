//! Synchronisation: the operator's level (experience from fights), four stats bought with level
//! points on a falling curve, and mastery per weapon class that grows only with the gun you fight with.
//! A profile with nothing spent and no mastery leaves every number exactly as the plain rules.
use crate::*;

pub const STATS: [&str; 4] = ["power", "cooling", "instinct", "reflexes"];
pub const CLASSES: [&str; 4] = ["pistol", "shotgun", "rifle", "sniper"];

fn cfg() -> &'static Value {
    &constants()["SYNC"]
}
fn from() -> f64 {
    n(&cfg()["mastery"]["from"])
}
fn to() -> f64 {
    n(&cfg()["mastery"]["to"])
}
fn budget(level: f64) -> f64 {
    n(&cfg()["start"]) + n(&cfg()["perLevel"]) * (level - 1.0)
}
/// Experience from `level` to the next one.
fn need(level: f64) -> f64 {
    n(&cfg()["levelXp"]["base"]) + n(&cfg()["levelXp"]["step"]) * (level - 1.0)
}
/// The first points give the most: p / (p + half) of the stat's ceiling.
fn share(points: f64) -> f64 {
    points / (points + n(&cfg()["half"]))
}
fn mastery_share(p: &Value, class: &str) -> f64 {
    if !CLASSES.contains(&class) {
        return 0.0;
    }
    (n(&p["mastery"][class]) - from()) / (to() - from())
}
fn tenth(v: f64) -> f64 {
    round(v * 10.0) / 10.0
}
fn add(v: &mut Value, key: &str, d: f64) {
    if d != 0.0 {
        v[key] = num(round((n(&v[key]) + d) * 10000.0) / 10000.0);
    }
}
pub fn fresh() -> Value {
    let mut mastery = json!({});
    for k in CLASSES {
        mastery[k] = num(from());
    }
    json!({"level":1,"xp":0,"free":cfg()["start"],
        "stats":{"power":0,"cooling":0,"instinct":0,"reflexes":0},"mastery":mastery})
}
/// A stored profile with every field present and inside the rules.
pub fn clean(p: &Value) -> Value {
    let mut out = fresh();
    if !p.is_object() {
        return out;
    }
    let level = clamp(n(&p["level"]).floor(), 1.0, n(&cfg()["maxLevel"]));
    out["level"] = num(level);
    out["xp"] = num(n(&p["xp"]).max(0.0).floor());
    let mut left = budget(level);
    for k in STATS {
        let v = n(&p["stats"][k]).max(0.0).floor().min(left);
        out["stats"][k] = num(v);
        left -= v;
    }
    out["free"] = num(left);
    for k in CLASSES {
        out["mastery"][k] = num(clamp(
            p["mastery"][k].as_f64().unwrap_or(from()),
            from(),
            to(),
        ));
    }
    out
}
/// True when the profile changes anything in a fight.
pub fn plays(p: &Value) -> bool {
    let p = clean(p);
    STATS.iter().any(|k| n(&p["stats"][k]) > 0.0)
        || CLASSES.iter().any(|k| n(&p["mastery"][k]) > from())
}
/// What the spent points add, summed over the four stats.
fn bonus(p: &Value) -> Value {
    let mut out = json!({});
    for k in STATS {
        let f = share(n(&p["stats"][k]));
        if let Some(max) = cfg()["stats"][k]["max"].as_object() {
            for (key, v) in max {
                out[key] = num(n(&out[key]) + n(v) * f);
            }
        }
    }
    out
}
/// The compiled kit with the profile's stats and mastery on top.
pub fn apply(kit: &Value, p: &Value) -> Value {
    let p = clean(p);
    let fx = bonus(&p);
    let m = &cfg()["mastery"];
    let mut kit = kit.clone();
    for slot in ["weapon", "sidearm", "knife"] {
        if kit[slot].is_null() {
            continue;
        }
        let w = &mut kit[slot];
        let t = mastery_share(&p, s(&w["id"]));
        let power = n(&w["power"]);
        // damage is a share of the whole shot: 27 + power·0.74
        add(w, "power", n(&fx["damage"]) * (27.0 / 0.74 + power));
        add(w, "flatPen", n(&fx["pen"]));
        add(w, "crit", n(&fx["crit"]) + t * n(&m["crit"]));
        add(w, "critBonus", n(&fx["critMult"]));
        add(w, "doubleShot", n(&fx["doubleShot"]));
        add(w, "guardBonus", n(&fx["guard"]));
        let acc = t * n(&m["acc"]);
        if acc != 0.0 {
            w["accuracy"] = json!(a(&w["accuracy"])
                .iter()
                .map(|v| if n(v) > 0.0 {
                    round(clamp(n(v) + acc, 0.02, 0.98) * 10000.0) / 10000.0
                } else {
                    0.0
                })
                .collect::<Vec<_>>());
        }
    }
    add(&mut kit, "dodge", n(&fx["dodge"]));
    add(&mut kit, "dashEvade", n(&fx["dashEvade"]));
    add(&mut kit, "antiPen", n(&fx["antiPen"]));
    add(&mut kit, "heatResist", n(&fx["heat"]));
    let regen = round(n(&fx["regen"]));
    if regen > 0.0 {
        kit["regen"] = num(regen);
    }
    kit
}
/// One point into a stat, if there is a free one.
pub fn spend(p: &Value, stat: &str) -> Value {
    let mut p = clean(p);
    if STATS.contains(&stat) && n(&p["free"]) > 0.0 {
        p["stats"][stat] = num(n(&p["stats"][stat]) + 1.0);
        p["free"] = num(n(&p["free"]) - 1.0);
    }
    p
}
/// The bot mirrors the operator: as many points as the operator has spent, laid out for its gun,
/// and the operator's best mastery on its own class. Null when that changes nothing.
pub fn bot(p: &Value, weapon: &str) -> Value {
    let p = clean(p);
    let spent: f64 = STATS.iter().map(|k| n(&p["stats"][k])).sum();
    let best = CLASSES
        .iter()
        .map(|k| n(&p["mastery"][k]))
        .fold(from(), f64::max);
    if spent == 0.0 && best <= from() {
        return Value::Null;
    }
    let weights = &cfg()["botStats"][weapon];
    let mut out = fresh();
    let level = (1..=n(&cfg()["maxLevel"]) as usize)
        .map(|l| l as f64)
        .find(|l| budget(*l) >= spent)
        .unwrap_or(n(&cfg()["maxLevel"]));
    out["level"] = num(level);
    let mut left = spent;
    for k in STATS {
        let v = (spent * n(&weights[k])).floor().min(left);
        out["stats"][k] = num(v);
        left -= v;
    }
    // the leftovers go to the gun's favourite stat
    let first = STATS.iter().fold("power", |best, k| {
        if n(&weights[*k]) > n(&weights[best]) {
            *k
        } else {
            best
        }
    });
    out["stats"][first] = num(n(&out["stats"][first]) + left);
    out["free"] = num(budget(level) - spent);
    if CLASSES.contains(&weapon) {
        out["mastery"][weapon] = num(best);
    }
    out
}
fn comma(v: f64) -> String {
    format!("{v}").replace('.', ",")
}
fn pct(v: f64) -> String {
    format!("+{}%", comma(tenth(v * 100.0)))
}
/// Experience and mastery after a finished fight, for `side`.
pub fn gain(p: &Value, m: &Value, side: usize) -> Value {
    let mut p = clean(p);
    let c = cfg();
    let winner = &m["result"]["winner"];
    let won = winner.as_f64() == Some(side as f64);
    let outcome = if won {
        "win"
    } else if winner.is_null() && !m["result"].is_null() {
        "draw"
    } else {
        "loss"
    };
    let mut dealt = 0.0;
    let mut hits = [0.0f64; 4];
    for turn in a(&m["history"]) {
        let at = &turn["attacks"][side];
        if b(&at["hit"]) {
            dealt += n(&at["damage"]);
            if let Some(i) = CLASSES.iter().position(|k| *k == s(&at["weapon"])) {
                hits[i] += 1.0;
            }
        }
    }
    let xp = round(
        n(&c["xp"][outcome]) + (dealt * n(&c["xp"]["perDamage"])).min(n(&c["xp"]["damageCap"])),
    );
    let mut lines = vec![format!("+{xp} опыта")];
    let max = n(&c["maxLevel"]);
    let mut level = n(&p["level"]);
    let mut have = n(&p["xp"]) + xp;
    let before = level;
    while level < max && have >= need(level) {
        have -= need(level);
        level += 1.0;
    }
    if level >= max {
        have = 0.0;
    }
    p["level"] = num(level);
    p["xp"] = num(have);
    p["free"] = num(n(&p["free"]) + budget(level) - budget(before));
    if level > before {
        lines.push(format!(
            "Уровень {level}: +{} очка в капсуле",
            budget(level) - budget(before)
        ));
    }
    let main = s(&m["config"]["fighters"][side]["weapon"]);
    let mc = &c["mastery"];
    for (i, k) in CLASSES.iter().enumerate() {
        let mut g = hits[i].min(n(&mc["hitCap"])) * n(&mc["perHit"]);
        if won && *k == main {
            g += n(&mc["win"]);
        }
        let old = n(&p["mastery"][*k]);
        g *= 1.0 - n(&mc["slow"]) * mastery_share(&p, k);
        let new = tenth((old + g).min(to()));
        if new > old {
            p["mastery"][*k] = num(new);
            lines.push(format!(
                "{}: {}% → {}%",
                s(&constants()["WEAPONS"][*k]["name"]),
                comma(tenth(old)),
                comma(new)
            ));
        }
    }
    json!({"profile":p,"xp":xp,"levels":level-before,"lines":lines})
}
fn stat_lines(key: &str, f: f64) -> Vec<String> {
    let max = &cfg()["stats"][key]["max"];
    let mut out = vec![];
    let v = |k: &str| n(&max[k]) * f;
    match key {
        "power" => {
            out.push(format!("урон {}", pct(v("damage"))));
            out.push(format!(
                "сила крита +{}",
                comma(round(v("critMult") * 100.0) / 100.0)
            ));
            out.push(format!("пробитие брони +{}", comma(tenth(v("pen")))));
        }
        "cooling" => {
            out.push(format!("пробитие по тебе −{}", comma(tenth(v("antiPen")))));
            out.push(format!("+{} здоровья за ход", round(v("regen"))));
            out.push(format!(
                "жара гасит щит на {}% меньше",
                round(v("heat") * 100.0)
            ));
        }
        "instinct" => {
            out.push(format!("шанс крита {}", pct(v("crit"))));
            out.push(format!("двойной выстрел {}", pct(v("doubleShot"))));
            out.push(format!("сквозь щит {}", pct(v("guard"))));
        }
        _ => {
            out.push(format!("уклонение {}", pct(v("dodge"))));
            out.push(format!("уклонение в рывке {}", pct(v("dashEvade"))));
        }
    }
    out
}
/// Everything the capsule screen shows.
pub fn view(p: &Value) -> Value {
    let p = clean(p);
    let level = n(&p["level"]);
    let top = level >= n(&cfg()["maxLevel"]);
    let stats: Vec<Value> = STATS
        .iter()
        .map(|k| {
            let points = n(&p["stats"][*k]);
            let now = share(points);
            let next = share(points + 1.0);
            json!({"id":k,"name":cfg()["stats"][*k]["name"],"about":cfg()["stats"][*k]["about"],"points":points,
                "share":now,"lines":stat_lines(k,now),"next":stat_lines(k,next-now)})
        })
        .collect();
    let m = &cfg()["mastery"];
    let mastery: Vec<Value> = CLASSES
        .iter()
        .map(|k| {
            let v = n(&p["mastery"][*k]);
            let t = mastery_share(&p, k);
            json!({"id":k,"name":constants()["WEAPONS"][*k]["name"],"value":v,"text":format!("{}%",comma(tenth(v))),"share":t,
                "lines":[format!("точность {}",pct(t*n(&m["acc"]))),format!("шанс крита {}",pct(t*n(&m["crit"])))]})
        })
        .collect();
    let sync: f64 = (CLASSES.iter().map(|k| n(&p["mastery"][*k])).sum::<f64>()) / 4.0;
    json!({"level":level,"xp":p["xp"],"need":if top{Value::Null}else{num(need(level))},
        "progress":if top{1.0}else{n(&p["xp"])/need(level)},"free":p["free"],"stats":stats,"mastery":mastery,
        "sync":format!("{}%",comma(tenth(sync)))})
}
