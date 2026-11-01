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
        .or_else(|| {
            frames
                .iter()
                .filter(matches)
                .find(|f| f["weather"] == config["weather"])
        })
        .or_else(|| frames.iter().find(matches))
        .or_else(|| frames.first());
    let Some(mut frame) = selected.cloned() else {
        return Value::Null;
    };
    // Weather may have no dedicated plate. Keep the actual match weather for
    // atmospheric effects instead of silently making a fog battle clear.
    for key in ["mapId", "time", "weather"] {
        if !config[key].is_null() {
            frame[key] = config[key].clone();
        }
    }
    frame
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "client.conditions.initial" => {
            let frames = args[0].as_array().ok_or("Conditions require frames")?;
            let requested = text(&args[1]);
            let frame_id = text(&args[2]);
            let saved = text(&args[3]);
            if TIMES.contains(&requested) {
                frame_for(
                    frames,
                    &json!({"mapId":"saltHall","weather":"clear","time":requested}),
                    "",
                )
            } else if let Some(frame) = named_frame(frames, frame_id) {
                frame
            } else if truth(&args[4]) {
                frames.first().cloned().unwrap_or(Value::Null)
            } else {
                let time = if TIMES.contains(&saved) { saved } else { "day" };
                frame_for(
                    frames,
                    &json!({"mapId":"saltHall","weather":"clear","time":time}),
                    "",
                )
            }
        }
        "client.conditions.frame" => frame_for(
            args[0].as_array().ok_or("Conditions require frames")?,
            &args[1],
            text(&args[2]),
        ),
        "client.conditions.choices" => json!([
            {"id":"morning","name":"Утро","icon":"i-morning"},
            {"id":"day","name":"День","icon":"i-day"},
            {"id":"evening","name":"Вечер","icon":"i-evening"},
            {"id":"night","name":"Ночь","icon":"i-night"}
        ]),
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
            {"id":"hall2","mapId":"saltHall","time":"day","weather":"fog","src":"fog.jpg"}
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
            json!([frames(),{"mapId":"saltHall","time":"day","weather":"fog"},"hall1"]),
        )
        .unwrap();
        assert_eq!(fog["id"], "hall2");
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
