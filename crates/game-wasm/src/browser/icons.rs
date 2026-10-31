//! Equipment slot art, drawn from the silhouette catalog in game-ui.
use std::sync::atomic::{AtomicU32, Ordering};
use wasm_bindgen::prelude::*;
/// Every icon gets its own cut mask id: one page shows the same item in several places.
static SERIAL: AtomicU32 = AtomicU32::new(0);
/// What an empty slot shows.
fn fallback(slot: &str) -> Option<&'static str> {
    Some(match slot {
        "head" => "head-k6",
        "body" => "body-plita2",
        "suit" => "suit-field",
        "boots" => "boots-bertsy",
        "knife" => "knife-std",
        "grenade" => "grenade-rg1",
        "weapon1" => "rifle",
        "weapon2" => "pistol",
        _ => return None,
    })
}
/// The item a slot draws: the worn item when it belongs in the slot, else the slot's default.
fn gear_id<'a>(slot: &str, id: &'a str) -> Option<&'a str> {
    let kind = id.split_once('-').map_or("", |(kind, _)| kind);
    let fits = match slot {
        "weapon1" | "weapon2" => matches!(kind, "pistol" | "rifle" | "shotgun" | "sniper"),
        _ => !kind.is_empty() && kind == slot,
    };
    if fits { Some(id) } else { fallback(slot) }
}
#[wasm_bindgen]
pub fn equipment_icon(slot: &str, id: Option<String>) -> String {
    let Some(item) = gear_id(slot, id.as_deref().unwrap_or("")) else {
        return String::new();
    };
    let mask = format!("gear-{}", SERIAL.fetch_add(1, Ordering::Relaxed));
    game_ui::icons::gear_svg(item, &mask)
        .or_else(|| game_ui::icons::gear_svg(fallback(slot)?, &mask))
        .unwrap_or_default()
}
#[wasm_bindgen]
pub fn equipment_item_icon(id: &str) -> String {
    let prefix = id.split('-').next().unwrap_or("");
    let slot = match prefix {
        "pistol" | "rifle" | "shotgun" | "sniper" => "weapon1",
        "head" | "body" | "suit" | "boots" | "knife" | "grenade" => prefix,
        _ => return String::new(),
    };
    equipment_icon(slot, Some(id.into()))
}
#[wasm_bindgen]
pub fn fitting_room_icon() -> String {
    equipment_icon("body", None)
}
#[wasm_bindgen]
pub fn navigation_back_arrow() -> String {
    "<svg class=\"cd-back-arrow\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M10 5.5 3.5 12l6.5 6.5M4 12h16.5\"/></svg>".into()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_secondary_weapon_uses_pistol() {
        assert_eq!(gear_id("weapon2", ""), Some("pistol"));
        assert_eq!(gear_id("weapon1", ""), Some("rifle"));
        assert_eq!(gear_id("weapon2", "rifle-ak"), Some("rifle-ak"));
        assert_eq!(gear_id("head", "body-light"), Some("head-k6"));
        assert_eq!(gear_id("pocket", "head-cap"), None);
    }
    #[test]
    fn own_art_per_item_and_unknown_items() {
        assert_ne!(
            game_ui::icons::gear_svg("suit-ten", "m"),
            game_ui::icons::gear_svg("suit-solonchak", "m")
        );
        assert_eq!(equipment_item_icon("unknown-1"), "");
        assert!(equipment_item_icon("head-cap").contains("currentColor"));
        // A helmet nobody drew yet still shows a helmet.
        assert!(equipment_item_icon("head-new").contains("cd-equipment-icon"));
        assert_ne!(equipment_item_icon("head-cap"), equipment_item_icon("head-cap"));
    }
}
