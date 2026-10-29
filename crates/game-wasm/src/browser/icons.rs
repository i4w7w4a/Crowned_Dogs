//! Equipment variant selection. The original authored SVG artwork is a data asset.
use std::sync::LazyLock;
use wasm_bindgen::prelude::*;
static ART: LazyLock<serde_json::Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("equipment-art.json")).expect("equipment artwork")
});
fn variant<'a>(slot: &'a str, id: &str) -> &'a str {
    match slot {
        "weapon1" | "weapon2" => {
            if id.starts_with("pistol-") {
                "pistol"
            } else if id.starts_with("shotgun-") {
                "shotgun"
            } else if id.starts_with("sniper-") {
                "sniper"
            } else if id.starts_with("rifle-") {
                "rifle"
            } else if slot == "weapon2" {
                "pistol"
            } else {
                "rifle"
            }
        }
        "head" => match id {
            "head-cap" => "cap",
            "head-sfera" => "visor",
            "head-marevo" => "thermalHelmet",
            "head-filin" => "heavyHelmet",
            "head-sych" => "nightHelmet",
            _ => "helmet",
        },
        "body" => match id {
            "body-light" => "lightVest",
            "body-plita4" => "heavyVest",
            "body-bastion" => "assaultVest",
            _ => "vest",
        },
        "suit" => match id {
            "suit-leshy" => "cloak",
            "suit-ryvok" => "assaultSuit",
            "suit-ten" | "suit-solonchak" => "thermalSuit",
            _ => "suit",
        },
        "boots" => match id {
            "boots-sprint" => "lightBoot",
            "boots-saper" | "boots-hodok" => "armoredBoot",
            _ => "boot",
        },
        "knife" => match id {
            "knife-kogot" => "curvedKnife",
            "knife-shtyk" => "bayonet",
            "knife-zhalo" => "heavyKnife",
            "knife-iskra" => "thermalKnife",
            _ => "knife",
        },
        "grenade" => match id {
            "grenade-emp" => "emp",
            "grenade-smoke" => "smoke",
            "grenade-grad" => "heavyGrenade",
            _ => "grenade",
        },
        _ => "",
    }
}
#[wasm_bindgen]
pub fn equipment_icon(slot: &str, id: Option<String>) -> String {
    ART[variant(slot, id.as_deref().unwrap_or(""))]
        .as_str()
        .unwrap_or("")
        .to_owned()
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
    "<svg class=\"cd-back-arrow\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m10 5-7 7 7 7M3 12h18\"/></svg>".into()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_secondary_weapon_uses_pistol() {
        assert_eq!(variant("weapon2", ""), "pistol");
        assert_eq!(variant("weapon1", ""), "rifle");
        assert_eq!(variant("weapon2", "rifle-ak"), "rifle");
    }
    #[test]
    fn shared_variants_and_unknown_items() {
        assert_eq!(
            equipment_item_icon("suit-ten"),
            equipment_item_icon("suit-solonchak")
        );
        assert_eq!(
            equipment_item_icon("boots-saper"),
            equipment_item_icon("boots-hodok")
        );
        assert_eq!(equipment_item_icon("unknown-1"), "");
        assert!(equipment_item_icon("head-cap").contains("currentColor"));
    }
}
