//! The shared game clock. One game day lasts one real hour, counted from UTC,
//! so every device sees the same hour. Each map keeps its own hour offset, which
//! spreads the four times of day over the maps at any moment. Weather changes
//! every four game hours from the odds of the region's event of the day.
//! The clock also shuts maps for a while: the region's event, a map's own hours,
//! the water released at the dam and the shelling that moves round the region
//! (`MAP_ACCESS` in the catalogue). A region's home map never shuts.
use crate::text;
use game_core::{constants, rng};
use serde_json::{json, Value};

/// Real milliseconds in one game day.
pub const DAY_MS: f64 = 3_600_000.0;
const DAY_MINUTES: f64 = 1440.0;
/// Weather holds for four game hours (ten real minutes).
const WEATHER_MINUTES: f64 = 240.0;

/// Game hours each map runs ahead of the shared clock. The salt hall and the
/// water intake are the reference; in each region the other three maps sit a
/// quarter, a half and three quarters of a day off.
fn map_offset(map: &str) -> f64 {
    match map {
        "blocks" | "backwater" => 12.0,
        "dam" | "railyard" => 6.0,
        "dryBed" | "sludgePonds" => 18.0,
        _ => 0.0,
    }
}

/// Covered maps (a roof, or the intake's gallery) never get rain inside; lightning still
/// shows through the roof and the openings. The cover is in `MAP_WEATHER`.
fn indoor(map: &str) -> bool {
    constants()["MAP_WEATHER"][map]["cover"]
        .as_str()
        .is_some_and(|c| c != "open")
}

/// The weather's name on this map: the salt hall's dust is a salt haze, the ponds' fog is fumes.
fn weather_name(map: &str, weather: &str) -> Value {
    let c = constants();
    let own = &c["MAP_WEATHER"][map]["weather"][weather]["name"];
    if own.is_string() {
        own.clone()
    } else {
        c["WEATHER"][weather]["name"].clone()
    }
}

/// The colour of a map's own fog or dust, by day: smoke under a roof, mist off the water,
/// salt in the air of the hall, yellow fumes over the sludge ponds.
fn tint(map: &str, weather: &str) -> Option<u32> {
    match (map, weather) {
        ("saltHall" | "railyard", "fog") => Some(0x8f8a84),
        ("saltHall", "dust") => Some(0xd3d6d4),
        ("dryBed", "dust") => Some(0xd0d0c8),
        ("dam" | "intake" | "backwater", "fog") => Some(0xa7b2bb),
        ("sludgePonds", "fog") => Some(0x9fa184),
        ("sludgePonds", "dust") => Some(0xbdb98f),
        _ => None,
    }
}

/// Where each time of day starts, in game minutes. Four equal quarters.
const PHASES: [(&str, f64); 4] = [
    ("morning", 4.0 * 60.0),
    ("day", 10.0 * 60.0),
    ("evening", 16.0 * 60.0),
    ("night", 22.0 * 60.0),
];

/// How often each regional event comes up for a game day. Calm is the usual day.
const EVENT_WEIGHTS: [(&str, f64); 4] = [
    ("calm", 40.0),
    ("storms", 20.0),
    ("dryWind", 20.0),
    ("chemical", 20.0),
];

fn phase_at(minute: f64) -> (&'static str, f64) {
    // Night wraps past midnight: before 04:00 it is still last night.
    let mut current = PHASES[3];
    for p in PHASES {
        if minute >= p.1 {
            current = p;
        }
    }
    let next = PHASES
        .iter()
        .map(|p| p.1)
        .map(|start| {
            if start > minute {
                start - minute
            } else {
                start + DAY_MINUTES - minute
            }
        })
        .fold(f64::INFINITY, f64::min);
    (current.0, next)
}

