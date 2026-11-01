//! Guns carried in the fitting room. Each Persona pose has its own fit, made offline against the posed body
//! (scripts kept outside the repo, research/carry-fit): where the gun hangs, the straps that lie on the clothes, the sling clips and the holster.
use serde_json::{json, Value};
use std::sync::OnceLock;

const THICKNESS: f64 = 0.0024;

fn data() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("carry.json")).expect("carry.json is valid")
    })
}

fn triples(v: &Value) -> Result<Vec<[f64; 3]>, String> {
    v.as_array()
        .ok_or("Missing strap points")?
        .iter()
        .map(|p| {
            let p = p
                .as_array()
                .filter(|p| p.len() == 3)
                .ok_or("A strap point needs xyz")?;
            let n = |i: usize| p[i].as_f64().ok_or("Invalid strap coordinate");
            Ok([n(0)?, n(1)?, n(2)?])
        })
        .collect()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn length(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn normalize(a: [f64; 3]) -> [f64; 3] {
    mul(a, 1. / length(a).max(1e-9))
}

/// A flat woven strap with a little thickness along a fitted path; `normals` face away from the body.
pub fn strap(spec: &Value) -> Result<Value, String> {
    let points = triples(&spec["points"])?;
    let normals = triples(&spec["normals"])?;
    let n = points.len();
    if n < 2 || normals.len() != n {
        return Err("A strap needs matching points and normals".into());
    }
    let half = spec["width"].as_f64().unwrap_or(0.031) / 2.;
    let mut position = Vec::with_capacity(n * 24);
    let mut uv = Vec::with_capacity(n * 16);
    let mut index = Vec::with_capacity((n - 1) * 24);
    let mut along = 0.;
    for i in 0..n {
        if i > 0 {
            along += length(sub(points[i], points[i - 1]));
        }
        let tangent = normalize(sub(points[(i + 1).min(n - 1)], points[i.saturating_sub(1)]));
        let up = normalize(normals[i]);
        let across = mul(normalize(cross(tangent, up)), half);
        let lift = mul(up, THICKNESS / 2.);
        let p = points[i];
        let corners = [
            add(add(p, across), lift),
            add(sub(p, across), lift),
            sub(sub(p, across), lift),
            sub(add(p, across), lift),
        ];
        // Outer face, edge, inner face, edge: each keeps its own vertices so the shading stays crisp.
        for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            position.extend(corners[a]);
            position.extend(corners[b]);
            uv.extend([0., along, 1., along]);
        }
    }
    for i in 0..n as u32 - 1 {
        for face in 0..4 {
            let k = i * 8 + face * 2;
            let m = k + 8;
            index.extend([k, m, k + 1, k + 1, m, m + 1]);
        }
    }
    Ok(json!({"position":position,"uv":uv,"index":index}))
}

/// The fitted carry of `weapon` on the `sex` Persona, or null when that pose has none.
pub fn carry(args: &Value) -> Result<Value, String> {
    let sex = args["sex"].as_str().unwrap_or("male");
    let weapon = args["weapon"].as_str().ok_or("Missing weapon")?;
    let Some(fit) = data().get(format!("{sex}/{weapon}")) else {
        return Ok(Value::Null);
    };
    let straps = fit["straps"]
        .as_array()
        .map(|all| all.iter().map(strap).collect::<Result<Vec<_>, _>>())
        .transpose()?
        .unwrap_or_default();
    let holster = if fit["holster"] == true {
        data()["holster"].clone()
    } else {
        json!([])
    };
    Ok(json!({
        "matrix": fit["matrix"],
        "straps": straps,
        "clips": fit["clips"],
        "clipPath": crate::stage::clip_path(),
        "holster": holster,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_gun_has_a_fit_on_both_personas() {
        for sex in ["male", "female"] {
            for weapon in ["pistol", "rifle", "shotgun", "sniper"] {
                let c = carry(&json!({"sex":sex,"weapon":weapon})).unwrap();
                assert_eq!(c["matrix"].as_array().unwrap().len(), 16, "{sex}/{weapon}");
                assert!(!c["straps"].as_array().unwrap().is_empty());
                for s in c["straps"].as_array().unwrap() {
                    let v = s["position"].as_array().unwrap();
                    assert!(v.iter().all(|v| v.as_f64().unwrap().is_finite()));
                    let count = v.len() as u64 / 3;
                    assert!(s["index"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|i| i.as_u64().unwrap() < count));
                }
                let clips = c["clips"].as_array().unwrap().len();
                let holster = c["holster"].as_array().unwrap().len();
                if weapon == "pistol" {
                    assert_eq!((clips, holster), (0, 2));
                } else {
                    assert_eq!((clips, holster), (2, 0));
                }
            }
        }
        assert!(carry(&json!({"sex":"male","weapon":"knife"}))
            .unwrap()
            .is_null());
    }
    #[test]
    fn strap_faces_away_from_the_body() {
        let s =
            strap(&json!({"points":[[0,0,0],[0,1,0]],"normals":[[0,0,1],[0,0,1]],"width":0.03}))
                .unwrap();
        let p: Vec<f64> = s["position"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let i: Vec<usize> = s["index"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let at = |k: usize| [p[k * 3], p[k * 3 + 1], p[k * 3 + 2]];
        let n = cross(sub(at(i[1]), at(i[0])), sub(at(i[2]), at(i[0])));
        assert!(n[2] > 0., "outer face normal {n:?}");
    }
}
