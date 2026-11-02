//! Motion and inverse kinematics. f64 buffers avoid per-bone JSON serialization.
use glam::{DMat4, DQuat, DVec3};
use serde_json::{Value, json};
mod math;
mod shaders;
mod timeline;
pub use math::compute;
fn vec(v: &[f64], at: usize) -> DVec3 {
    DVec3::from_slice(&v[at..at + 3])
}
fn quat(v: &[f64], at: usize) -> DQuat {
    DQuat::from_xyzw(v[at], v[at + 1], v[at + 2], v[at + 3])
}
fn matrix(v: &[f64], at: usize) -> DMat4 {
    DMat4::from_cols_array(v[at..at + 16].try_into().unwrap())
}
fn normalized(v: DVec3) -> DVec3 {
    if v.length_squared() > 0.0 {
        v.normalize()
    } else {
        DVec3::ZERO
    }
}
fn arc(from: DVec3, to: DVec3) -> DQuat {
    let mut r = from.dot(to) + 1.0;
    let xyz = if r < f64::EPSILON {
        r = 0.0;
        if from.x.abs() > from.z.abs() {
            DVec3::new(-from.y, from.x, 0.0)
        } else {
            DVec3::new(0.0, -from.z, from.y)
        }
    } else {
        from.cross(to)
    };
    DQuat::from_xyzw(xyz.x, xyz.y, xyz.z, r).normalize()
}
fn ease(v: f64) -> f64 {
    v * v * (3.0 - 2.0 * v)
}
fn smooth(current: f64, target: f64, dt: f64, speed: f64) -> f64 {
    current + (target - current) * (1.0 - (-dt * speed).exp())
}
fn appendq(out: &mut Vec<f64>, q: DQuat) {
    out.extend(q.to_array());
}
fn appendv(out: &mut Vec<f64>, v: DVec3) {
    out.extend(v.to_array());
}
/// Each stance's gun in first person: where it rests and where it rises to fire (right, down,
/// forward from the eye), then its kick. The pistol is held out at arm's length, a little right
/// of centre (Art, 2026-11-01: not pressed to the chest). Not quite at full length: the left hand
/// still has to reach a foregrip ahead of the larger pistol. 4 is the pistol with a shoulder stock
/// (Art, 2026-11-02: its own hold): the grip high and close in, so the butt plate 30 cm behind
/// and below it sits in the right shoulder pocket, and the stock takes most of the kick.
fn held(i: usize) -> ([f64; 3], [f64; 3], [f64; 3]) {
    match i {
        0 => ([0.08, 0.27, 0.46], [0.025, 0.15, 0.51], [0.035, 0.02, 0.3]),
        4 => ([0.09, 0.17, 0.405], [0.06, 0.125, 0.415], [0.03, 0.012, 0.12]),
        2 => ([0.07, 0.33, 0.2], [0.07, 0.275, 0.23], [0.09, 0.025, 0.2]),
        3 => ([0.09, 0.36, 0.22], [0.08, 0.29, 0.25], [0.08, 0.02, 0.14]),
        _ => ([0.07, 0.33, 0.22], [0.07, 0.27, 0.25], [0.05, 0.012, 0.1]),
    }
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    match method.strip_prefix("motion.").unwrap_or(method) {
        "timeline" => Ok(timeline::event(&args)),
        "dustShaders" => {
            Ok(json!({"vertex":shaders::DUST_VERTEX,"fragment":shaders::DUST_FRAGMENT}))
        }
        "materialShaders" => {
            let taps = args[0].as_u64().unwrap_or(5).max(5);
            Ok(json!({"shadow":shaders::shadow(taps),"specular":shaders::SPECULAR_AA}))
        }
        _ => Err(format!("Unknown motion method: {method}")),
    }
}
