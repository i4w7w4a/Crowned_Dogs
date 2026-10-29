//! Platform-independent view models, text/HTML composition and interaction geometry.
//! DOM events and painting are deliberately kept at the browser boundary.
use serde_json::{Value, json};

fn n(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}
fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn call(method: &str, args: Value) -> Result<Value, String> {
    game_core::dispatch(method, args)
}
fn client(method: &str, args: Value) -> Result<Value, String> {
    game_client::dispatch(method, args)
}
fn icon(id: &str, class: &str) -> String {
    let class = if class.is_empty() {
        String::new()
    } else {
        format!(" {}", escape(class))
    };
    format!(
        "<svg class=\"ico{class}\" aria-hidden=\"true\"><use href=\"#{}\"/></svg>",
        escape(id)
    )
}
const CHECK: &str = "<span class=\"cd-equipped-check\" aria-hidden=\"true\"><svg viewBox=\"0 0 24 24\"><path d=\"m5 12 4 4L19 6\"/></svg></span>";

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    let x = |i: usize| args.get(i).unwrap_or(&Value::Null);
    Ok(match method {
        "ui.icon" => json!(icon(s(x(0)), s(x(1)))),
        "ui.escape" => json!(escape(s(x(0)))),
        "ui.backdrop" => {
            let w = n(x(0));
            let h = n(x(1));
            let ratio = n(x(2));
            let still = b(x(3));
            let t = if still { 0.0 } else { n(x(4)) / 1000.0 };
            let lean = [
                n(&x(5)[0]) + (n(&x(6)[0]) - n(&x(5)[0])) * 0.04,
                n(&x(5)[1]) + (n(&x(6)[1]) - n(&x(5)[1])) * 0.04,
            ];
            let scale = if still { 1.0 } else { 1.07 };
            let sway = if still { 0.0 } else { 1.0 };
            let tx = w * (0.022 * (t / 9.0).sin() - 0.018 * lean[0] * sway);
            let ty = h * (0.022 * 0.6 * (t / 13.0 + 1.0).sin() - 0.018 * lean[1] * sway);
            let fit = (w / ratio).max(h);
            let iw = fit * ratio;
            let ih = fit;
            let points: Vec<_> = x(7)
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|uv| {
                    let px = (w - iw) / 2.0 + n(&uv[0]) * iw;
                    let py = (h - ih) / 2.0 + n(&uv[1]) * ih;
                    [
                        w / 2.0 + scale * (px - w / 2.0) + tx,
                        h / 2.0 + scale * (py - h / 2.0) + ty,
                    ]
                })
                .collect();
            json!({"lean":lean,"scale":scale,"tx":tx,"ty":ty,"points":points})
        }
        "ui.loaderPose" => {
            let t = n(x(0));
            let sway = if b(x(1)) { 0.0 } else { 1.0 };
            let rad = std::f64::consts::PI / 180.0;
            let breath = if x(2).is_null() {
                0.0
            } else {
                0.5 - 0.5 * (2.0 * std::f64::consts::PI * (n(x(2)) % 4.0) / 4.0).cos()
            };
            json!({"rotation":[(16.0+sway*2.0*(t*0.7).sin())*rad,(-22.0+sway*9.0*(t*0.45).sin())*rad,(-13.0+sway*1.5*(t*0.33).cos())*rad],"scale":1.0+0.012*breath})
        }
        "ui.loaderDistance" => {
            let half = n(x(0)).to_radians() / 2.0;
            let horizontal = (half.tan() * n(x(1))).atan();
            json!(n(x(2)) / half.min(horizontal).sin() * 1.02)
        }
        "ui.health" => {
            let hp = n(x(0));
            let max = n(x(1)).max(1.0);
            json!({"text":hp.max(0.0).round().to_string(),"width":format!("{}%",(hp/max*100.0).clamp(0.0,100.0)),"low":hp>0.0&&hp<=max*0.25})
        }
        "ui.battleControls" => {
            let m = x(0);
            let d = x(1);
            let busy = b(x(2));
            let over = !m["result"].is_null();
            let kind = s(&d["kind"]);
            let lock = busy || over;
            let aimed = matches!(kind, "shoot" | "knife");
            let step = n(&d["move"]).abs() <= 1.0;
            let knife_move = if b(&call("canUseKnife", json!([m, d["move"]]))?) {
                n(&d["move"])
            } else {
                1.0
            };
            let knife_ok = step && b(&call("canUseKnife", json!([m, knife_move]))?);
            let nade_ok = step && b(&call("canThrowGrenade", json!([m, 0]))?);
            let planned = n(&call("plannedDistance", json!([m, d["move"]]))?);
            let chance = call("draftChance", json!([m, 0, d]))?;
            let msg = if over {
                json!("Бой окончен")
            } else if busy {
                json!("Ход идёт…")
            } else {
                call("missing", json!([d]))?
            };
            let mut knife_draft = d.clone();
            knife_draft["kind"] = json!("knife");
            knife_draft["move"] = json!(knife_move);
            json!({"lock":lock,"run":matches!(kind,"run"|"dash"|"flee"),"aimed":aimed,"step":step,
                "knifeMove":knife_move,"knifeOk":knife_ok,"nadeOk":nade_ok,"planned":planned,"chance":chance,
                "knifeChance":call("draftChance",json!([m,0,knife_draft]))?,
                "need":call("guardsNeeded",json!([d]))?,"grenade":call("grenadeSpec",json!([m,0]))?,
                "spare":call("swapWeapon",json!([m,0]))?,"message":msg,
                "dist":if n(&d["move"])!=0.0&&!busy {json!(planned)}else{m["distance"].clone()},
                "noShot":(matches!(kind,"run"|"dash"|"flee"|"swap"))&&!lock})
        }
        "ui.reticles" => {
            let points = x(0).as_array().ok_or("Missing reticle projections")?;
            let radius = n(x(1));
            let mut limit = n(x(2)) - radius - 6.0;
            let mut out = vec![[0.0; 3]; points.len()];
            for i in (0..points.len()).rev() {
                let y = n(&points[i][1]).min(limit);
                out[i] = [n(&points[i][0]), y, 0.0];
                limit = y - (22.0_f64).max(radius * 2.0 + 8.0);
            }
            for i in 0..out.len() {
                let mut near = 40.0_f64;
                for j in 0..out.len() {
                    if i != j {
                        near = near.min((out[i][0] - out[j][0]).hypot(out[i][1] - out[j][1]));
                    }
                }
                out[i][2] = (radius * 2.0 + 4.0).max(near).round();
            }
            json!(out)
        }
        "ui.layout" => {
            let cw = n(x(0));
            let ch = n(x(1));
            let f = x(2);
            let top = ch * n(&f["top"]);
            let scale = (cw / n(&f["w"])).max((ch - top) / n(&f["h"]));
            let w = n(&f["w"]) * scale;
            let h = n(&f["h"]) * scale;
            let left = (cw - w) * 0.5;
            let vx = (-left).max(0.0);
            let vy = (-top).max(0.0);
            json!({"x":left,"y":top,"w":w,"h":h,"radius":(w*0.02-0.75).max(6.0),
                "viewport":{"x":vx/w,"y":vy/h,"w":(w.min(cw-left)-vx)/w,"h":(h.min(ch-top)-vy)/h}})
        }
        "ui.catalog" => {
            let slot = s(x(0));
            let selected = x(1);
            let items = x(2).as_array().ok_or("Missing catalog items")?;
            let mut html = String::new();
            if slot == "weapon2" {
                html = format!(
                    "<button class=\"glass cd-item-card\" type=\"button\" data-item=\"\" aria-pressed=\"{}\"><span class=\"cd-item-art\" aria-hidden=\"true\">—</span><span class=\"cd-item-copy\"><strong>Без второго оружия</strong></span>{}</button>",
                    selected.is_null(),
                    if selected.is_null() { CHECK } else { "" }
                );
            }
            for item in items {
                let value = &item["item"];
                let effects = call("describeItem", json!([value]))?;
                let effects: Vec<_> = effects
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| escape(s(v)))
                    .collect();
                let effects = if effects.is_empty() {
                    String::new()
                } else {
                    format!("<span class=\"cd-effects\">{}</span>", effects.join(" · "))
                };
                let on = selected == &value["id"];
                html.push_str(&format!("<button class=\"glass cd-item-card\" type=\"button\" data-item=\"{}\" aria-pressed=\"{on}\"><span class=\"cd-item-art\">{}</span><span class=\"cd-item-copy\"><strong>{}</strong><small>{}</small>{effects}</span>{}</button>",escape(s(&value["id"])),s(&item["icon"]),escape(s(&value["name"])),escape(s(&value["note"])),if on{CHECK}else{""}));
            }
            json!(html)
        }
        "ui.fightLog" => {
            let r = x(0);
            let names = x(1);
            let weapons = x(2);
            let a = client("words.turnLine", json!([r, 0, weapons[0]]))?;
            let c = client("words.turnLine", json!([r, 1, weapons[1]]))?;
            let dist = if r["distanceBefore"] == r["distanceAfter"] {
                format!("{}", r["distanceAfter"])
            } else {
                format!("{} → {}", r["distanceBefore"], r["distanceAfter"])
            };
            json!({"entry":format!("<div class=\"eh\"><span>Ход {}</span><span>дистанция {dist}</span></div>{}{}",r["turn"],line_html("me","Ты",&a),line_html("en",s(&names[1]),&c)),
                "ticker":format!("<b>Ход {}</b>Ты: {} · {}: {}",r["turn"],short_result(&a,true),escape(s(&names[1])),short_result(&c,false))})
        }
        "ui.finishStats" => {
            let history = x(0).as_array().ok_or("Missing battle history")?;
            let dealt: f64 = history.iter().map(|t| n(&t["attacks"][0]["damage"])).sum();
            let taken: f64 = history.iter().map(|t| n(&t["attacks"][1]["damage"])).sum();
            let turns = history.len();
            let word = client("words.turnsWord", json!([turns]))?;
            json!(format!(
                "<div><b>{dealt}</b><span>Нанесено</span></div><div><b>{taken}</b><span>Получено</span></div><div><b>{turns}</b><span>{}</span></div>",
                escape(s(&word))
            ))
        }
        "ui.pickerNext" => {
            let key = s(x(0));
            let index = n(x(1)) as i64;
            let len = n(x(2)) as i64;
            json!(if len <= 0 {
                0
            } else if key == "Home" {
                0
            } else if key == "End" {
                len - 1
            } else {
                (index + if key == "ArrowDown" { 1 } else { -1 } + len).rem_euclid(len)
            })
        }
        _ => return Err(format!("Unknown UI operation: {method}")),
    })
}

