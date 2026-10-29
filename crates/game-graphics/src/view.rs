use serde_json::{json, Value};
use std::f64::consts::TAU;

fn number(args: &Value, key: &str, default: f64) -> f64 {
    args[key]
        .as_f64()
        .filter(|n| n.is_finite())
        .unwrap_or(default)
}
fn outline() -> Vec<[f64; 3]> {
    let mut points = vec![[-0.2, 1.95, 0.2], [0.2, 1.95, 0.2]];
    for i in 0..16 {
        let angle = i as f64 / 16. * TAU;
        points.push([angle.cos() * 0.81, 0., angle.sin() * 0.81]);
        points.push([angle.cos() * 0.63, 1.1, angle.sin() * 0.63]);
    }
    points
}
fn project(point: [f64; 3], distance: f64, tangent: f64, aspect: f64) -> [f64; 2] {
    let norm = distance.hypot(0.48);
    let up = [0., distance / norm, -0.48 / norm];
    let back = [0., 0.48 / norm, distance / norm];
    let delta = [point[0], point[1] - 1.36, point[2] - distance];
    let depth = -(delta[1] * back[1] + delta[2] * back[2]);
    [
        delta[0] / (depth * tangent * aspect),
        (delta[1] * up[1] + delta[2] * up[2]) / (depth * tangent),
    ]
}
pub fn fitting_camera(args: &Value) -> Result<Value, String> {
    let width = (number(args, "width", 1.) + 0.5).floor().max(1.);
    let height = (number(args, "height", 1.) + 0.5).floor().max(1.);
    let fov = number(args, "fov", 33.).clamp(1., 175.);
    let aspect = width / height;
    let tangent = (fov.to_radians() / 2.).tan();
    let mut distance = (1.12 / tangent).max(0.86 / (tangent * aspect));
    let limit_y = 1. - 24. / height;
    let outline = outline();
    for _ in 0..60 {
        if outline.iter().all(|p| {
            let [x, y] = project(*p, distance, tangent, aspect);
            x.abs() < 0.94 && y.abs() < limit_y
        }) {
            break;
        }
        distance *= 1.015;
    }
    let ratio = (number(args, "dpr", 1.) * number(args, "ssaa", 1.))
        .min(number(args, "maxDpr", 2.))
        .min((number(args, "maxPixels", 2_000_000.) / (width * height)).sqrt())
        .max(0.5);
    Ok(json!({"width":width,"height":height,"aspect":aspect,"distance":distance,"ratio":ratio}))
}
pub fn fitting_aim(args: &Value) -> Result<Value, String> {
    let distance = number(args, "distance", 4.);
    let norm = distance.hypot(0.48);
    let k = number(args, "height", 1.).max(1.)
        / 2.
        / (number(args, "fov", 33.).to_radians() / 2.).tan()
        / norm;
    let dx = -number(args, "x", 0.) / k;
    let dy = number(args, "y", 0.) / k;
    let shift = [dx, dy * distance / norm, -dy * 0.48 / norm];
    Ok(
        json!({"position":[shift[0],1.36 + shift[1],distance + shift[2]],"target":[shift[0],0.88 + shift[1],shift[2]]}),
    )
}
pub fn backdrop(args: &Value) -> Result<Value, String> {
    let width = number(args, "width", 1.).max(1.);
    let height = number(args, "height", 1.).max(1.);
    let ratio = number(args, "ratio", 1.).max(0.001);
    let fit = (width / ratio).max(height) * number(args, "zoom", 1.).max(0.001);
    let iw = fit * ratio;
    Ok(
        json!({"cover":[width / iw,height / fit],"shift":[-number(args,"x",0.) / iw,number(args,"y",0.) / fit],"dpr":number(args,"dpr",1.).min(2.)}),
    )
}
pub fn coverage(args: &Value) -> Result<Value, String> {
    let loadout = &args["loadout"];
    let real = args["hasRifleModel"] == true && args["weapon"] == "rifle";
    let mut missing: Vec<Value> = ["head", "body", "suit", "boots", "knife", "grenade"]
        .iter()
        .filter_map(|key| loadout.get(key).cloned())
        .collect();
    if !real {
        if let Some(id) = loadout["weapon1"].get("item") {
            missing.push(id.clone());
        }
    }
    if let Some(id) = loadout["weapon2"].get("item") {
        missing.push(id.clone());
    }
    if let Some(mods) = loadout["weapon1"]["mods"].as_object() {
        missing.extend(
            mods.values()
                .filter(|id| {
                    !real || !["optic-sokol", "muzzle-tishina"].contains(&id.as_str().unwrap_or(""))
                })
                .cloned(),
        );
    }
    if let Some(skin) = loadout["weapon1"]
        .get("skin")
        .filter(|id| id.as_str().is_some_and(|s| !s.is_empty()))
    {
        missing.push(skin.clone());
    }
    let mut unique = Vec::new();
    for id in missing {
        if !unique.contains(&id) {
            unique.push(id);
        }
    }
    Ok(
        json!({"exact":false,"description":"3D: комплект A и Варяг · остальные предметы пока показаны условно","missingItemIds":unique}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_and_helmet_fit_phone_and_desktop_frusta() {
        for (w, h) in [(320., 480.), (390., 844.), (1280., 720.), (1920., 1080.)] {
            let result = fitting_camera(&json!({"width":w,"height":h})).unwrap();
            let d = result["distance"].as_f64().unwrap();
            assert!(outline().iter().all(|p| {
                let [x, y] = project(*p, d, (33_f64.to_radians() / 2.).tan(), w / h);
                x.abs() < 0.94 && y.abs() < 1. - 24. / h
            }));
        }
    }
    #[test]
    fn camera_drift_translates_target_and_eye_equally() {
        let result = fitting_aim(&json!({"distance":4.,"height":700.,"x":13.,"y":-8.})).unwrap();
        let p = &result["position"];
        let t = &result["target"];
        assert!((p[0].as_f64().unwrap() - t[0].as_f64().unwrap()).abs() < 1e-10);
        assert!((p[1].as_f64().unwrap() - t[1].as_f64().unwrap() - 0.48).abs() < 1e-10);
        assert!((p[2].as_f64().unwrap() - t[2].as_f64().unwrap() - 4.).abs() < 1e-10);
    }
}
