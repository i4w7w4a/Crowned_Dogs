//! The capsule's «Синхронизация» screen: level ring, stat points and weapon mastery.
//! The profile lives on the device under `cd-sync`; fights add to it in `battle::finish`.
use super::platform;
use serde_json::{json, Value};

const KEY: &str = "cd-sync";

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
/// One free point into `stat`; true when it was spent.
pub fn spend(stat: &str) -> bool {
    let before = load();
    let after = core("syncSpend", json!([before, stat]));
    if after == before {
        return false;
    }
    platform::store_set(KEY, &after.to_string());
    true
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
pub fn markup() -> String {
    let v = core("syncView", json!([load()]));
    let free = num(&v["free"]);
    let xp = if v["need"].is_null() {
        "Высший уровень".to_string()
    } else {
        format!("Опыт {} / {}", num(&v["xp"]), num(&v["need"]))
    };
    let mut out = format!(
        "<div class=\"cd-sync-head\"><div class=\"cd-sync-ring\" style=\"--p:{:.3}\"><svg viewBox=\"0 0 120 120\" aria-hidden=\"true\"><circle class=\"cd-sync-track\" cx=\"60\" cy=\"60\" r=\"52\"/><circle class=\"cd-sync-arc\" cx=\"60\" cy=\"60\" r=\"52\" pathLength=\"100\"/></svg><div><b>{}</b><span>уровень</span></div></div>\
         <div class=\"cd-sync-sum\"><p class=\"cd-overline\">СИНХРОНИЗАЦИЯ</p><h2>{}</h2><p>{xp}</p><p class=\"cd-sync-free\">Свободные очки: <b>{free}</b></p></div></div>\
         <p class=\"cd-overline cd-sync-title\">ХАРАКТЕРИСТИКИ</p><div class=\"cd-sync-stats\">",
        num(&v["progress"]),
        num(&v["level"]),
        text(&v["sync"]),
    );
    for s in v["stats"].as_array().into_iter().flatten() {
        let points = num(&s["points"]);
        out.push_str(&format!(
            "<div class=\"glass cd-sync-stat\"><div class=\"cd-sync-stat-top\"><div><strong>{name}</strong><small>{about}</small></div><b>{points}</b>\
             <button class=\"glass cd-sync-plus\" type=\"button\" data-stat=\"{id}\" aria-label=\"{name}: вложить очко\"{off}>+</button></div>\
             <p>{now}</p><p class=\"cd-sync-next\">+1 очко: {next}</p></div>",
            name = text(&s["name"]),
            about = text(&s["about"]),
            id = text(&s["id"]),
            off = if free > 0.0 { "" } else { " disabled" },
            now = if points > 0.0 {
                joined(&s["lines"])
            } else {
                "очков нет".into()
            },
            next = joined(&s["next"]),
        ));
    }
    out.push_str("</div><p class=\"cd-overline cd-sync-title\">МАСТЕРСТВО ОРУЖИЯ</p><div class=\"glass cd-sync-mastery\">");
    for m in v["mastery"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "<div class=\"cd-sync-bar\"><span>{}</span><i style=\"--p:{:.3}\"></i><b>{}</b><small>{}</small></div>",
            text(&m["name"]),
            num(&m["share"]),
            text(&m["text"]),
            if num(&m["share"]) > 0.0 {
                joined(&m["lines"])
            } else {
                String::new()
            },
        ));
    }
    out.push_str("</div><p class=\"cd-sync-note\">Опыт идёт за каждый бой, за уровень даётся 3 очка. Мастерство растёт от 74% до 100% только у оружия, которым воюешь.</p>");
    out
}
