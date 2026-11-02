//! A battle's conditions choose its plate as well as its rules. Visual settings
//! never replace a server-authored match configuration.
use crate::{text, truth};
use serde_json::{json, Value};

const TIMES: [&str; 4] = ["morning", "day", "evening", "night"];

fn named_frame(frames: &[Value], id: &str) -> Option<Value> {
    frames.iter().find(|f| text(&f["id"]) == id).cloned()
}

fn frame_for(frames: &[Value], config: &Value, preferred: &str) -> Value {
    let matches = |f: &&Value| {
        text(&f["mapId"]) == text(&config["mapId"]) && text(&f["time"]) == text(&config["time"])
    };
    let selected = frames
        .iter()
        .filter(matches)
        .find(|f| text(&f["id"]) == preferred && f["weather"] == config["weather"])
        // The plate on screen stays when it fits the hour: weather is drawn
        // live over it, so a dedicated weather plate only wins a fresh pick.
        .or_else(|| {
            frames
                .iter()
                .filter(matches)
                .find(|f| !preferred.is_empty() && text(&f["id"]) == preferred)
        })
        .or_else(|| {
            frames
                .iter()
                .filter(matches)
                .find(|f| f["weather"] == config["weather"])
        })
        .or_else(|| frames.iter().find(matches));
    let Some(mut frame) = selected.cloned() else {
        return Value::Null;
    };
    // Weather may have no dedicated plate. Keep the actual match weather for
    // atmospheric effects instead of silently making a fog battle clear.
    if !config["weather"].is_null() {
        frame["weather"] = config["weather"].clone();
    }
    frame
}

/// Local selection may fall back, but the selected plate supplies BOTH map and
/// time. Never label a daytime picture as a nonexistent night variant.
fn select_frame(frames: &[Value], config: &Value) -> Value {
    let available: Vec<Value> = frames
        .iter()
        .filter(|f| f["id"] != "cal")
        .cloned()
        .collect();
    let map = text(&config["mapId"]);
    let Some(first) = available.iter().find(|f| text(&f["mapId"]) == map) else {
        return available.first().cloned().unwrap_or(Value::Null);
    };
    let requested =
        json!({"mapId":first["mapId"],"time":config["time"],"weather":config["weather"]});
    let exact = frame_for(&available, &requested, "");
    if exact.is_null() {
        first.clone()
    } else {
        exact
    }
}

fn time_choices(frames: Option<&Vec<Value>>, map: &str) -> Value {
    let choices = json!([
        {"id":"morning","name":"Утро","icon":"i-morning"},
        {"id":"day","name":"День","icon":"i-day"},
        {"id":"evening","name":"Вечер","icon":"i-evening"},
        {"id":"night","name":"Ночь","icon":"i-night"}
    ]);
    json!(choices
        .as_array()
        .unwrap()
        .iter()
        .filter(|choice| frames.is_none_or(|frames| frames
            .iter()
            .any(|f| text(&f["mapId"]) == map && f["time"] == choice["id"] && f["id"] != "cal")))
        .cloned()
        .collect::<Vec<_>>())
}

/// The menu's test pickers: the clock first, then each drawn time of the map and
/// every weather. An empty id means "follow the map's game clock".
fn picks(frames: Option<&Vec<Value>>, map: &str) -> Value {
    let clock = json!({"id": "", "name": "По часам"});
    let mut times = vec![clock.clone()];
    times.extend(
        time_choices(frames, map)
            .as_array()
            .cloned()
            .unwrap_or_default(),
    );
    // Short names: five buttons share one row on a phone.
    let mut weathers = vec![clock];
    for (id, name) in [
        ("clear", "Ясно"),
        ("fog", "Туман"),
        ("dust", "Пыль"),
        ("storm", "Гроза"),
    ] {
        weathers.push(json!({"id": id, "name": name}));
    }
    json!({"times": times, "weathers": weathers})
}

