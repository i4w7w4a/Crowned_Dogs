//! The operator's base follows the region of the saved map: «Соль» in the salt mine of Белый Яр,
//! «Шлюз» inside the crest of the Каскад dam. The region is switched at the comms table on the
//! home screen, which saves the region's first map (`cd-map`), so the battle opens there too.
use super::{bay, platform};

pub struct Region {
    /// The catalogue's region (`REGIONS` in catalog.json).
    pub id: &'static str,
    /// The base's own name on the home screen.
    pub base: &'static str,
    /// Home: the depth-photo manifest and the plain picture under `img/`, and what it shows.
    pub home: &'static str,
    pub home_picture: &'static str,
    pub home_alt: &'static str,
    /// Where the home rings stand on the picture (u, v from the top left): equipment, battle,
    /// Persona and the region table.
    pub rings: [(&'static str, f64, f64); 4],
    /// Equipment room under `img/`: the picture, a sharper one for dense screens, its depth map.
    pub fitting: &'static str,
    pub fitting_sharp: &'static str,
    pub fitting_depth: &'static str,
    pub fitting_alt: &'static str,
    pub bay: &'static bay::Bay,
}

impl Region {
    /// The base's name in a sentence: «Шлюз».
    pub fn title(&self) -> String {
        let mut letters = self.base.chars();
        letters.next().into_iter().chain(letters.flat_map(char::to_lowercase)).collect()
    }
}

pub const REGIONS: [Region; 2] = [
    Region {
        id: "belyYar",
        base: "СОЛЬ",
        home: "base-depth/manifest.json",
        home_picture: "base/salt-base.webp",
        home_alt: "Убежище в соляной шахте: Персоны в ложементах, капсула оператора и узел очистки воды",
        rings: [
            (".cd-to-fitting", 0.275, 0.36),
            (".cd-enter", 0.235, 0.527),
            (".cd-to-persona", 0.76, 0.33),
            (".cd-to-region", 0.42, 0.73),
        ],
        fitting: "base/fitting-room-v2.png",
        fitting_sharp: "base/fitting-room-v2@2x.webp",
        fitting_depth: "base/fitting-room-v2-depth.png",
        fitting_alt: "Комната снаряжения в соляной шахте",
        bay: &bay::SALT,
    },
    Region {
        id: "kaskad",
        base: "ШЛЮЗ",
        home: "base-kaskad/home.json",
        home_picture: "base-kaskad/home.webp",
        home_alt: "Бункер в гребне плотины: кресло оператора у смотровой щели, стол с планшетами и рациями, Персона в ложементе",
        rings: [
            (".cd-to-fitting", 0.21, 0.36),
            (".cd-enter", 0.53, 0.43),
            (".cd-to-persona", 0.83, 0.3),
            (".cd-to-region", 0.3, 0.66),
        ],
        fitting: "base-kaskad/fitting.webp",
        fitting_sharp: "base-kaskad/fitting.webp",
        fitting_depth: "base-kaskad/fitting-depth.png",
        fitting_alt: "Галерея подъёмников затворов на гребне плотины, оружие и снаряжение между машинами",
        bay: &bay::SLUICE,
    },
];

/// The catalogue's maps of a region, in order.
pub fn maps(id: &str) -> Vec<String> {
    game_core::constants()["REGIONS"][id]["maps"]
        .as_array()
        .map(|maps| maps.iter().filter_map(|m| m.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// The catalogue's name of a region, such as «Каскад».
pub fn name(id: &str) -> String {
    game_core::constants()["REGIONS"][id]["name"]
        .as_str()
        .unwrap_or(id)
        .into()
}

/// The region a map belongs to; Белый Яр for no map or an unknown one.
pub fn of_map(map: &str) -> &'static Region {
    REGIONS
        .iter()
        .find(|r| maps(r.id).iter().any(|m| m == map))
        .unwrap_or(&REGIONS[0])
}

/// The region of the saved map.
pub fn current() -> &'static Region {
    of_map(&platform::store_get("cd-map").as_string().unwrap_or_default())
}

/// Moves the saved map into a region: its first map, unless the saved one is already there.
pub fn choose(id: &str) {
    if current().id == id {
        return;
    }
    if let Some(first) = maps(id).first() {
        platform::store_set("cd-map", first);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_base_names_a_catalogue_region_with_maps() {
        for region in &REGIONS {
            assert!(!maps(region.id).is_empty(), "{} has no maps", region.id);
            assert_ne!(name(region.id), region.id);
        }
        assert_eq!(of_map("dryBed").base, "ШЛЮЗ");
        assert_eq!(of_map("blocks").base, "СОЛЬ");
        assert_eq!(of_map("").base, "СОЛЬ");
        assert_eq!(REGIONS[1].title(), "Шлюз");
    }

    #[test]
    fn home_rings_stay_inside_a_phone_crop() {
        // A 390×844 phone shows about u 0.09–0.91 of a home picture.
        for region in &REGIONS {
            for (_, u, v) in region.rings {
                assert!((0.12..=0.88).contains(&u) && (0.1..=0.85).contains(&v), "{} ring at {u}, {v}", region.id);
            }
        }
    }
}
