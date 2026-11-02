//! The capsule's «Синхронизация» screen, shown on a field tablet: level ring, sync pulse, stat radar,
//! stat points and weapon mastery. The profile lives on the device under `cd-sync`; fights add to it
//! in `battle::finish`.
use super::{icons, platform};
use serde_json::{Value, json};

const KEY: &str = "cd-sync";
/// Radar axes, clockwise from the top.
const AXES: [&str; 4] = ["power", "cooling", "instinct", "reflexes"];
const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

fn core(method: &str, args: Value) -> Value {
    game_core::dispatch(method, args).unwrap_or(Value::Null)
}
pub fn load() -> Value {
    let saved = platform::store_get(KEY)
        .as_string()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(Value::Null);
    core("syncClean", json!([saved]))
}
/// One free point into `stat`; the profile as it was before, when the point was spent.
pub fn spend(stat: &str) -> Option<Value> {
    let before = load();
    let after = core("syncSpend", json!([before, stat]));
    if after == before {
        return None;
    }
    platform::store_set(KEY, &after.to_string());
    Some(before)
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}
fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}
fn joined(v: &Value) -> String {
    v.as_array()
        .map(|l| l.iter().map(text).collect::<Vec<_>>().join(" · "))
        .unwrap_or_default()
}
/// The tier-1 gun each mastery row draws.
fn gun(class: &str) -> &'static str {
    match class {
        "pistol" => "pistol-pustelga",
        "shotgun" => "shotgun-taran",
        "rifle" => "rifle-varyag",
        _ => "sniper-berkut",
    }
}
fn stat<'a>(view: &'a Value, id: &str) -> &'a Value {
    view["stats"]
        .as_array()
        .and_then(|l| l.iter().find(|s| text(&s["id"]) == id))
        .unwrap_or(&Value::Null)
}
/// Radar corners for the four stat shares (0..1 each), in a 200×200 box.
fn shape(view: Option<&Value>) -> [(f64, f64); 4] {
    let r = |i: usize| {
        8.0 + view
            .map_or(0.0, |v| num(&stat(v, AXES[i])["share"]))
            .clamp(0.0, 1.0)
            * 70.0
    };
    [
        (100.0, 100.0 - r(0)),
        (100.0 + r(1), 100.0),
        (100.0, 100.0 + r(2)),
        (100.0 - r(3), 100.0),
    ]
}
fn points(shape: &[(f64, f64); 4]) -> String {
    shape
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ")
}
/// SMIL tween from the old value to the new one after `delay` of `total` seconds.
fn tween(attr: &str, from: &str, to: &str, delay: f64, total: f64) -> String {
    format!(
        "<animate attributeName=\"{attr}\" values=\"{from};{from};{to}\" keyTimes=\"0;{:.3};1\" dur=\"{total}s\" calcMode=\"spline\" keySplines=\"0 0 1 1;.2 .8 .2 1\" fill=\"freeze\"/>",
        delay / total
    )
}
/// A heartbeat line whose spikes grow with the sync level, four beats so it can scroll seamlessly.
fn pulse(amp: f64) -> String {
    let mut d = String::from("M0 16");
    for k in 0..4 {
        let x = k as f64 * 100.0;
        d += &format!(
            " L{} 16 Q{} {:.1} {} 16 L{} 16 L{} {:.1} L{} {:.1} L{} 16 L{} 16 Q{} {:.1} {} 16 L{} 16",
            x + 22.0,
            x + 28.0,
            16.0 - 4.0 * amp,
            x + 34.0,
            x + 42.0,
            x + 46.0,
            16.0 - 14.0 * amp,
            x + 50.0,
            16.0 + 8.0 * amp,
            x + 54.0,
            x + 64.0,
            x + 72.0,
            16.0 - 6.0 * amp,
            x + 80.0,
            x + 100.0
        );
    }
    d
}
/// The screen's content. `before` is the profile before a point was spent: the radar and that stat's bar
/// move from it. Without it the screen boots: everything counts up and draws in.
pub fn markup(before: Option<&Value>) -> String {
    let v = core("syncView", json!([load()]));
    let old = before.map(|b| core("syncView", json!([b])));
    let boot = old.is_none();
    let free = num(&v["free"]);
    let sync_text = text(&v["sync"]);
    let sync: f64 = sync_text
        .trim_end_matches('%')
        .replace(',', ".")
        .parse()
        .unwrap_or(74.0);
    let (whole, tenth) = (sync.floor(), ((sync - sync.floor()) * 10.0).round());
    let amp = (0.45 + (sync - 74.0) / 26.0 * 0.55).clamp(0.45, 1.0);
    let progress = num(&v["progress"]);
    let xp = if v["need"].is_null() {
        "максимум".to_string()
    } else {
        format!("{} / {}", num(&v["xp"]), num(&v["need"]))
    };
    let ring = if boot {
        tween(
            "stroke-dasharray",
            "0 100",
            &format!("{:.1} 100", progress * 100.0),
            0.45,
            1.6,
        )
    } else {
        String::new()
    };
    let mut out = format!(
        "<div class=\"cd-tab-in{boot}\">\
         <div class=\"cd-tab-bar\"><span class=\"cd-tab-live\"><i></i>Связь с Персоной</span></div>\
         <section class=\"cd-tab-hero cd-tab-part\" style=\"--i:0\">\
         <div class=\"cd-sync-ring\" style=\"--p:{progress:.3}\" role=\"img\" aria-label=\"Уровень {level}\"><svg viewBox=\"0 0 120 120\" aria-hidden=\"true\">\
         <circle class=\"cd-ring-ticks\" cx=\"60\" cy=\"60\" r=\"57\" pathLength=\"120\"/><circle class=\"cd-sync-track\" cx=\"60\" cy=\"60\" r=\"48\"/>\
         <circle class=\"cd-sync-arc\" cx=\"60\" cy=\"60\" r=\"48\" pathLength=\"100\" stroke-dasharray=\"{dash:.1} 100\">{ring}</circle></svg>\
         <div aria-hidden=\"true\"><b class=\"cd-count\" style=\"--to:{level}\"></b><span>уровень</span></div></div>\
         <div class=\"cd-tab-sum\"><p class=\"cd-tab-label\">Синхронизация</p>\
         <h2 class=\"cd-tab-pct\" aria-label=\"{sync_text}\"><span class=\"cd-pct{whole_class}\" style=\"--a:{whole};--b:{tenth}\" aria-hidden=\"true\"></span></h2>\
         <svg class=\"cd-tab-wave\" viewBox=\"0 0 200 28\" preserveAspectRatio=\"none\" aria-hidden=\"true\"><path class=\"cd-wave-move\" d=\"{wave}\"/></svg>\
         <p class=\"cd-tab-xp\"><span>Опыт</span><b>{xp}</b></p><i class=\"cd-tab-xpbar\" style=\"--p:{progress:.3}\"></i></div></section>",
        boot = if boot { " cd-tab-boot" } else { "" },
        level = num(&v["level"]),
        dash = progress * 100.0,
        whole_class = if tenth > 0.0 && tenth < 10.0 {
            ""
        } else {
            " cd-whole"
        },
        wave = pulse(amp),
    );
    // Radar: the stat shape grows from the centre on boot, or from the old shape after a spend.
    let now = shape(Some(&v));
    let was = shape(old.as_ref());
    let delay = if boot { 0.7 } else { 0.0 };
    let total = delay + 0.8;
    out.push_str("<section class=\"cd-tab-radar cd-tab-part\" style=\"--i:1\"><svg viewBox=\"0 0 200 200\" aria-hidden=\"true\"><g class=\"cd-radar-grid\">");
    for k in 1..=4 {
        let r = 78.0 * k as f64 / 4.0;
        out.push_str(&format!(
            "<polygon points=\"100,{:.1} {:.1},100 100,{:.1} {:.1},100\"/>",
            100.0 - r,
            100.0 + r,
            100.0 + r,
            100.0 - r
        ));
    }
    out.push_str(&format!(
        "<path d=\"M100 22V178M22 100H178\"/></g><polygon class=\"cd-radar-shape\" points=\"{}\">{}</polygon>",
        points(&now),
        tween("points", &points(&was), &points(&now), delay, total)
    ));
    for (i, (x, y)) in now.iter().enumerate() {
        let (ox, oy) = was[i];
        let anim = if i % 2 == 0 {
            tween("cy", &format!("{oy:.1}"), &format!("{y:.1}"), delay, total)
        } else {
            tween("cx", &format!("{ox:.1}"), &format!("{x:.1}"), delay, total)
        };
        out.push_str(&format!(
            "<circle class=\"cd-radar-dot\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"3\">{anim}</circle>"
        ));
    }
    out.push_str("</svg>");
    for (i, id) in AXES.iter().enumerate() {
        let s = stat(&v, id);
        out.push_str(&format!(
            "<span class=\"cd-radar-l cd-radar-{}\">{}<b>{}</b></span>",
            SIDES[i],
            text(&s["name"]),
            num(&s["points"])
        ));
    }
    out.push_str(&format!(
        "</section><p class=\"cd-tab-free{}\"><b>{free}</b>свободных очков</p>",
        if free > 0.0 { " on" } else { "" }
    ));
    // Stat rows: a 20-segment bar shows how far the stat's effect has saturated.
    out.push_str("<section class=\"cd-tab-part\" style=\"--i:2\"><p class=\"cd-tab-title\">Характеристики</p>");
    for (i, id) in AXES.iter().enumerate() {
        let s = stat(&v, id);
        let points = num(&s["points"]);
        let prior = old.as_ref().map(|o| stat(o, id));
        let gained = prior.is_some_and(|p| num(&p["points"]) < points);
        out.push_str(&format!(
            "<div class=\"cd-tab-stat{gain}\" style=\"--i:{i}\"><div class=\"cd-tab-stat-head\"><div><strong>{name}</strong><small>{about}</small></div><b>{points}</b>\
             <button class=\"glass cd-sync-plus\" type=\"button\" data-stat=\"{id}\" aria-label=\"{name}: вложить очко\"{off}>+</button></div>\
             <i class=\"cd-tab-seg\" style=\"--n:{share:.3};--from:{from:.3}\"></i><p>{now}</p><p class=\"cd-sync-next\">+1 очко: {next}</p></div>",
            gain = if gained { " cd-gain" } else { "" },
            name = text(&s["name"]),
            about = text(&s["about"]),
            off = if free > 0.0 { "" } else { " disabled" },
            share = num(&s["share"]),
            from = prior.map_or(0.0, |p| num(&p["share"])),
            now = if points > 0.0 { joined(&s["lines"]) } else { "очков нет".into() },
            next = joined(&s["next"]),
        ));
    }
    out.push_str("</section><section class=\"cd-tab-part\" style=\"--i:3\"><p class=\"cd-tab-title\">Мастерство оружия</p>");
    for (i, m) in v["mastery"].as_array().into_iter().flatten().enumerate() {
        let share = num(&m["share"]);
        out.push_str(&format!(
            "<div class=\"cd-tab-gun\" style=\"--i:{}\"><span class=\"cd-tab-gun-icon\" aria-hidden=\"true\">{}</span><div><div class=\"cd-tab-gun-head\"><strong>{}</strong><b>{}</b></div>\
             <i class=\"cd-tab-seg cd-tab-seg26\" style=\"--n:{share:.3};--from:0\"></i><small>{}</small></div></div>",
            i + 4,
            icons::equipment_item_icon(gun(text(&m["id"]))),
            text(&m["name"]),
            text(&m["text"]),
            if share > 0.0 { joined(&m["lines"]) } else { "растёт в боях с этим оружием".into() },
        ));
    }
    out.push_str("</section><p class=\"cd-sync-note\">Опыт идёт за каждый бой, за уровень 3 очка. Чем больше очков в одной характеристике, тем меньше даёт следующее. Мастерство растёт от 74% до 100% только у оружия, которым воюешь.</p></div>");
    out
}
