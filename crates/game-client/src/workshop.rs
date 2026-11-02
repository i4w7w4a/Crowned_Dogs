//! Fitting catalogue and weapon workshop view-models, layout, and safe markup.
use crate::{core, loadout, num, text, truth};
use serde_json::{json, Value};
const SLOTS: [&str; 5] = ["optic", "muzzle", "rail", "mag", "stock"];
/// Item id to preview model. `file#Prefix` shows only that file's meshes named with the prefix.
/// Only items with a model of their own are offered (Art, 2026-11-01 and 2026-11-02); the night-vision
/// helmet «Сыч» stays on the K-6 shell for now.
fn models() -> Value {
    // Temporary, explicitly approved placeholder: Sych keeps its own rules and
    // catalogue icon, but uses the ordinary helmet until its model is ready.
    json!({"head-k6":"tripo-helmet","head-sych":"tripo-helmet",
        "head-cap":"gear#Cap","head-sfera":"gear#Sfera",
        "body-plita2":"tripo-vest","body-plita4":"gear#Plita4",
        "suit-field":"gear#Suit","suit-ryvok":"gear#Ryvok","boots-bertsy":"gear#Boots",
        "knife":"knife","knife-std":"knife","grenade-rg1":"grenade",
        "pistol-pustelga":"pustelga","shotgun-taran":"taran","rifle-varyag":"varyag","sniper-berkut":"berkut"})
}
/// Mods with a mesh inside each gun class's model. The mesh node is the id with `_` for `-`.
const MOD_MESHES: [(&str, &[&str]); 4] = [
    (
        "pistol",
        &[
            "optic-tritium",
            "optic-kp1",
            "muzzle-klyk",
            "muzzle-pb9",
            "rail-luch",
            "rail-lcu",
            "rail-kulak",
            "mag-pistol-long",
            "mag-pistol-hp",
            "mag-pistol-ap",
            "stock-pistol-hvat",
            "stock-pistol-upor",
        ],
    ),
    (
        "shotgun",
        &[
            "optic-blik",
            "optic-gran",
            "muzzle-choke",
            "muzzle-vata",
            "rail-luch",
            "rail-lcu",
            "mag-shotgun-box",
            "mag-shotgun-slug",
            "mag-shotgun-buck",
            "stock-shotgun-fold",
            "stock-shotgun-otdacha",
        ],
    ),
    (
        "rifle",
        &[
            "optic-sokol",
            "optic-znoy",
            "optic-gran",
            "optic-blik",
            "muzzle-tishina",
            "muzzle-klyk",
            "rail-koster",
            "rail-kulak",
            "rail-luch",
            "mag-rifle-twin",
            "mag-rifle-ap",
            "mag-rifle-drum",
            "stock-rifle-opora",
            "stock-rifle-tesno",
        ],
    ),
    (
        "sniper",
        &[
            "optic-znoy",
            "optic-oryol",
            "optic-blik-b",
            "muzzle-tishina-s",
            "muzzle-molot",
            "rail-lcu",
            "rail-upor",
            "mag-sniper-10",
            "mag-sniper-ap",
            "stock-sniper-izgotovka",
            "stock-sniper-opora",
        ],
    ),
];
fn meshed(id: &str, class: &str) -> bool {
    MOD_MESHES
        .iter()
        .any(|(c, ids)| (class.is_empty() || *c == class) && ids.contains(&id))
}
fn modelled_class(class: &Value) -> bool {
    game_core::constants()["ITEMS"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| {
            i["kind"] == "weapon" && i["weapon"] == *class && models().get(text(&i["id"])).is_some()
        })
}
/// Whether the item can be picked; for a mod, on a gun of `class` (empty: on any gun).
fn available(item: &Value, class: &str) -> bool {
    match text(&item["kind"]) {
        "mod" => {
            (truth(&item["isStock"])
                && item["fits"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(modelled_class)))
                || meshed(text(&item["id"]), class)
        }
        _ => models().get(text(&item["id"])).is_some(),
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn point(slot: &str) -> (f64, f64) {
    match slot {
        "optic" => (76.0, 53.0),
        "muzzle" => (286.0, 53.0),
        "rail" => (292.0, 256.0),
        "mag" => (184.0, 310.0),
        _ => (56.0, 264.0),
    }
}
fn slot_name(slot: &str, pistol: bool) -> &str {
    match slot {
        "optic" => "Прицел",
        "muzzle" => "Дуло",
        "rail" => "Планка",
        "mag" => "Магазин",
        "stock" if pistol => "Рукоять",
        "stock" => "Приклад",
        _ => "",
    }
}
fn weapon(kit: &Value) -> Result<Value, String> {
    let item = core("getItem", json!([kit["weapon1"]["item"]]))?;
    if item["kind"] != "weapon" {
        Err("Неизвестное оружие".into())
    } else {
        Ok(item)
    }
}
fn parts(kit: &Value) -> Result<Value, String> {
    let mut result = json!({});
    for slot in SLOTS {
        result[slot] = loadout::dispatch("client.effectiveWeaponModId", json!([kit, slot]))?;
    }
    Ok(result)
}
fn model(kit: &Value) -> Result<Value, String> {
    let weapon = weapon(kit)?;
    let parts = parts(kit)?;
    let name = text(&weapon["name"]);
    let display = name
        .split_once('«')
        .and_then(|(_, tail)| tail.split_once('»').map(|(name, _)| name))
        .unwrap_or(name);
    let mut sockets = vec![];
    for slot in SLOTS {
        let part = core("getItem", json!([parts[slot]]))?;
        let title = text(&part["name"]);
        sockets.push(json!({"slot":slot,"name":slot_name(slot,weapon["weapon"]=="pistol"),"title":title,"modified":parts[slot]!=weapon["stock"][slot]}));
    }
    Ok(
        json!({"weapon":weapon,"parts":parts,"className":game_core::constants()["WEAPONS"][text(&weapon["weapon"])]["name"],"displayName":display,"sockets":sockets,"model":models().get(text(&weapon["id"])).cloned().unwrap_or(Value::Null)}),
    )
}
fn anchor(slot: &str, lx: f64, ly: f64, ax: f64, ay: f64, r: u8) -> String {
    format!(
        r#"<g class="cw-anchor" data-anchor="{slot}"><path d="M{lx} {ly} L{ax} {ay}"/><circle class="cw-anchor-hit" cx="{ax}" cy="{ay}" r="{r}"/><circle class="cw-anchor-pin" cx="{ax}" cy="{ay}" r="4"/></g>"#
    )
}
fn links(points: &Value, w: f64, h: f64) -> String {
    SLOTS
        .iter()
        .filter_map(|slot| {
            let p = points.get(*slot)?;
            let (lx, ly) = point(slot);
            Some(anchor(
                slot,
                lx / 360.0 * w,
                ly / 360.0 * h,
                num(&p[0]),
                num(&p[1]),
                18,
            ))
        })
        .collect::<Vec<_>>()
        .join("")
}
fn initial(base: &str, back: &str) -> String {
    let buttons=SLOTS.iter().map(|slot|{let(x,y)=point(slot);format!(r#"<button class="cw-socket glass" type="button" data-socket="{slot}" style="left:{}%;top:{}%"><span class="cw-socket-dot" aria-hidden="true"></span><span class="cw-socket-name">{}</span></button>"#,x/3.6,y/3.6,slot_name(slot,false))}).collect::<Vec<_>>().join("");
    format!(
        r#"<img class="cw-room" src="{}img/base/fitting-room.webp" alt="" draggable="false">
<header class="cw-header"><button class="cw-back glass" type="button" aria-label="Назад к снаряжению">{back}<span>Назад</span></button><span>Оружие</span><button class="cw-info cw-icon glass" type="button" aria-label="Характеристики оружия"><span aria-hidden="true">i</span></button></header>
<div class="cw-title"><p class="cw-class"></p><h1 class="cw-name"></h1></div>
<div class="cw-stage"><div class="cw-model" hidden></div><svg class="cw-gun" viewBox="0 0 360 360" role="img" aria-label="Оружие с установленными модификациями"></svg><svg class="cw-links" hidden aria-hidden="true"></svg><div class="cw-sockets">{buttons}</div><button class="cw-reset cw-icon glass" type="button" aria-label="Вернуть вид оружия" title="Вернуть вид"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5.5 14.5A6.8 6.8 0 1 0 7.4 7.6L5.3 9.7"/><path d="M5.1 4.6v5.2h5.2"/></svg></button></div>
<footer class="cw-footer"><button class="cw-change glass" type="button">Сменить оружие</button><p class="cw-error" role="status" hidden></p></footer>
<dialog class="cw-sheet sheet glass" aria-labelledby="cw-sheet-title"><header class="cw-sheet-header"><h2 id="cw-sheet-title"></h2><button class="cw-close cw-icon glass" type="button" aria-label="Закрыть панель">×</button></header><div class="cw-sheet-body"></div></dialog>"#,
        escape(base)
    )
}
fn choices(kit: &Value, slot: &str) -> Result<Value, String> {
    let class = if slot == "weapon1" {
        Value::Null
    } else {
        weapon(kit)?["weapon"].clone()
    };
    let (items, selected) = if slot == "weapon1" {
        (
            loadout::dispatch("client.itemsForSlot", json!([slot]))?,
            kit["weapon1"]["item"].clone(),
        )
    } else {
        (
            loadout::dispatch("client.weaponModsForSlot", json!([kit, slot]))?,
            loadout::dispatch("client.effectiveWeaponModId", json!([kit, slot]))?,
        )
    };
    Ok(json!(items
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| available(i, text(&class)))
        .map(|item| {
            let lines = core("describeItem", json!([item])).unwrap_or(json!([]));
            json!({"item":item,"selected":item["id"]==selected,"lines":lines})
        })
        .collect::<Vec<_>>()))
}
fn options(choices: &Value, kind: &str) -> String {
    let check = r#"<span class="cd-equipped-check" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="M4.5 12.5l5 5 10-11"/></svg></span>"#;
    let kind = if kind == "weapon" { "weapon" } else { "mod" };
    let body=choices.as_array().into_iter().flatten().map(|c|format!(r#"<button class="cw-option glass" type="button" data-{kind}="{}" aria-pressed="{}"><span class="cw-part-icon">{}</span><strong>{}</strong>{}{}</button>"#,escape(text(&c["item"]["id"])),truth(&c["selected"]),text(&c["icon"]),escape(text(&c["item"]["name"])),if truth(&c["selected"]){check}else{""},tip(c))).collect::<Vec<_>>().join("");
    format!(r#"<div class="cw-options">{body}</div>"#)
}
/// Hidden card content the workshop shows beside a hovered or focused option.
fn tip(choice: &Value) -> String {
    format!(
        r#"<span class="cw-option-tip" hidden>{}</span>"#,
        card(&choice["item"], &choice["lines"])
    )
}
/// Card for the part that sits in a slot now, shown when the cursor rests on it on the 3D gun.
fn part(kit: &Value, slot: &str) -> Result<String, String> {
    let id = loadout::dispatch("client.effectiveWeaponModId", json!([kit, slot]))?;
    let item = core("getItem", json!([id]))?;
    let lines = core("describeItem", json!([item])).unwrap_or(json!([]));
    Ok(card(&item, &lines))
}
fn card(item: &Value, lines: &Value) -> String {
    let lines = lines
        .as_array()
        .into_iter()
        .flatten()
        .map(|l| format!("<em>{}</em>", escape(text(l))))
        .collect::<String>();
    let note = text(&item["note"]);
    format!(
        r#"<b>{}</b>{}{lines}"#,
        escape(text(&item["name"])),
        if note.is_empty() {
            String::new()
        } else {
            format!("<span>{}</span>", escape(note))
        }
    )
}
fn stats(kit: &Value) -> Result<String, String> {
    let compiled = core("compileKit", json!([kit]))?;
    let w = &compiled["weapon"];
    let damage = (27.0 + num(&w["power"]) * 0.74) * num(&w["falloff"][6]);
    let damage = format!("{:.1}", (damage * 10.0 + 0.5).floor() / 10.0)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",");
    let rows = [
        ("Базовый урон в корпус", damage),
        (
            if truth(&w["needsStance"]) {
                "Точность из стойки"
            } else {
                "Точность оружия"
            },
            format!("{}%", (num(&w["accuracy"][6]) * 100.0 + 0.5).floor()),
        ),
        ("Пробитие брони", num(&w["flatPen"]).to_string()),
        (
            "Шанс критического попадания",
            format!("{}%", ((0.07 + num(&w["crit"])) * 100.0 + 0.5).floor()),
        ),
    ];
    let mut mods = vec![];
    for slot in SLOTS {
        let id = loadout::dispatch("client.effectiveWeaponModId", json!([kit, slot]))?;
        let item = core("getItem", json!([id]))?;
        let lines = core("describeItem", json!([item]))?;
        if let Some(lines) = lines.as_array().filter(|a| !a.is_empty()) {
            mods.push(format!(
                "<p><strong>{}</strong><span>{}</span></p>",
                escape(text(&item["name"])),
                lines
                    .iter()
                    .map(|l| escape(text(l)))
                    .collect::<Vec<_>>()
                    .join(" · ")
            ));
        }
    }
    Ok(format!(
        r#"<p class="cw-stat-context">7 шагов. Обычное попадание без брони, щита и влияния погоды.</p><dl class="cw-stat-list">{}</dl>{}"#,
        rows.iter()
            .map(|(name, value)| format!("<div><dt>{name}</dt><dd>{value}</dd></div>"))
            .collect::<Vec<_>>()
            .join(""),
        if mods.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div class="cw-effects"><h3>Установленные модификации</h3>{}</div>"#,
                mods.join("")
            )
        }
    ))
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "client.models.catalog" => models(),
        "client.models.available" => json!(available(&args[0], "")),
        "client.workshop.initial" => json!(initial(text(&args[0]), text(&args[1]))),
        "client.workshop.model" => return model(&args[0]),
        "client.workshop.slotName" => json!(slot_name(
            text(&args[1]),
            weapon(&args[0])?["weapon"] == "pistol"
        )),
        "client.workshop.choices" => return choices(&args[0], text(&args[1])),
        "client.workshop.options" => json!(options(&args[0], text(&args[1]))),
        "client.workshop.links" => json!(links(&args[0], num(&args[1]), num(&args[2]))),
        "client.workshop.stats" => json!(stats(&args[0])?),
        "client.workshop.part" => json!(part(&args[0], text(&args[1]))?),
        "client.workshop.fallback" => {
            let box_ = &args[0];
            let (x, y, w, h) = (num(&box_[0]), num(&box_[1]), num(&box_[2]), num(&box_[3]));
            let angle = -std::f64::consts::PI / 6.0;
            let (cos, sin) = (angle.cos(), angle.sin());
            let scale = (300.0 / (w * cos - h * sin)).min(260.0 / (-w * sin + h * cos));
            let (cx, cy) = (x + w / 2.0, y + h / 2.0);
            let lines = SLOTS
                .iter()
                .map(|slot| {
                    let (ax, ay) = (num(&args[1][*slot][0]) - cx, num(&args[1][*slot][1]) - cy);
                    let (lx, ly) = point(slot);
                    anchor(
                        slot,
                        lx,
                        ly,
                        180.0 + (ax * cos - ay * sin) * scale,
                        180.0 + (ax * sin + ay * cos) * scale,
                        15,
                    )
                })
                .collect::<Vec<_>>()
                .join("");
            json!(format!(
                r#"<g class="cw-gun-body" transform="translate(180 180) rotate(-30) scale({scale}) translate({} {})">{}</g>{lines}"#,
                -cx,
                -cy,
                text(&args[2])
            ))
        }
        _ => return Err(format!("Unknown workshop operation: {method}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_modelled_and_explicit_exceptions_are_offered() {
        for i in game_core::constants()["ITEMS"].as_array().unwrap() {
            if available(i, "") {
                assert!(
                    models().get(text(&i["id"])).is_some() || text(&i["kind"]) == "mod"
                );
            }
        }
    }
    #[test]
    fn sych_uses_only_the_approved_helmet_placeholder_and_keeps_night_vision() {
        let sych = core("getItem", json!(["head-sych"])).unwrap();
        assert!(available(&sych, ""));
        assert_eq!(models()["head-sych"], models()["head-k6"]);
        assert_eq!(models()["head-sych"], "tripo-helmet");
        let mut kit = core("starterLoadout", json!(["rifle"])).unwrap();
        kit["head"] = json!("head-sych");
        assert_eq!(core("compileKit", json!([kit])).unwrap()["nvg"], true);
        // Batch-2 gear now has authored models in main. Its availability is
        // covered by only_modelled_and_explicit_exceptions_are_offered above.
    }
    #[test]
    fn mods_are_offered_only_where_their_gun_has_the_mesh() {
        let luch = json!({"id":"rail-luch","kind":"mod","fits":["pistol","shotgun","rifle"]});
        assert!(available(&luch, "rifle") && available(&luch, "pistol"));
        let luch_s = json!({"id":"rail-luch","kind":"mod","fits":["sniper"]});
        assert!(!available(&luch_s, "sniper"));
        let none = json!({"id":"rail-none","kind":"mod","fits":["sniper"]});
        assert!(!available(&none, ""));
        for (class, ids) in MOD_MESHES {
            for id in ids {
                let item = game_core::constants()["ITEMS"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|i| i["id"] == *id)
                    .cloned();
                let item = item.unwrap_or_else(|| panic!("{id} is not in the catalogue"));
                assert!(
                    item["fits"].as_array().unwrap().iter().any(|c| c == class),
                    "{id} does not fit {class}"
                );
            }
        }
    }
    #[test]
    fn option_text_is_escaped() {
        let result = options(
            &json!([{"item":{"id":"x\" onclick=\"evil","name":"<script>"},"selected":false,"icon":"<svg/>"}]),
            "weapon",
        );
        assert!(!result.contains("<script>"));
        assert!(result.contains("&lt;script&gt;"));
        assert!(result.contains("x&quot; onclick=&quot;evil"));
    }
}
