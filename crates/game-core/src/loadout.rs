use crate::*;

fn check_weapon(slot: &str, w: &Value) -> Value {
    let Ok(base) = item(s(&w["item"])) else {
        return json!({"slot":slot,"error":"unknown_item"});
    };
    if s(&base["kind"]) != "weapon" {
        return json!({"slot":slot,"error":"wrong_slot"});
    }
    if slot == "weapon2" && n(&constants()["WEAPONS"][s(&base["weapon"])]["hands"]) != 1.0 {
        return json!({"slot":slot,"error":"two_handed"});
    }
    if let Some(mods) = w["mods"].as_object() {
        for (key, id) in mods {
            let Ok(m) = item(s(id)) else {
                return json!({"slot":slot,"mod":key,"error":"unknown_item"});
            };
            if s(&m["kind"]) != "mod" || s(&m["slot"]) != key {
                return json!({"slot":slot,"mod":key,"error":"wrong_slot"});
            }
            if !contains(&m["fits"], s(&base["weapon"])) {
                return json!({"slot":slot,"mod":key,"error":"does_not_fit"});
            }
        }
    }
    if let Some(skin) = w.get("skin") {
        if item(s(skin))
            .map(|i| s(&i["kind"]) != "skin")
            .unwrap_or(true)
        {
            return json!({"slot":slot,"error":"bad_skin"});
        }
    }
    Value::Null
}
pub fn validate(l: &Value) -> Value {
    let err = check_weapon("weapon1", &l["weapon1"]);
    if !err.is_null() {
        return err;
    }
    if !l["weapon2"].is_null() {
        let err = check_weapon("weapon2", &l["weapon2"]);
        if !err.is_null() {
            return err;
        }
    }
    for slot in &GEAR[2..] {
        let Ok(i) = item(s(&l[*slot])) else {
            return json!({"slot":slot,"error":"unknown_item"});
        };
        if s(&i["kind"]) != *slot {
            return json!({"slot":slot,"error":"wrong_slot"});
        }
    }
    Value::Null
}
pub fn parts(w: &Value) -> Result<Value, String> {
    let mut out = item(s(&w["item"]))?["stock"].clone();
    for slot in MODS {
        if let Some(id) = w["mods"][slot].as_str() {
            if !id.is_empty() {
                out[slot] = json!(id);
            }
        }
    }
    Ok(out)
}
fn sum(list: &[Value], key: &str) -> f64 {
    list.iter().map(|e| n(&e[key])).sum()
}
fn sub_sum(list: &[Value], key: &str, sub: &str) -> f64 {
    list.iter().map(|e| n(&e[key][sub])).sum()
}
fn rounded(v: f64) -> f64 {
    round(v * 1000.0) / 1000.0
}
pub fn apply(spec: &Value, list: &[Value]) -> Value {
    let mut out = spec.clone();
    out["power"] = num(n(&spec["power"]) + sum(list, "power"));
    out["flatPen"] = num(n(&spec["flatPen"]) + sum(list, "pen"));
    out["crit"] = num(rounded(n(&spec["crit"]) + sum(list, "crit")));
    out["critBonus"] = num(rounded(n(&spec["critBonus"]) + sum(list, "critMult")));
    out["accuracy"] = json!(a(&spec["accuracy"])
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if n(v) > 0.0 {
                rounded(clamp(
                    n(v) + sub_sum(list, "acc", band((i + 1) as f64)),
                    0.02,
                    0.98,
                ))
            } else {
                0.0
            }
        })
        .collect::<Vec<_>>());
    out["falloff"] = json!(a(&spec["falloff"])
        .iter()
        .enumerate()
        .map(|(i, v)| rounded((n(v) + sub_sum(list, "falloff", band((i + 1) as f64))).max(0.1)))
        .collect::<Vec<_>>());
    for key in ["doubleShot", "nightAcc", "tracking"] {
        out[key] = num(rounded(sum(list, key)));
    }
    out["stanceTurns"] = json!(if list.iter().any(|e| n(&e["stanceTurns"]) == 1.0) {
        1
    } else {
        2
    });
    out["noStancePenalty"] = num(rounded((0.5 + sum(list, "noStance")).max(0.0)));
    out["guardBonus"] = num(rounded(sum(list, "guard")));
    out["suppressor"] = json!(list.iter().any(|e| b(&e["suppressor"])));
    out
}
fn weapon_effects(w: &Value) -> Result<Vec<Value>, String> {
    let mut effects = vec![item(s(&w["item"]))?["effects"].clone()];
    if let Some(parts) = parts(w)?.as_object() {
        for id in parts.values() {
            effects.push(item(s(id))?["effects"].clone());
        }
    }
    Ok(effects)
}
pub fn compile(l: &Value) -> Result<Value, String> {
    let err = validate(l);
    if !err.is_null() {
        return Err(format!("Invalid loadout: {err}"));
    }
    let main = item(s(&l["weapon1"]["item"]))?;
    let main_fx = weapon_effects(&l["weapon1"])?;
    let side = if l["weapon2"].is_null() {
        Value::Null
    } else {
        item(s(&l["weapon2"]["item"]))?
    };
    let side_fx = if side.is_null() {
        vec![]
    } else {
        weapon_effects(&l["weapon2"])?
    };
    let knife = item(s(&l["knife"]))?;
    let grenade = item(s(&l["grenade"]))?;
    let mut persona = Vec::new();
    for key in ["head", "body", "suit", "boots"] {
        persona.push(item(s(&l[key]))?["effects"].clone());
    }
    persona.extend(main_fx.clone());
    persona.extend(side_fx.clone());
    let mut armor = json!({});
    for zone in ZONES {
        armor[zone] = num(sub_sum(&persona, "armor", zone));
    }
    let mut camo = json!({});
    for band in BANDS {
        camo[band] = num(sub_sum(&persona, "camo", band));
    }
    Ok(json!({
        "weapon":apply(&constants()["WEAPONS"][s(&main["weapon"])],&main_fx),
        "sidearm":if side.is_null(){Value::Null}else{apply(&constants()["WEAPONS"][s(&side["weapon"])],&side_fx)},
        "knife":apply(&constants()["WEAPONS"]["knife"],&[knife["effects"].clone()]),
        "armor":armor,"nvg":persona.iter().any(|e|b(&e["nvg"])),"thermal":persona.iter().any(|e|b(&e["thermal"])),
        "heatProof":persona.iter().any(|e|b(&e["heatProof"])),"grenade":grenade["grenade"],
        "grenades":1.0+sum(&persona,"grenades"),"dodge":rounded(sum(&persona,"dodge")),
        "dashEvade":rounded(sum(&persona,"dashEvade")),"camo":camo
    }))
}
fn word_num(v: f64) -> String {
    format!("{}", round(v * 100.0) / 100.0).replace('.', ",")
}
fn pct(v: f64) -> String {
    format!(
        "{}{}%",
        if v > 0.0 { "+" } else { "−" },
        round(v.abs() * 100.0)
    )
}
fn band_word(band: &str) -> &'static str {
    match band {
        "near" => "вблизи (1–4 шага)",
        "mid" => "на средней (5–8)",
        _ => "вдали (9+)",
    }
}
pub fn effect_lines(e: &Value) -> Vec<Value> {
    let mut out = vec![];
    let mut add = |text: String, good: bool| out.push(json!({"text":text,"good":good}));
    for band in BANDS {
        let v = n(&e["acc"][band]);
        if v != 0.0 {
            add(
                format!("{} к точности {}", pct(v), band_word(band)),
                v > 0.0,
            );
        }
    }
    let v = n(&e["nightAcc"]);
    if v != 0.0 {
        add(format!("{} к точности ночью", pct(v)), v > 0.0);
    }
    for band in BANDS {
        let v = n(&e["falloff"][band]);
        if v != 0.0 {
            add(format!("урон {} {}", band_word(band), pct(v)), v > 0.0);
        }
    }
    let v = n(&e["power"]) * 0.74;
    if v != 0.0 {
        add(
            format!(
                "урон {}{}",
                if v > 0.0 { "+" } else { "−" },
                word_num(round(v.abs() * 10.0) / 10.0)
            ),
            v > 0.0,
        );
    }
    let v = n(&e["pen"]);
    if v != 0.0 {
        add(
            format!(
                "{}{} к пробитию брони",
                if v > 0.0 { "+" } else { "−" },
                v.abs()
            ),
            v > 0.0,
        );
    }
    let v = n(&e["crit"]);
    if v != 0.0 {
        add(format!("{} к шансу крита", pct(v)), v > 0.0);
    }
    let v = n(&e["critMult"]);
    if v != 0.0 {
        add(
            format!(
                "{}{} к силе крита",
                if v > 0.0 { "+" } else { "−" },
                word_num(v.abs())
            ),
            v > 0.0,
        );
    }
    let v = n(&e["doubleShot"]);
    if v != 0.0 {
        add(format!("{}% шанс попасть дважды", round(v * 100.0)), true);
    }
    let v = n(&e["tracking"]);
    if v != 0.0 {
        add(format!("{} к точности по цели в рывке", pct(v)), v > 0.0);
    }
    if n(&e["stanceTurns"]) == 1.0 {
        add("стойка готова за 1 ход вместо 2".into(), true);
    }
    let v = n(&e["noStance"]);
    if v != 0.0 {
        add(
            format!(
                "без стойки точность ×{} вместо ×{}",
                word_num(0.5 + v),
                word_num(0.5)
            ),
            v > 0.0,
        );
    }
    let v = n(&e["guard"]);
    if v != 0.0 {
        add(
            format!(
                "сквозь щит проходит {}% вместо 50%",
                round((0.5 + v) * 100.0)
            ),
            v > 0.0,
        );
    }
    if b(&e["suppressor"]) {
        add("глушитель: ночью нет вспышки".into(), true);
    }
    for zone in ZONES {
        let v = n(&e["armor"][zone]);
        if v != 0.0 {
            add(
                format!("броня: {} {v}", s(&constants()["ZONE_NAMES"][zone])),
                true,
            );
        }
    }
    let v = n(&e["dodge"]);
    if v != 0.0 {
        add(format!("{} к уклонению", pct(v)), v > 0.0);
    }
    let v = n(&e["dashEvade"]);
    if v != 0.0 {
        add(format!("{} к уклонению в рывке", pct(v)), v > 0.0);
    }
    for band in BANDS {
        let v = n(&e["camo"][band]);
        if v != 0.0 {
            add(
                format!("по тебе {} {} к точности", band_word(band), pct(-v)),
                v > 0.0,
            );
        }
    }
    let v = n(&e["grenades"]);
    if v != 0.0 {
        add(format!("+{v} граната за бой"), true);
    }
    if b(&e["nvg"]) {
        add("ПНВ: нет ночного штрафа".into(), true);
    }
    if b(&e["thermal"]) {
        add("тепловизор: пыль и дым мешают вдвое меньше".into(), true);
    }
    if b(&e["heatProof"]) {
        add("щит не перегревается днём".into(), true);
    }
    out
}
pub fn describe_grenade(g: &Value) -> Value {
    let range = format!("{}–{} шагов", n(&g["minDistance"]), n(&g["maxDistance"]));
    match s(&g["type"]) {
        "frag" => json!([format!("осколки: {} до брони", n(&g["base"])), range]),
        "emp" => json!([
            "щит врага выключен на следующий ход",
            format!("урон {} до брони", n(&g["base"])),
            range
        ]),
        _ => json!([
            "дым под ноги: в этот и следующий ход по тебе −35% к точности",
            "с любой дистанции"
        ]),
    }
}