/// The region's maps, home first, then the ones still being drawn.
fn region_maps(region: &str) -> Vec<String> {
    let c = constants();
    let drawn = c["REGIONS"][region]["maps"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let planned = c["MAP_ACCESS"]["planned"][region].as_object();
    drawn
        .iter()
        .map(|m| text(m).to_string())
        .chain(planned.into_iter().flat_map(|p| p.keys().cloned()))
        .collect()
}

fn region_of(map: &str) -> String {
    constants()["REGIONS"]
        .as_object()
        .and_then(|all| {
            all.keys()
                .find(|r| region_maps(r).iter().any(|m| m == map))
        })
        .cloned()
        .unwrap_or_else(|| "belyYar".into())
}

fn listed(list: &Value, id: &str) -> bool {
    list.as_array()
        .is_some_and(|l| l.iter().any(|v| text(v) == id))
}

/// Why the world itself shuts `map` at shared game minute `shared`: the region's
/// event of the day, the map's own hours or the water released at the dam.
fn shut_by_world(map: &str, region: &str, shared: f64) -> Option<String> {
    let a = &constants()["MAP_ACCESS"];
    let event = event_for(region, (shared / DAY_MINUTES).floor() as i64);
    if listed(&a["events"][event]["maps"], map) {
        return Some(event.into());
    }
    let minute = (shared + map_offset(map) * 60.0).rem_euclid(DAY_MINUTES);
    if listed(&a["hours"][map]["shut"], phase_at(minute).0) {
        return Some("hours".into());
    }
    let release = &a["release"];
    if text(&release["map"]) == map {
        // The release keeps the dam's own clock.
        let clock = (shared + map_offset(text(&release["clock"])) * 60.0).rem_euclid(DAY_MINUTES);
        let length = release["minutes"].as_f64().unwrap_or(0.0);
        let starts = release["starts"].as_array().cloned().unwrap_or_default();
        if starts
            .iter()
            .any(|s| (clock - s.as_f64().unwrap_or(0.0)).rem_euclid(DAY_MINUTES) < length)
        {
            return Some("release".into());
        }
    }
    None
}

/// Game minutes the guns stay on one map.
fn shelling_minutes() -> f64 {
    constants()["MAP_ACCESS"]["shellingMinutes"]
        .as_f64()
        .unwrap_or(360.0)
}

/// The map under fire during shelling turn `turn`. The guns move round the
/// region's maps other than home, one map a turn, so each of them is under fire
/// a third of the time. They do not skip a map the world has shut already:
/// skipping piled the fire onto the maps that stay open.
fn shelled(region: &str, turn: i64) -> Option<String> {
    let targets: Vec<String> = region_maps(region).into_iter().skip(1).collect();
    let n = targets.len() as i64;
    if n == 0 {
        return None;
    }
    let shift = (rng::next(&mut rng::seed(&format!("world:shelling:{region}"))) * n as f64) as i64;
    targets.get((turn + shift).rem_euclid(n) as usize).cloned()
}

/// Why `map` is shut at shared game minute `shared`, or None while it is open.
fn closure(map: &str, region: &str, shared: f64) -> Option<String> {
    if region_maps(region).first().is_none_or(|home| home == map) {
        return None;
    }
    shut_by_world(map, region, shared).or_else(|| {
        let turn = (shared / shelling_minutes()).floor() as i64;
        (shelled(region, turn).as_deref() == Some(map)).then(|| "shelling".into())
    })
}

/// What the player reads for a closure.
fn closed_name(reason: &str, map: &str) -> Value {
    let a = &constants()["MAP_ACCESS"];
    match reason {
        "shelling" => a["shellingWhy"].clone(),
        "hours" => a["hours"][map]["why"].clone(),
        "release" => a["release"]["why"].clone(),
        event => a["events"][event]["why"].clone(),
    }
}

/// When `map` next opens or shuts, and why it shuts then. Every rule turns on a
/// whole game hour, so the hours ahead are checked one by one, three days out.
fn next_change(
    map: &str,
    region: &str,
    shared: f64,
    now: &Option<String>,
) -> Option<(f64, Option<String>)> {
    let hour = (shared / 60.0).floor();
    (1..=72).map(|k| (hour + k as f64) * 60.0).find_map(|start| {
        let then = closure(map, region, start);
        (then.is_some() != now.is_some()).then_some((start, then))
    })
}

/// A map's name, drawn or still planned.
fn map_name(map: &str, region: &str) -> Value {
    let c = constants();
    let drawn = &c["MAPS"][map]["name"];
    if drawn.is_string() {
        drawn.clone()
    } else {
        c["MAP_ACCESS"]["planned"][region][map].clone()
    }
}

fn weighted<'a>(seed: &str, items: impl Iterator<Item = (&'a str, f64)> + Clone) -> &'a str {
    let mut random = rng::seed(seed);
    let total: f64 = items.clone().map(|(_, w)| w).sum();
    let mut roll = rng::next(&mut random) * total;
    let mut last = "";
    for (id, w) in items {
        last = id;
        roll -= w;
        if roll < 0.0 {
            return id;
        }
    }
    last
}

