use serde_json::{json, Value};
use std::f64::consts::{PI, TAU};

const TOP: f64 = 0.145;

fn mesh(geometry: Value, material: &str, position: [f64; 3], rotation: [f64; 3]) -> Value {
    json!({"geometry":geometry,"material":material,"position":position,"rotation":rotation,"castShadow":true,"receiveShadow":true})
}
fn disc(radius: f64, height: f64, material: &str, position: [f64; 3], segments: u32) -> Value {
    mesh(
        json!({"kind":"cylinder","args":[radius,radius,height,segments]}),
        material,
        position,
        [0., 0., 0.],
    )
}
fn contact(w: f64, d: f64, opacity: f64, position: [f64; 3], angle: f64) -> Value {
    let mut v = mesh(
        json!({"kind":"plane","args":[w,d]}),
        "contact",
        position,
        [-PI / 2., 0., angle],
    );
    v["opacity"] = json!(opacity);
    v["castShadow"] = json!(false);
    v["receiveShadow"] = json!(false);
    v
}

/// Only the platform assembly changes. The authored Persona coordinates remain untouched.
pub fn stage() -> Value {
    let mut fixed = vec![
        mesh(
            json!({"kind":"plane","args":[8.,8.]}),
            "floor",
            [0., -0.008, 0.],
            [-PI / 2., 0., 0.],
        ),
        contact(2.05, 1.98, 0.78, [0., -0.004, 0.], 0.),
        mesh(
            json!({"kind":"lathe","profile":[[0.,0.],[0.76,0.],[0.795,0.012],[0.795,0.06],[0.78,0.078],[0.,0.078]],"segments":96}),
            "steel",
            [0., 0., 0.],
            [0., 0., 0.],
        ),
        disc(0.715, 0.032, "rubber", [0., 0.086, 0.], 80),
    ];
    fixed[0]["castShadow"] = json!(false);
    let mut rotating = vec![disc(0.77, 0.028, "edge", [0., 0.12, 0.], 80)];
    let mut top = mesh(
        json!({"kind":"silhouetteDeck","radius":0.745,"height":0.016}),
        "deck",
        [0., TOP - 0.008, 0.],
        [0., 0., 0.],
    );
    top["name"] = json!("Rust_Silhouette_POM_Deck");
    rotating.push(top);
    for angle in [-0.65_f64, 0.65, PI - 0.65, PI + 0.65] {
        fixed.push(mesh(
            json!({"kind":"box","args":[0.18,0.025,0.12]}),
            "steel",
            [angle.sin() * 0.77, 0.0125, angle.cos() * 0.77],
            [0., -angle, 0.],
        ));
        fixed.push(disc(
            0.018,
            0.012,
            "edge",
            [angle.sin() * 0.81, 0.031, angle.cos() * 0.81],
            6,
        ));
        fixed.push(contact(
            0.27,
            0.2,
            0.66,
            [angle.sin() * 0.77, -0.003, angle.cos() * 0.77],
            angle,
        ));
    }
    fixed.push(mesh(json!({"kind":"tube","points":[[0.73,0.041,-0.25],[0.88,0.025,-0.35],[0.97,0.018,-0.65],[1.14,0.018,-0.95],[1.35,0.018,-1.35]],"segments":40,"radius":0.014,"radialSegments":8}), "rubber", [0.,0.,0.], [0.,0.,0.]));
    for radius in [0.535, 0.709] {
        rotating.push(mesh(
            json!({"kind":"torus","args":[radius,0.0028,5,96]}),
            "rubber",
            [0., TOP + 0.001, 0.],
            [PI / 2., 0., 0.],
        ));
    }
    for i in 0..12 {
        let angle = i as f64 / 12. * TAU;
        rotating.push(disc(
            0.008,
            0.003,
            "edge",
            [angle.cos() * 0.68, TOP + 0.001, angle.sin() * 0.68],
            6,
        ));
        if i % 3 == 0 {
            rotating.push(mesh(
                json!({"kind":"box","args":[0.18,0.0015,0.003]}),
                "rubber",
                [angle.cos() * 0.618, TOP + 0.0015, angle.sin() * 0.618],
                [0., -angle, 0.],
            ));
        }
    }
    for x in [-0.19, 0.19] {
        rotating.push(mesh(
            json!({"kind":"box","args":[0.055,0.001,0.012]}),
            "paint",
            [x, TOP + 0.002, 0.57],
            [0., 0., 0.],
        ));
    }
    let feet = [-0.11, 0.11].map(|x| contact(0.26, 0.43, 0.66, [x, TOP + 0.003, 0.045], 0.));
    json!({"top":TOP,"fixed":fixed,"rotating":rotating,"feet":feet})
}

type V3 = [f64; 3];
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, n: f64) -> V3 {
    [a[0] * n, a[1] * n, a[2] * n]
}
fn length(a: V3) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn normalize(a: V3) -> V3 {
    mul(a, 1. / length(a).max(1e-12))
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn transform(p: V3, m: &[f64]) -> V3 {
    let w = 1. / (m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15]);
    [
        (m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12]) * w,
        (m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13]) * w,
        (m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14]) * w,
    ]
}

