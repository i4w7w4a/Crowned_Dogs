//! The operator's base follows the region of the saved map: «Соль» in the salt mine of Белый Яр,
//! «Шлюз» inside the crest of the Каскад dam. The region is switched at the comms table on the
//! home screen, which saves the region's first map (`cd-map`), so the battle opens there too.
use super::{bay, live::{Life, STILL}, platform};

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
    /// synchronisation, Persona and the region table.
    pub rings: [(&'static str, f64, f64); 5],
    /// Equipment room under `img/`: the picture, a sharper one for dense screens, its depth map.
    pub fitting: &'static str,
    pub fitting_sharp: &'static str,
    pub fitting_depth: &'static str,
    pub fitting_alt: &'static str,
    pub bay: &'static bay::Bay,
    /// What lives in the home picture and in the equipment room on high and ultra.
    pub home_life: &'static Life,
    pub fitting_life: &'static Life,
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
            (".cd-to-sync", 0.43, 0.64),
            (".cd-to-persona", 0.76, 0.33),
            (".cd-to-region", 0.74, 0.81),
        ],
        fitting: "base/fitting-room-v2.png",
        fitting_sharp: "base/fitting-room-v2@2x.webp",
        fitting_depth: "base/fitting-room-v2-depth.png",
        fitting_alt: "Комната снаряжения в соляной шахте",
        bay: &bay::SALT,
        home_life: &Life {
            lamps: &[
                [0.725, 0.156, 0.14, 0.45],
                [0.578, 0.221, 0.09, 0.35],
                [0.493, 0.262, 0.07, 0.5],
                [0.255, 0.242, 0.08, 0.4],
                [0.09, 0.247, 0.06, 0.3],
                [0.258, 0.327, 0.06, 0.55],
                [0.38, 0.339, 0.04, 0.3],
            ],
            screens: &[
                [0.0, 0.69, 0.08, 0.78],
                [0.13, 0.858, 0.37, 0.905],
                [0.3, 0.89, 0.56, 0.955],
            ],
            leds: &[
                [0.717, 0.891, 0.006, 2.3],
                [0.802, 0.854, 0.005, 3.1],
                [0.789, 0.911, 0.006, 1.7],
                [0.869, 0.872, 0.005, 2.7],
                [0.025, 0.378, 0.01, 4.3],
            ],
            steam: &[[0.225, 0.772, 0.02, 0.07]],
            drips: &[
                [0.78, 0.62, 5.3, 0.1],
                [0.88, 0.74, 7.1, 0.55],
                [0.6, 0.48, 6.2, 0.3],
                [0.93, 0.56, 4.7, 0.8],
            ],
            tube: [0.905, 0.348, 0.962, 0.421],
            wet: [0.5, 0.44, 1.0, 0.8],
            mist: [0.3, 0.22, 0.0, 0.0],
            ..STILL
        },
        fitting_life: &Life {
            lamps: &[
                [0.505, 0.162, 0.16, 0.3],
                [0.07, 0.14, 0.1, 0.4],
                [0.212, 0.262, 0.06, 0.5],
                [0.795, 0.27, 0.06, 0.3],
                [0.957, 0.357, 0.06, 0.4],
                [0.9, 0.506, 0.06, 0.45],
                [0.175, 0.42, 0.05, 0.35],
            ],
            wet: [0.05, 0.56, 0.95, 1.0],
            mist: [0.3, 0.15, 0.0, 0.0],
            ..STILL
        },
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
            (".cd-to-sync", 0.7, 0.58),
            (".cd-to-persona", 0.83, 0.3),
            (".cd-to-region", 0.3, 0.66),
        ],
        fitting: "base-kaskad/fitting.webp",
        fitting_sharp: "base-kaskad/fitting.webp",
        fitting_depth: "base-kaskad/fitting-depth.png",
        fitting_alt: "Галерея подъёмников затворов на гребне плотины, оружие и снаряжение между машинами",
        bay: &bay::SLUICE,
        home_life: &Life {
            lamps: &[
                [0.131, 0.238, 0.08, 0.45],
                [0.083, 0.332, 0.05, 0.3],
                [0.855, 0.233, 0.12, 0.35],
            ],
            screens: &[[0.145, 0.622, 0.38, 0.676], [0.245, 0.668, 0.5, 0.74]],
            leds: &[
                [0.117, 0.577, 0.005, 2.1],
                [0.157, 0.6, 0.005, 2.9],
                [0.176, 0.578, 0.005, 1.6],
                [0.202, 0.594, 0.005, 3.4],
                [0.123, 0.594, 0.005, 2.5],
            ],
            steam: &[[0.33, 0.574, 0.02, 0.07]],
            drips: &[
                [0.7, 0.62, 6.5, 0.2],
                [0.85, 0.75, 5.1, 0.6],
                [0.62, 0.8, 7.7, 0.4],
            ],
            wet: [0.55, 0.5, 1.0, 0.88],
            net: [0.29, 0.244, 0.8, 0.3],
            window: [0.292, 0.272, 0.8, 0.386],
            sun: [0.3, 0.44, 0.75, 0.62],
            ..STILL
        },
        fitting_life: &Life {
            lamps: &[
                [0.155, 0.255, 0.06, 0.4],
                [0.275, 0.316, 0.04, 0.3],
                [0.325, 0.343, 0.035, 0.5],
                [0.357, 0.37, 0.03, 0.3],
                [0.86, 0.157, 0.1, 0.35],
                [0.695, 0.25, 0.06, 0.3],
                [0.63, 0.3, 0.05, 0.45],
                [0.59, 0.325, 0.04, 0.3],
            ],
            window: [0.0, 0.12, 0.225, 0.36],
            sun: [0.0, 0.45, 0.62, 0.72],
            mist: [0.3, 0.15, 0.0, 0.0],
            ..STILL
        },
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