fn event_for(region: &str, day: i64) -> &'static str {
    weighted(
        &format!("world:event:{region}:{day}"),
        EVENT_WEIGHTS.into_iter(),
    )
}

fn weather_for(map: &str, region: &str, window: i64, day: i64) -> String {
    let event = event_for(region, day);
    let odds = &constants()["REGION_EVENTS"][event]["odds"];
    let ids: Vec<(String, f64)> = constants()["WEATHERS"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|w| (text(w).to_string(), odds[text(w)].as_f64().unwrap_or(0.0)))
                .collect()
        })
        .unwrap_or_default();
    weighted(
        &format!("world:weather:{map}:{window}"),
        ids.iter().map(|(id, w)| (id.as_str(), *w)),
    )
    .to_string()
}

fn clock_text(minute: f64) -> String {
    let m = minute.floor() as i64;
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// Everything a map shows at real time `ms` (milliseconds since the Unix epoch).
pub fn at(ms: f64, map: &str) -> Value {
    let region = region_of(map);
    // Game minutes since the epoch, shifted to this map's own hour.
    let shared = ms / DAY_MS * DAY_MINUTES;
    let local = shared + map_offset(map) * 60.0;
    let minute = local.rem_euclid(DAY_MINUTES);
    let (time, to_next) = phase_at(minute);
    let next_time = phase_at((minute + to_next + 0.5).rem_euclid(DAY_MINUTES)).0;
    // The region's event belongs to the shared day, so both maps of a region agree.
    let day = (shared / DAY_MINUTES).floor() as i64;
    let window = (local / WEATHER_MINUTES).floor() as i64;
    let weather = weather_for(map, &region, window, day);
    let window_end = (window + 1) as f64 * WEATHER_MINUTES;
    let next_shared = window_end - map_offset(map) * 60.0;
    let next_day = (next_shared / DAY_MINUTES).floor() as i64;
    let next_weather = weather_for(map, &region, window + 1, next_day);
    let to_minutes = |game: f64| game / DAY_MINUTES * DAY_MS;
    let event = event_for(&region, day);
    let closed = closure(map, &region, shared);
    let change = next_change(map, &region, shared, &closed);
    let next_closed = change.as_ref().and_then(|(_, why)| why.clone());
    let turn = (shared / shelling_minutes()).floor() as i64;
    let c = constants();
    json!({
        "mapId": map,
        "mapName": map_name(map, &region),
        "regionId": region,
        "minute": minute,
        "clock": clock_text(minute),
        "time": time,
        "timeName": c["TIME_NAMES"][time],
        "nextTime": next_time,
        "nextTimeName": c["TIME_NAMES"][next_time],
        "nextTimeInMs": to_minutes(to_next),
        "weather": weather,
        "weatherName": weather_name(map, &weather),
        "nextWeather": next_weather,
        "nextWeatherName": weather_name(map, &next_weather),
        "nextWeatherInMs": to_minutes(window_end - local),
        "event": event,
        "eventName": c["REGION_EVENTS"][event]["name"],
        "indoor": indoor(map),
        "toxic": constants()["MAP_WEATHER"][map]["weather"][&weather]["toxic"].as_f64().unwrap_or(0.0) > 0.0,
        "home": region_maps(&region).first().is_some_and(|h| h == map),
        "open": closed.is_none(),
        "closedName": closed.as_deref().map(|why| closed_name(why, map)),
        "closed": closed,
        "changeInMs": change.as_ref().map(|(start, _)| to_minutes(start - shared)),
        "nextClosedName": next_closed.as_deref().map(|why| closed_name(why, map)),
        "nextClosed": next_closed,
        "shelled": shelled(&region, turn),
        "nextShelled": shelled(&region, turn + 1),
        "shellingMovesInMs": to_minutes((turn + 1) as f64 * shelling_minutes() - shared),
    })
}

fn rgb(hex: u32) -> Value {
    json!([
        ((hex >> 16) & 255) as f64 / 255.0,
        ((hex >> 8) & 255) as f64 / 255.0,
        (hex & 255) as f64 / 255.0
    ])
}

/// What the lens draws for a weather: fog and dust strength, rain and lightning,
/// a dimming of the light and the colours, tuned per time of day. Clear has no
/// weather pass, only heat shimmer over sun-warmed ground. `surge` is how far the
/// dust gusts close the view, `wet` how much rain beads the lens, the gun and the ground.
pub fn fx(weather: &str, time: &str, map: &str) -> Value {
    let night = time == "night";
    let (fog_color, dust_color, rain_color) = match time {
        "morning" => (0x9ba6b1, 0xb4bbc0, 0xb7c2cc),
        "evening" => (0x978d85, 0xc9b9a6, 0xb4aaa2),
        "night" => (0x27303a, 0x3e4750, 0x55606a),
        _ => (0xa3a9ad, 0xbfc1bd, 0xc3c9ce),
    };
    let outdoor = !indoor(map);
    // The map's own fog or dust colour, darker at dusk and at night like the plain ones.
    let (fog_color, dust_color) = match tint(map, weather) {
        Some(day) => {
            let k = match time {
                "night" => 0.3,
                "evening" | "morning" => 0.85,
                _ => 1.0,
            };
            let shade = |hex: u32| {
                let ch = |sh: u32| ((((hex >> sh) & 255) as f64 * k).round() as u32).min(255) << sh;
                ch(16) | ch(8) | ch(0)
            };
            if weather == "fog" {
                (shade(day), dust_color)
            } else {
                (fog_color, shade(day))
            }
        }
        None => (fog_color, dust_color),
    };
    let (fog, dust, rain, flash, dim, haze) = match weather {
        "fog" => (if night { 0.75 } else { 0.78 }, 0.0, 0.0, 0.0, 0.94, 3.0),
        "dust" => (0.1, if night { 0.6 } else { 0.8 }, 0.0, 0.0, 0.94, 2.4),
        "storm" => (
            0.18,
            0.0,
            if outdoor { 0.85 } else { 0.0 },
            if night { 1.0 } else { 0.55 },
            if night { 0.9 } else { 0.8 },
            1.4,
        ),
        _ => (0.0, 0.0, 0.0, 0.0, 1.0, 1.0),
    };
    let clear = weather == "clear" || weather.is_empty();
    let sun = match time {
        "day" => 1.0,
        "evening" => 0.55,
        "morning" => 0.3,
        _ => 0.0,
    };
    let heat = if clear {
        sun * if outdoor { 1.0 } else { 0.35 }
    } else {
        0.0
    };
    let surge = match (weather, outdoor) {
        ("dust", true) => 0.95,
        ("dust", false) => 0.7,
        _ => 0.0,
    };
    json!({
        "active": !clear,
        "fog": fog, "dust": dust, "rain": rain, "flash": flash, "dim": dim, "haze": haze,
        "heat": heat, "surge": surge, "wet": rain,
        "fogColor": rgb(fog_color), "dustColor": rgb(dust_color), "rainColor": rgb(rain_color),
    })
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "client.world.at" => at(
            args[0].as_f64().ok_or("World clock needs a time")?,
            text(&args[1]),
        ),
        "client.world.maps" => {
            let ms = args[0].as_f64().ok_or("World clock needs a time")?;
            let maps = constants()["MAPS"].as_object().ok_or("No maps")?;
            json!(maps.keys().map(|m| at(ms, m)).collect::<Vec<_>>())
        }
        "client.world.region" => {
            let ms = args[0].as_f64().ok_or("World clock needs a time")?;
            json!(region_maps(text(&args[1]))
                .iter()
                .map(|m| at(ms, m))
                .collect::<Vec<_>>())
        }
        "client.world.fx" => fx(text(&args[0]), text(&args[1]), text(&args[2])),
        "client.world.weatherName" => weather_name(text(&args[0]), text(&args[1])),
        _ => return Err(format!("Unknown world operation: {method}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const HOUR: f64 = DAY_MS;

    #[test]
    fn one_real_hour_is_one_game_day() {
        let start = 1_800_000_000_000.0_f64.div_euclid(HOUR) * HOUR;
        assert_eq!(at(start, "saltHall")["clock"], "00:00");
        assert_eq!(at(start + HOUR / 2.0, "saltHall")["clock"], "12:00");
        assert_eq!(at(start + HOUR / 24.0 * 15.5, "saltHall")["clock"], "15:30");
        assert_eq!(at(start + HOUR, "saltHall")["clock"], "00:00");
    }

    #[test]
    fn each_time_of_day_takes_a_quarter_hour() {
        let start = 1_800_000_000_000.0_f64.div_euclid(HOUR) * HOUR;
        let mut minutes = std::collections::HashMap::<String, u32>::new();
        for i in 0..60 {
            let w = at(start + i as f64 * 60_000.0 + 1.0, "saltHall");
            *minutes.entry(text(&w["time"]).to_string()).or_default() += 1;
        }
        for t in ["morning", "day", "evening", "night"] {
            assert_eq!(minutes[t], 15, "{t}");
        }
        let w = at(start + HOUR / 24.0 * 9.0, "saltHall");
        assert_eq!(w["time"], "morning");
        assert_eq!(w["nextTime"], "day");
        assert!((w["nextTimeInMs"].as_f64().unwrap() - HOUR / 24.0).abs() < 1.0);
    }

    #[test]
    fn maps_are_spread_over_the_day() {
        let start = 1_800_000_000_000.0_f64.div_euclid(HOUR) * HOUR + HOUR / 24.0 * 12.0;
        assert_eq!(at(start, "saltHall")["time"], "day");
        assert_eq!(at(start, "blocks")["time"], "night");
        assert_eq!(at(start, "dam")["time"], "evening");
        assert_eq!(at(start, "dryBed")["time"], "morning");
    }

    #[test]
    fn weather_is_shared_steady_and_varied() {
        let start = 1_800_000_000_000.0_f64;
        let a = at(start, "blocks");
        assert_eq!(a, at(start, "blocks"), "every device sees the same weather");
        // Holds for its ten real minutes, then may change.
        let left = a["nextWeatherInMs"].as_f64().unwrap();
        assert!(left > 0.0 && left <= 600_000.0 + 1.0);
        assert_eq!(at(start + left - 1.0, "blocks")["weather"], a["weather"]);
        assert_eq!(
            at(start + left + 1.0, "blocks")["weather"],
            a["nextWeather"]
        );
        let mut seen = std::collections::HashSet::new();
        for i in 0..400 {
            seen.insert(text(&at(start + i as f64 * 600_000.0, "saltHall")["weather"]).to_string());
        }
        assert_eq!(seen.len(), 4, "all four weathers come up: {seen:?}");
    }

    /// Every game hour of three real days, from a whole real hour.
    fn hours() -> impl Iterator<Item = f64> {
        let start = 1_800_000_000_000.0_f64.div_euclid(HOUR) * HOUR;
        (0..72 * 24).map(move |h| start + h as f64 * HOUR / 24.0 + 1.0)
    }

    const AWAY: [&str; 6] = ["blocks", "railyard", "sludgePonds", "dryBed", "intake", "backwater"];

    /// The open-share table in docs/map-access.md: every game hour of 30 game days.
    /// cargo test -p game-client open_share -- --ignored --nocapture
    #[test]
    #[ignore]
    fn open_share() {
        let start = 1_800_000_000_000.0_f64.div_euclid(HOUR) * HOUR;
        for map in AWAY {
            let mut why = std::collections::BTreeMap::<String, u32>::new();
            let n = 30 * 24;
            for h in 0..n {
                let w = at(start + h as f64 * HOUR / 24.0 + 1.0, map);
                *why.entry(w["closed"].as_str().unwrap_or("open").to_string()).or_default() += 1;
            }
            let share = |k: &str| (why.get(k).copied().unwrap_or(0) as f64 / n as f64 * 100.0).round();
            println!("{map}: {}", why.keys().map(|k| format!("{k} {}%", share(k))).collect::<Vec<_>>().join(", "));
        }
    }

    #[test]
    fn a_region_home_never_shuts() {
        for ms in hours() {
            for map in ["saltHall", "dam"] {
                let w = at(ms, map);
                assert_eq!(w["open"], true, "{map} at {ms}");
                assert_eq!(w["home"], true);
                assert!(w["changeInMs"].is_null());
            }
        }
        assert_eq!(at(0.0, "dam")["home"], true);
        assert_eq!(at(0.0, "railyard")["home"], false);
        assert_eq!(at(0.0, "railyard")["mapName"], "Сортировка");
    }

    #[test]
    fn the_shelling_moves_on_every_quarter_and_hits_what_it_names() {
        let mut targets = std::collections::HashSet::new();
        for ms in hours() {
            for region in ["belyYar", "kaskad"] {
                let all = dispatch("client.world.region", json!([ms, region])).unwrap();
                let all = all.as_array().unwrap();
                assert_eq!(all.len(), 4, "every region has four maps");
                let shelled = &all[0]["shelled"];
                let hit: Vec<&Value> = all.iter().filter(|w| w["closed"] == "shelling").collect();
                assert!(hit.len() <= 1, "one map under fire at a time");
                if let Some(w) = hit.first() {
                    assert_eq!(&w["mapId"], shelled);
                }
                targets.insert(text(shelled).to_string());
            }
            let left = at(ms, "blocks")["shellingMovesInMs"].as_f64().unwrap();
            assert!(left > 0.0 && left <= HOUR / 4.0 + 1.0);
        }
        for map in AWAY {
            assert!(targets.contains(map), "the guns reach {map}: {targets:?}");
        }
    }

    #[test]
    fn the_world_shuts_maps_for_its_own_reasons() {
        let mut why = std::collections::HashSet::<(String, String)>::new();
        for ms in hours() {
            for map in AWAY {
                let w = at(ms, map);
                if let Some(r) = w["closed"].as_str() {
                    assert!(w["closedName"].is_string(), "{map} {r}");
                    why.insert((map.into(), r.into()));
                }
                // The map's own hours shut it exactly in those times of day.
                let shut_time = match map {
                    "railyard" => w["time"] != "night",
                    "sludgePonds" => w["time"] == "day",
                    "backwater" => w["time"] == "morning",
                    _ => false,
                };
                assert!(w["closed"] != "hours" || shut_time, "{map} at {}", w["time"]);
                assert!(!shut_time || w["open"] == false, "{map} shuts at {}", w["time"]);
            }
        }
        let seen = |map: &str, r: &str| why.contains(&(map.to_string(), r.to_string()));
        assert!(seen("dryBed", "release"));
        assert!(seen("dryBed", "storms") && seen("backwater", "storms"));
        assert!(seen("sludgePonds", "dryWind"));
        assert!(seen("intake", "chemical"));
        assert!(!seen("blocks", "hours") && !seen("blocks", "release"));
    }

    #[test]
    fn the_dam_releases_water_twice_a_day() {
        let start = 1_800_000_000_000.0_f64.div_euclid(HOUR) * HOUR;
        // Dam 08:00 and 20:00 are shared 02:00 and 14:00, six game hours each.
        for (h, shut) in [
            (1.5, false),
            (2.5, true),
            (7.5, true),
            (8.5, false),
            (14.5, true),
            (19.5, true),
            (20.5, false),
        ] {
            let w = at(start + h * HOUR / 24.0, "dryBed");
            let release = w["closed"] == "release";
            // A storm day shuts the dry bed first and hides the release.
            if w["event"] != "storms" {
                assert_eq!(release, shut, "shared hour {h}: {}", w["closed"]);
            }
        }
    }

    #[test]
    fn the_countdown_lands_on_the_change() {
        for ms in hours().step_by(5) {
            for map in AWAY {
                let w = at(ms, map);
                let Some(left) = w["changeInMs"].as_f64() else {
                    continue;
                };
                assert!(left > 0.0, "{map}");
                assert_eq!(at(ms + left - 2.0, map)["open"], w["open"], "{map} holds until the change");
                let after = at(ms + left + 2.0, map);
                assert_ne!(after["open"], w["open"], "{map} flips at the change");
                if w["open"] == true {
                    assert_eq!(after["closed"], w["nextClosed"]);
                }
            }
        }
    }

    #[test]
    fn the_salt_hall_never_gets_rain() {
        assert_eq!(fx("storm", "day", "saltHall")["rain"], 0.0);
        assert!(fx("storm", "day", "blocks")["rain"].as_f64().unwrap() > 0.0);
        assert_eq!(fx("clear", "night", "blocks")["active"], false);
    }

    #[test]
    fn heat_shimmers_only_in_clear_sunlight() {
        assert_eq!(fx("clear", "day", "dryBed")["heat"], 1.0);
        assert_eq!(fx("clear", "night", "dryBed")["heat"], 0.0);
        assert_eq!(fx("fog", "day", "dryBed")["heat"], 0.0);
        assert!(fx("clear", "day", "saltHall")["heat"].as_f64().unwrap() < 0.5);
        assert!(fx("dust", "day", "dam")["surge"].as_f64().unwrap() > 0.9);
        assert_eq!(fx("storm", "day", "saltHall")["wet"], 0.0);
    }
}
