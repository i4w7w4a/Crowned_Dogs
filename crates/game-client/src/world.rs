//! The shared game clock. One game day lasts one real hour, counted from UTC,
//! so every device sees the same hour. Each map keeps its own hour offset, which
//! spreads the four times of day over the maps at any moment. Weather changes
//! every four game hours from the odds of the region's event of the day.
use crate::text;
use game_core::{constants, rng};
use serde_json::{json, Value};

/// Real milliseconds in one game day.
pub const DAY_MS: f64 = 3_600_000.0;
const DAY_MINUTES: f64 = 1440.0;
/// Weather holds for four game hours (ten real minutes).
const WEATHER_MINUTES: f64 = 240.0;

/// Game hours each map runs ahead of the shared clock. The salt hall is the
/// reference; the others sit a quarter, a half and three quarters of a day off.
fn map_offset(map: &str) -> f64 {
    match map {
        "blocks" => 12.0,
        "dam" => 6.0,
        "dryBed" => 18.0,
        _ => 0.0,
    }
}

/// Covered maps never get rain or snow inside; lightning still shows through the roof.
fn indoor(map: &str) -> bool {
    map == "saltHall"
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

fn region_of(map: &str) -> String {
    let regions = &constants()["REGIONS"];
    regions
        .as_object()
        .and_then(|all| {
            all.iter().find(|(_, r)| {
                r["maps"]
                    .as_array()
                    .is_some_and(|maps| maps.iter().any(|m| text(m) == map))
            })
        })
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| "belyYar".into())
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
    let c = constants();
    json!({
        "mapId": map,
        "regionId": region,
        "minute": minute,
        "clock": clock_text(minute),
        "time": time,
        "timeName": c["TIME_NAMES"][time],
        "nextTime": next_time,
        "nextTimeName": c["TIME_NAMES"][next_time],
        "nextTimeInMs": to_minutes(to_next),
        "weather": weather,
        "weatherName": c["WEATHER"][&weather]["name"],
        "nextWeather": next_weather,
        "nextWeatherName": c["WEATHER"][&next_weather]["name"],
        "nextWeatherInMs": to_minutes(window_end - local),
        "event": event,
        "eventName": c["REGION_EVENTS"][event]["name"],
        "indoor": indoor(map),
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
/// a dimming of the light and the colours, tuned per time of day. Clear is all zero.
pub fn fx(weather: &str, time: &str, map: &str) -> Value {
    let night = time == "night";
    let (fog_color, dust_color, rain_color) = match time {
        "morning" => (0x9ba6b1, 0xb4bbc0, 0xb7c2cc),
        "evening" => (0x978d85, 0xc9b9a6, 0xb4aaa2),
        "night" => (0x27303a, 0x3e4750, 0x55606a),
        _ => (0xa3a9ad, 0xbfc1bd, 0xc3c9ce),
    };
    let outdoor = !indoor(map);
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
    json!({
        "active": weather != "clear" && !weather.is_empty(),
        "fog": fog, "dust": dust, "rain": rain, "flash": flash, "dim": dim, "haze": haze,
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
        "client.world.fx" => fx(text(&args[0]), text(&args[1]), text(&args[2])),
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

    #[test]
    fn the_salt_hall_never_gets_rain() {
        assert_eq!(fx("storm", "day", "saltHall")["rain"], 0.0);
        assert!(fx("storm", "day", "blocks")["rain"].as_f64().unwrap() > 0.0);
        assert_eq!(fx("clear", "night", "blocks")["active"], false);
    }
}