fn line_html(side: &str, who: &str, l: &Value) -> String {
    let ic = match s(&l["icon"]) {
        "move" => icon(
            if b(&l["dash"]) { "i-mv-3" } else { "i-mv-2" },
            if b(&l["back"]) { "flip" } else { "" },
        ),
        "shot" => icon("i-shot", ""),
        kind => icon(&format!("i-{kind}"), ""),
    };
    let o = &l["outcome"];
    let result = match s(&o["kind"]) {
        "none" => format!("<span class=\"r nil\">{}</span>", escape(s(&o["text"]))),
        "miss" => "<span class=\"r nil\">мимо</span>".into(),
        "blocked" => format!(
            "<span class=\"r blk\">{}−{}</span>",
            icon("i-zont", ""),
            o["damage"]
        ),
        _ => format!(
            "<span class=\"r dmg\">{}−{}</span>",
            if b(&o["crit"]) {
                icon("i-crit", "")
            } else {
                String::new()
            },
            o["damage"]
        ),
    };
    let act = s(&l["act"]);
    format!(
        "<div class=\"ln {side}\">{ic}<span class=\"w\">{}</span><span class=\"a\">{}</span>{result}</div>",
        escape(who),
        escape(if act.is_empty() {
            "на месте"
        } else {
            act
        })
    )
}
fn short_result(l: &Value, mine: bool) -> String {
    let o = &l["outcome"];
    match s(&o["kind"]) {
        "none" => {
            if s(&l["icon"]) == "swap" {
                "смена оружия".into()
            } else if s(&l["act"]).is_empty() {
                "на месте".into()
            } else {
                escape(s(&l["act"]))
            }
        }
        "miss" => format!(
            "<em class=\"nil\">{}</em>",
            if s(&l["icon"]) == "grenade" {
                "граната мимо"
            } else {
                "промах"
            }
        ),
        "blocked" => format!("<em class=\"blk\">Зонт −{}</em>", o["damage"]),
        _ => format!(
            "<em class=\"{}\">{} −{}{}</em>",
            if mine { "us" } else { "them" },
            escape(s(&l["target"])),
            o["damage"],
            if b(&o["crit"]) {
                icon("i-crit", "")
            } else {
                String::new()
            }
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_escapes_item_text() {
        let html=dispatch("ui.catalog",json!(["head","test",[{"item":{"id":"test","name":"<img>","note":"a & b","kind":"head","effects":{}},"icon":"<svg/>"}]])).unwrap();
        assert!(s(&html).contains("&lt;img&gt;"));
        assert!(s(&html).contains("a &amp; b"));
        assert!(s(&html).contains("cd-equipped-check"));
    }
    #[test]
    fn constrained_reticles_stay_separate() {
        let result = dispatch(
            "ui.reticles",
            json!([[[100, 400], [100, 420], [100, 440], [100, 460]], 8, 350]),
        )
        .unwrap();
        let a = result.as_array().unwrap();
        for pair in a.windows(2) {
            assert!(n(&pair[1][1]) - n(&pair[0][1]) >= 24.0);
        }
        assert!(n(&a[3][1]) < 350.0);
    }
}