/// Maps that have a picture, in frame order, each with the catalogue region it belongs to.
fn drawn_maps(frames: &[Value]) -> Vec<Value> {
    let constants = game_core::constants();
    let mut result = Vec::<Value>::new();
    for frame in frames {
        let map = text(&frame["mapId"]);
        if !result.iter().any(|m| m["id"] == map) && frame["id"] != "cal" {
            let name = constants["MAPS"][map]["name"].clone();
            if !name.is_null() {
                let region = constants["REGIONS"]
                    .as_object()
                    .and_then(|all| {
                        all.iter().find(|(_, r)| {
                            r["maps"]
                                .as_array()
                                .is_some_and(|maps| maps.iter().any(|m| m == map))
                        })
                    })
                    .map(|(id, _)| id.clone())
                    .unwrap_or_default();
                result.push(json!({"id":map,"name":name,"region":region}));
            }
        }
    }
    result
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "client.conditions.initial" => {
            let frames = args[0].as_array().ok_or("Conditions require frames")?;
            let requested = text(&args[1]);
            let frame_id = text(&args[2]);
            let saved = text(&args[3]);
            // The last two arguments are optional for old callers.
            let requested_map = text(&args[5]);
            let saved_map = text(&args[6]);
            let named = named_frame(frames, frame_id);
            if requested_map.is_empty() && !TIMES.contains(&requested) && named.is_some() {
                named.unwrap()
            } else if requested_map.is_empty() && !TIMES.contains(&requested) && truth(&args[4]) {
                frames.first().cloned().unwrap_or(Value::Null)
            } else {
                let map = if !requested_map.is_empty() {
                    requested_map
                } else if let Some(frame) = &named {
                    text(&frame["mapId"])
                } else if !saved_map.is_empty() {
                    saved_map
                } else {
                    "saltHall"
                };
                let time = if !requested.is_empty() {
                    requested
                } else if let Some(frame) = &named {
                    text(&frame["time"])
                } else if TIMES.contains(&saved) {
                    saved
                } else {
                    "day"
                };
                select_frame(frames, &json!({"mapId":map,"weather":"clear","time":time}))
            }
        }
        // The battle follows the game clock: the map's own hour picks the plate
        // and the weather. Address overrides stay for checks and screenshots.
        "client.conditions.live" => {
            let frames = args[0].as_array().ok_or("Conditions require frames")?;
            let o = &args[1];
            let time = text(&o["time"]);
            let named = named_frame(frames, text(&o["frame"]));
            let pinned = !TIMES.contains(&time) && text(&o["map"]).is_empty();
            if pinned && named.is_some() {
                named.unwrap()
            } else if pinned && truth(&o["cal"]) {
                frames.first().cloned().unwrap_or(Value::Null)
            } else {
                let map = [
                    text(&o["map"]),
                    named.as_ref().map_or("", |f| text(&f["mapId"])),
                    text(&o["savedMap"]),
                ]
                .into_iter()
                .find(|m| !m.is_empty() && frames.iter().any(|f| text(&f["mapId"]) == *m))
                .unwrap_or("saltHall");
                let world = crate::world::at(o["now"].as_f64().unwrap_or(0.0), map);
                let weather = text(&o["weather"]);
                let weathers = game_core::constants()["WEATHERS"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                // Weather is drawn live over the clear plate of that hour, so the
                // projected stage and its lighting stay the same in every weather.
                let mut frame = select_frame(
                    frames,
                    &json!({
                        "mapId": map,
                        "time": if TIMES.contains(&time) { json!(time) } else { world["time"].clone() },
                        "weather": "clear",
                    }),
                );
                if frame.is_object() {
                    frame["weather"] = if weathers.iter().any(|w| text(w) == weather) {
                        json!(weather)
                    } else {
                        world["weather"].clone()
                    };
                    frame["clock"] = world;
                }
                frame
            }
        }
        "client.conditions.select" => select_frame(
            args[0].as_array().ok_or("Conditions require frames")?,
            &args[1],
        ),
        "client.conditions.maps" => json!(drawn_maps(
            args[0].as_array().ok_or("Conditions require frames")?
        )),
        "client.conditions.regions" => {
            // Regions in catalogue order, only those with at least one drawn map.
            let maps = drawn_maps(args[0].as_array().ok_or("Conditions require frames")?);
            let regions = &game_core::constants()["REGIONS"];
            json!(regions
                .as_object()
                .map(|all| all
                    .iter()
                    .filter(|(id, _)| maps.iter().any(|m| m["region"] == id.as_str()))
                    .map(|(id, r)| json!({"id":id,"name":r["name"]}))
                    .collect::<Vec<_>>())
                .unwrap_or_default())
        }
        "client.conditions.choices" => time_choices(args[0].as_array(), text(&args[1])),
        "client.conditions.picks" => picks(args[0].as_array(), text(&args[1])),
        "client.conditions.frame" => frame_for(
            args[0].as_array().ok_or("Conditions require frames")?,
            &args[1],
            text(&args[2]),
        ),
        "client.conditions.description" => {
            let m = &args[0];
            json!(match text(&m["config"]["time"]) {
                "day" => "Марево: −10% к точности дальше 8 шагов. Перегрев ослабляет щит; термозащита сохраняет его силу.".to_string(),
                "evening" => if truth(&m["fighters"][0]["facesSun"]) {
                    "Солнце перед вами: −10% к точности. Сторона солнца определяется в начале боя.".to_string()
                } else {
                    "Солнце перед противником: −10% к его точности. Сторона солнца определяется в начале боя.".to_string()
                },
                "night" => format!("{} Выстрел без глушителя выдаёт стрелка на один ход: по нему +10% к шансу попадания.",
                    if truth(&m["fighters"][0]["gear"]["nvg"]) { "ПНВ снимает ночной штраф к точности." }
                    else { "Без ПНВ: −25% к точности дальше 4 шагов." }),
                _ => "Ровный утренний свет. Штрафов времени суток нет; погода действует отдельно.".to_string(),
            })
        }
        _ => return Err(format!("Unknown conditions operation: {method}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frames() -> Value {
        json!([
            {"id":"hall1","mapId":"saltHall","time":"day","weather":"clear","src":"day.jpg"},
            {"id":"hall-morning","mapId":"saltHall","time":"morning","weather":"clear","src":"morning.jpg"},
            {"id":"hall-evening","mapId":"saltHall","time":"evening","weather":"clear","src":"evening.jpg"},
            {"id":"hall-night","mapId":"saltHall","time":"night","weather":"clear","src":"night.jpg"},
            {"id":"hall2","mapId":"saltHall","time":"day","weather":"fog","src":"fog.jpg"},
            {"id":"blocks-day","mapId":"blocks","time":"day","weather":"clear","src":"blocks.jpg"},
            {"id":"dam-night","mapId":"dam","time":"night","weather":"clear","src":"dam.jpg"}
        ])
    }
    #[test]
    fn url_saved_and_legacy_frame_choose_the_same_rules_as_the_plate() {
        for time in TIMES {
            let chosen = dispatch(
                "client.conditions.initial",
                json!([frames(), time, "", "morning", false]),
            )
            .unwrap();
            assert_eq!(chosen["time"], time);
            assert_eq!(chosen["weather"], "clear");
        }
        assert_eq!(
            dispatch(
                "client.conditions.initial",
                json!([frames(), "garbage", "hall2", "night", false])
            )
            .unwrap()["weather"],
            "fog"
        );
        assert_eq!(
            dispatch(
                "client.conditions.initial",
                json!([frames(), "", "invalid", "night", false])
            )
            .unwrap()["time"],
            "night"
        );
        assert_eq!(
            dispatch(
                "client.conditions.initial",
                json!([frames(), "", "", "garbage", false])
            )
            .unwrap()["id"],
            "hall1"
        );
    }
    #[test]
    fn canonical_match_beats_a_stale_visual_selection_without_dropping_weather() {
        let config = json!({"mapId":"saltHall","time":"night","weather":"dust"});
        let chosen = dispatch(
            "client.conditions.frame",
            json!([frames(), config, "hall1"]),
        )
        .unwrap();
        assert_eq!(chosen["src"], "night.jpg");
        assert_eq!(chosen["time"], "night");
        assert_eq!(chosen["weather"], "dust");
        let fog = dispatch(
            "client.conditions.frame",
            json!([frames(),{"mapId":"saltHall","time":"day","weather":"fog"},""]),
        )
        .unwrap();
        assert_eq!(fog["id"], "hall2");
        let kept = dispatch(
            "client.conditions.frame",
            json!([frames(),{"mapId":"saltHall","time":"day","weather":"fog"},"hall1"]),
        )
        .unwrap();
        assert_eq!(
            kept["id"], "hall1",
            "live fog is drawn over the plate on screen"
        );
        assert_eq!(kept["weather"], "fog");
    }
    #[test]
    fn only_available_map_and_time_pairs_can_be_selected() {
        let maps = dispatch("client.conditions.maps", json!([frames()])).unwrap();
        assert_eq!(
            maps.as_array()
                .unwrap()
                .iter()
                .map(|m| text(&m["id"]))
                .collect::<Vec<_>>(),
            vec!["saltHall", "blocks", "dam"]
        );
        assert_eq!(maps[1]["name"], "Пятиэтажки");
        assert_eq!(maps[1]["region"], "belyYar");
        assert_eq!(maps[2]["region"], "kaskad");
        let regions = dispatch("client.conditions.regions", json!([frames()])).unwrap();
        assert_eq!(
            regions,
            json!([{"id":"belyYar","name":"Белый Яр"},{"id":"kaskad","name":"Каскад"}])
        );
        let picks = dispatch("client.conditions.picks", json!([frames(), "blocks"])).unwrap();
        assert_eq!(picks["times"][0], json!({"id": "", "name": "По часам"}));
        assert_eq!(picks["times"][1]["id"], "day");
        let weathers: Vec<&str> = picks["weathers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| text(&w["name"]))
            .collect();
        assert_eq!(weathers, ["По часам", "Ясно", "Туман", "Пыль", "Гроза"]);
        let choices = dispatch("client.conditions.choices", json!([frames(), "blocks"])).unwrap();
        assert_eq!(choices.as_array().unwrap().len(), 1);
        assert_eq!(choices[0]["id"], "day");
        assert_eq!(
            dispatch("client.conditions.choices", json!([]))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            4,
            "old callers still receive all four times"
        );
        for config in [
            json!({"mapId":"blocks","time":"night","weather":"clear"}),
            json!({"mapId":"unknown","time":"night","weather":"clear"}),
        ] {
            assert!(
                dispatch("client.conditions.frame", json!([frames(), config, ""]))
                    .unwrap()
                    .is_null(),
                "a canonical match must never relabel an unsupported plate"
            );
            let selected = dispatch("client.conditions.select", json!([frames(), config])).unwrap();
            assert_eq!(selected["time"], "day");
            assert!(selected["id"] == "blocks-day" || selected["id"] == "hall1");
        }
    }
    #[test]
    fn map_links_and_saved_maps_fall_back_atomically_to_available_daytime() {
        for args in [
            json!([frames(), "night", "", "night", false, "blocks", "saltHall"]),
            json!([frames(), "", "", "night", false, "", "blocks"]),
            json!([frames(), "", "blocks-day", "night", false]),
        ] {
            let selected = dispatch("client.conditions.initial", args).unwrap();
            assert_eq!(selected["mapId"], "blocks");
            assert_eq!(selected["time"], "day");
            assert_eq!(selected["src"], "blocks.jpg");
        }
    }
    #[test]
    fn the_battle_follows_the_game_clock_unless_the_address_pins_it() {
        let hour = crate::world::DAY_MS;
        let noon = 1_800_000_000_000.0_f64.div_euclid(hour) * hour + hour / 2.0;
        let live = |o: Value| dispatch("client.conditions.live", json!([frames(), o])).unwrap();
        let hall = live(json!({"now": noon}));
        assert_eq!(hall["mapId"], "saltHall");
        assert_eq!(hall["time"], "day");
        assert_eq!(hall["weather"], hall["clock"]["weather"]);
        assert_ne!(hall["id"], "hall2", "weather is drawn over the clear plate");
        let night = live(json!({"now": noon + hour / 2.0}));
        assert_eq!(night["id"], "hall-night");
        let pinned = live(json!({"now": noon, "time": "evening", "weather": "storm"}));
        assert_eq!(pinned["time"], "evening");
        assert_eq!(pinned["weather"], "storm");
        // Blocks only has its day plate here: the rules follow the plate.
        let blocks = live(json!({"now": noon, "savedMap": "blocks"}));
        assert_eq!(blocks["mapId"], "blocks");
        assert_eq!(blocks["time"], "day");
        assert_eq!(live(json!({"now": noon, "frame": "hall2"}))["id"], "hall2");
    }
    #[test]
    fn explanations_follow_the_actual_side_and_equipment() {
        let mut m = json!({"config":{"time":"evening"},"fighters":[{"facesSun":true,"gear":{"nvg":true}},{}]});
        assert!(
            text(&dispatch("client.conditions.description", json!([m])).unwrap())
                .starts_with("Солнце перед вами")
        );
        m["fighters"][0]["facesSun"] = json!(false);
        assert!(
            text(&dispatch("client.conditions.description", json!([m])).unwrap())
                .starts_with("Солнце перед противником")
        );
        m["config"]["time"] = json!("night");
        assert!(
            text(&dispatch("client.conditions.description", json!([m])).unwrap())
                .starts_with("ПНВ снимает")
        );
    }
}