/// Non-uniform centripetal Catmull-Rom, matching the library spline used by the existing sling.
fn point(points: &[V3], t: f64) -> V3 {
    let value = (points.len() - 1) as f64 * t;
    let mut i = value.floor() as usize;
    let mut w = value - i as f64;
    if i == points.len() - 1 {
        i -= 1;
        w = 1.;
    }
    let p1 = points[i];
    let p2 = points[i + 1];
    let p0 = if i > 0 {
        points[i - 1]
    } else {
        sub(mul(p1, 2.), p2)
    };
    let p3 = if i + 2 < points.len() {
        points[i + 2]
    } else {
        sub(mul(p2, 2.), p1)
    };
    let mut dt0 = length(sub(p0, p1)).sqrt();
    let mut dt1 = length(sub(p1, p2)).sqrt();
    let mut dt2 = length(sub(p2, p3)).sqrt();
    if dt1 < 1e-4 {
        dt1 = 1.;
    }
    if dt0 < 1e-4 {
        dt0 = dt1;
    }
    if dt2 < 1e-4 {
        dt2 = dt1;
    }
    std::array::from_fn(|n| {
        let t1 =
            ((p1[n] - p0[n]) / dt0 - (p2[n] - p0[n]) / (dt0 + dt1) + (p2[n] - p1[n]) / dt1) * dt1;
        let t2 =
            ((p2[n] - p1[n]) / dt1 - (p3[n] - p1[n]) / (dt1 + dt2) + (p3[n] - p2[n]) / dt2) * dt1;
        p1[n]
            + t1 * w
            + (-3. * p1[n] + 3. * p2[n] - 2. * t1 - t2) * w * w
            + (2. * p1[n] - 2. * p2[n] + t1 + t2) * w * w * w
    })
}

/// The carabiner loop of a sling clip, in the clip's own plane.
pub fn clip_path() -> Value {
    json!([
        [0.009, -0.016, 0.],
        [-0.007, -0.018, 0.],
        [-0.011, -0.009, 0.],
        [-0.011, 0.014, 0.],
        [0., 0.021, 0.],
        [0.009, 0.014, 0.]
    ])
}

pub fn sling(args: &Value) -> Result<Value, String> {
    let matrix = args["matrix"]
        .as_array()
        .ok_or("Missing sling mount matrix")?;
    if matrix.len() != 16 {
        return Err("A sling mount needs a 4x4 matrix".into());
    }
    let matrix: Vec<f64> = matrix
        .iter()
        .map(|n| n.as_f64().ok_or("Invalid matrix element"))
        .collect::<Result<_, _>>()?;
    let pistol = args["pistol"].as_bool().unwrap_or(false);
    let points = if pistol {
        vec![
            transform([-0.025, -0.03, 0.], &matrix),
            [0.24, 1.02, 0.18],
            [0.24, 1.08, -0.12],
        ]
    } else {
        vec![
            transform([-0.24, 0.08, 0.], &matrix),
            [-0.21, 1.50, 0.20],
            [-0.22, 1.53, -0.08],
            [0.13, 1.35, -0.205],
            [0.25, 1.12, -0.10],
            [0.25, 1.06, 0.17],
            transform([0.34, 0.065, 0.], &matrix),
        ]
    };
    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let width = if pistol { 0.018 } else { 0.031 };
    for i in 0..=56 {
        let t = i as f64 / 56.;
        let p = point(&points, t);
        let tangent = normalize(sub(
            point(&points, (t + 0.0001).min(1.)),
            point(&points, (t - 0.0001).max(0.)),
        ));
        let mut outward = normalize([p[0], 0., p[2]]);
        if length(outward) < 0.1 {
            outward = [0., 0., 1.];
        }
        let across = mul(normalize(cross(tangent, outward)), width / 2.);
        vertices.extend(add(p, across));
        vertices.extend(sub(p, across));
        uvs.extend([0., t, 1., t]);
        if i < 56 {
            let k = i * 2;
            indices.extend([k, k + 1, k + 2, k + 1, k + 3, k + 2]);
        }
    }
    let clips = if pistol {
        Vec::new()
    } else {
        vec![
            json!({"name":"Sling_Clip_Stock","position":points[0]}),
            json!({"name":"Sling_Clip_Foreend","position":points[points.len() - 1]}),
        ]
    };
    Ok(json!({"position":vertices,"uv":uvs,"index":indices,"clips":clips,"clipPath":clip_path()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_persona_floor_height_is_unchanged() {
        let s = stage();
        assert_eq!(s["top"], 0.145);
        assert_eq!(s["rotating"][1]["geometry"]["kind"], "silhouetteDeck");
        assert_eq!(s["rotating"][1]["position"][1], TOP - 0.008);
    }
    #[test]
    fn ribbon_has_continuous_uvs_finite_vertices_and_valid_triangles() {
        let s =
            sling(&json!({"matrix":[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,1.,0.,1.]})).unwrap();
        assert_eq!(s["position"].as_array().unwrap().len(), 57 * 6);
        assert_eq!(s["index"].as_array().unwrap().len(), 56 * 6);
        assert!(s["position"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v.as_f64().unwrap().is_finite()));
        assert!(s["index"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v.as_u64().unwrap() < 114));
        assert_eq!(s["clips"].as_array().unwrap().len(), 2);
    }
}
