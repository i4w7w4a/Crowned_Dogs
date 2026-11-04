//! Scene sampling, scratch-animation ownership and inverse-kinematics application.
//! Three remains the foreign scene SDK; the numerical solver is game-motion.
use super::bridge::*;
use js_sys::Float64Array;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::prelude::*;

pub(super) fn make(three: &JsValue, name: &str, args: &[JsValue]) -> JsValue {
    construct(&get(three, name), args)
}
pub(super) fn vector(three: &JsValue, xyz: &[f64]) -> JsValue {
    make(
        three,
        "Vector3",
        &xyz.iter().map(|v| (*v).into()).collect::<Vec<_>>(),
    )
}
pub(super) fn quaternion(three: &JsValue) -> JsValue {
    make(three, "Quaternion", &[])
}
pub(super) fn components(value: &JsValue) -> Vec<f64> {
    // one host call that writes straight into our memory
    let mut out = vec![0.; 16];
    let len = super::packet::fill(value, &mut out) as usize;
    if len > out.len() {
        return Float64Array::new(value).to_vec();
    }
    out.truncate(len);
    out
}
pub(super) fn numbers(value: &JsValue) -> Vec<f64> {
    values(value).iter().map(num).collect()
}
pub(super) fn upload(value: &JsValue, data: &[f64]) {
    call(value, "fromArray", &[Float64Array::from(data).into()]);
}
pub(super) fn copy(to: &JsValue, from: &JsValue) {
    call(to, "copy", &[from.clone()]);
}
pub(super) fn clone_value(value: &JsValue) -> JsValue {
    call(value, "clone", &[])
}
pub(super) fn math(kind: &str, input: &[f64]) -> Vec<f64> {
    game_motion::compute(kind, input).expect("motion solver")
}
pub(super) fn world_position(three: &JsValue, bone: &JsValue) -> JsValue {
    call(bone, "getWorldPosition", &[vector(three, &[])])
}
pub(super) fn world_quat(three: &JsValue, bone: &JsValue) -> JsValue {
    call(bone, "getWorldQuaternion", &[quaternion(three)])
}
pub(super) fn update(root: &JsValue) {
    call(root, "updateMatrixWorld", &[true.into()]);
}
pub(super) fn find(model: &JsValue, names: &[&str]) -> JsValue {
    names
        .iter()
        .map(|name| call(model, "getObjectByName", &[(*name).into()]))
        .find(present)
        .unwrap_or(JsValue::NULL)
}
pub(super) fn descendants(root: &JsValue) -> Vec<JsValue> {
    let output = Rc::new(RefCell::new(Vec::new()));
    let sink = output.clone();
    let visitor = Closure::<dyn FnMut(JsValue)>::new(move |o| sink.borrow_mut().push(o));
    call(root, "traverse", &[visitor.as_ref().clone()]);
    output.take()
}
pub(super) fn local_position(three: &JsValue, model: &JsValue, bone: &JsValue) -> JsValue {
    call(model, "worldToLocal", &[world_position(three, bone)])
}

/// A bone's world matrix as Three last computed it: one copy, no walk up its parents and no
/// new vector. The caller makes sure it is current (an `updateWorldMatrix` above it this frame).
pub(super) fn world_matrix(bone: &JsValue) -> Vec<f64> {
    let mut out = vec![0.; 16];
    super::packet::world_of(bone, &mut out);
    out
}
/// A Three matrix's sixteen numbers in one typed-array copy.
pub(super) fn elements(matrix: &JsValue) -> Vec<f64> {
    let mut out = vec![0.; 16];
    super::packet::matrix(matrix, &mut out);
    out
}
pub(super) fn matrix_position(m: &[f64]) -> [f64; 3] {
    [m[12], m[13], m[14]]
}
/// The rotation in a world matrix, taken apart as Three's `decompose` does it.
pub(super) fn matrix_rotation(m: &[f64]) -> [f64; 4] {
    let det = m[0] * (m[5] * m[10] - m[9] * m[6]) - m[4] * (m[1] * m[10] - m[9] * m[2])
        + m[8] * (m[1] * m[6] - m[5] * m[2]);
    if det == 0. {
        return [0., 0., 0., 1.];
    }
    let length = |a: f64, b: f64, c: f64| (a * a + b * b + c * c).sqrt();
    let mut sx = length(m[0], m[1], m[2]);
    let (sy, sz) = (length(m[4], m[5], m[6]), length(m[8], m[9], m[10]));
    if det < 0. {
        sx = -sx;
    }
    let (ix, iy, iz) = (1. / sx, 1. / sy, 1. / sz);
    let (m11, m12, m13) = (m[0] * ix, m[4] * iy, m[8] * iz);
    let (m21, m22, m23) = (m[1] * ix, m[5] * iy, m[9] * iz);
    let (m31, m32, m33) = (m[2] * ix, m[6] * iy, m[10] * iz);
    let trace = m11 + m22 + m33;
    if trace > 0. {
        let s = 0.5 / (trace + 1.).sqrt();
        [(m32 - m23) * s, (m13 - m31) * s, (m21 - m12) * s, 0.25 / s]
    } else if m11 > m22 && m11 > m33 {
        let s = 2. * (1. + m11 - m22 - m33).sqrt();
        [0.25 * s, (m12 + m21) / s, (m13 + m31) / s, (m32 - m23) / s]
    } else if m22 > m33 {
        let s = 2. * (1. + m22 - m11 - m33).sqrt();
        [(m12 + m21) / s, 0.25 * s, (m23 + m32) / s, (m13 - m31) / s]
    } else {
        let s = 2. * (1. + m33 - m11 - m22).sqrt();
        [(m13 + m31) / s, (m23 + m32) / s, 0.25 * s, (m21 - m12) / s]
    }
}
/// Brings a bone and every parent above it up to date in one walk, leaving what hangs below.
pub(super) fn refresh(bone: &JsValue) {
    call(bone, "updateWorldMatrix", &[true.into(), false.into()]);
}
fn parent_rotation(bone: &JsValue) -> [f64; 4] {
    let mut m = [0.; 16];
    super::packet::parent_world_of(bone, &mut m);
    matrix_rotation(&m)
}
/// `rig_set_world_quat` for a bone whose parent's world matrix is current.
pub(super) fn set_world_rotation(bone: &JsValue, rotation: &[f64]) {
    let mut input = parent_rotation(bone).to_vec();
    input.extend(rotation);
    super::packet::set_quat(bone, &math("worldQuat", &input));
    update(bone);
}
/// `rig_turn` for a bone whose world matrix and its parent's are current.
pub(super) fn turn(bone: &JsValue, from: &[f64], to: &[f64]) {
    let mut input = from.to_vec();
    input.extend(to);
    input.extend(matrix_rotation(&world_matrix(bone)));
    input.extend(parent_rotation(bone));
    super::packet::set_quat(bone, &math("turn", &input));
    update(bone);
}
/// `rig_reach` for a chain whose world matrices are current: reads them instead of walking up.
pub(super) fn reach(a: &JsValue, b: &JsValue, c: &JsValue, target: &[f64], pole: &[f64]) {
    let (ma, mb) = (world_matrix(a), world_matrix(b));
    let mut input = Vec::with_capacity(31);
    input.extend(matrix_position(&ma));
    input.extend(matrix_position(&mb));
    input.extend(matrix_position(&world_matrix(c)));
    input.extend(target);
    input.extend(pole);
    input.extend(matrix_rotation(&ma));
    input.extend(matrix_rotation(&mb));
    input.extend(parent_rotation(a));
    input.extend(parent_rotation(b));
    let pose = math("reach", &input);
    if pose[0] == 0. {
        return;
    }
    super::packet::set_quat(a, &pose[1..5]);
    super::packet::set_quat(b, &pose[5..9]);
    update(a);
}

#[wasm_bindgen]
pub fn rig_set_world_quat(three: &JsValue, bone: &JsValue, rotation: &JsValue) {
    let mut input = components(&world_quat(three, &get(bone, "parent")));
    input.extend(components(rotation));
    super::packet::set_quat(bone, &math("worldQuat", &input));
    update(bone);
}
#[wasm_bindgen]
pub fn rig_turn(three: &JsValue, bone: &JsValue, from: &JsValue, to: &JsValue) {
    let mut input = components(from);
    input.extend(components(to));
    input.extend(components(&world_quat(three, bone)));
    input.extend(components(&world_quat(three, &get(bone, "parent"))));
    super::packet::set_quat(bone, &math("turn", &input));
    update(bone);
}
#[wasm_bindgen]
pub fn rig_reach(
    three: &JsValue,
    a: &JsValue,
    b: &JsValue,
    c: &JsValue,
    target: &JsValue,
    pole: &JsValue,
) {
    let mut input = Vec::with_capacity(31);
    for bone in [a, b, c] {
        input.extend(components(&world_position(three, bone)));
    }
    input.extend(components(target));
    input.extend(components(pole));
    for bone in [a.clone(), b.clone(), get(a, "parent"), get(b, "parent")] {
        input.extend(components(&world_quat(three, &bone)));
    }
    let pose = math("reach", &input);
    if pose[0] == 0. {
        return;
    }
    super::packet::set_quat(a, &pose[1..5]);
    super::packet::set_quat(b, &pose[5..9]);
    update(a);
}

struct Joint {
    /// The bone's own `quaternion`: Three never swaps it, so it is fetched once.
    quaternion: JsValue,
    rest: [f64; 4],
    axis: [f64; 3],
    finger: usize,
    n: usize,
}
const FINGERS: [&str; 5] = ["thumb", "index", "middle", "ring", "pinky"];
/// A hand's curl in degrees: up to three joints for each of `FINGERS`.
pub(super) type Curl = [[Option<f64>; 3]; 5];
/// Reads a `{thumb: [a, b, c], …}` curl once, so a frame's grip makes no host calls for it.
pub(super) fn curl_of(curl: &JsValue) -> Curl {
    FINGERS.map(|finger| {
        let angles = get(curl, finger);
        if !present(&angles) {
            return [None; 3];
        }
        let angles = values(&angles);
        [0, 1, 2].map(|n| angles.get(n).filter(|a| present(a)).map(num))
    })
}
#[wasm_bindgen]
pub struct RustGrip {
    joints: Vec<Joint>,
    /// The joints' quaternions in order, read and written as one packet.
    quats: js_sys::Array,
}
#[wasm_bindgen]
impl RustGrip {
    #[wasm_bindgen(constructor)]
    pub fn new(three: &JsValue, model: &JsValue, side: &str) -> Self {
        let bone = |name: &str| find(model, &[&format!("{name}_{side}")]);
        let mut result = Self {
            joints: vec![],
            quats: js_sys::Array::new(),
        };
        let hand = bone("hand");
        let mid = bone("middle_01");
        let index = bone("index_01");
        let pinky = bone("pinky_01");
        if [&hand, &mid, &index, &pinky].iter().any(|o| !present(o)) {
            return result;
        }
        update(model);
        let along = world_position(three, &mid);
        call(&along, "sub", &[world_position(three, &hand)]);
        call(&along, "normalize", &[]);
        let across = world_position(three, &index);
        call(&across, "sub", &[world_position(three, &pinky)]);
        let dot = num(&call(&across, "dot", &[along.clone()]));
        call(&across, "addScaledVector", &[along.clone(), (-dot).into()]);
        call(&across, "normalize", &[]);
        let palm = vector(three, &[]);
        call(&palm, "crossVectors", &[along, across]);
        call(
            &palm,
            "multiplyScalar",
            &[if side == "r" { -1. } else { 1. }.into()],
        );
        for (f, finger) in FINGERS.into_iter().enumerate() {
            for n in 1..=3 {
                let b = bone(&format!("{finger}_0{n}"));
                if !present(&b) {
                    continue;
                }
                let next = bone(&format!("{finger}_0{}", n + 1));
                let dir = if present(&next) {
                    let p = world_position(three, &next);
                    call(&p, "sub", &[world_position(three, &b)]);
                    p
                } else {
                    let p = world_position(three, &b);
                    call(&p, "sub", &[world_position(three, &get(&b, "parent"))]);
                    p
                };
                call(&dir, "normalize", &[]);
                let axis = vector(three, &[]);
                call(&axis, "crossVectors", &[dir, palm.clone()]);
                call(&axis, "normalize", &[]);
                let q = world_quat(three, &b);
                call(&q, "invert", &[]);
                call(&axis, "applyQuaternion", &[q]);
                let rest = components(&get(&b, "quaternion"));
                let axis = components(&axis);
                result.joints.push(Joint {
                    rest: [rest[0], rest[1], rest[2], rest[3]],
                    quaternion: get(&b, "quaternion"),
                    axis: [axis[0], axis[1], axis[2]],
                    finger: f,
                    n,
                });
            }
        }
        result.quats = result.joints.iter().map(|j| j.quaternion.clone()).collect();
        result
    }
    pub fn close(&self, curl: &JsValue, weight: f64) {
        self.close_curl(&curl_of(curl), weight);
    }
}
impl RustGrip {
    /// Closes the fingers by `curl`; only each joint's current rotation crosses to Three and back.
    pub(super) fn close_curl(&self, curl: &Curl, weight: f64) {
        if self.joints.is_empty() {
            return;
        }
        let mut now = vec![0.; self.joints.len() * 4];
        super::packet::read_quats(&self.quats, &mut now);
        let mut at = Vec::with_capacity(self.joints.len());
        let mut input = Vec::with_capacity(self.joints.len() * 13);
        for (i, j) in self.joints.iter().enumerate() {
            let Some(angle) = curl[j.finger][j.n - 1] else {
                continue;
            };
            at.push(i as u32);
            input.extend_from_slice(&now[i * 4..i * 4 + 4]);
            input.extend(j.rest);
            input.extend(j.axis);
            input.extend([angle, weight]);
        }
        if at.is_empty() {
            return;
        }
        let output = math("grip", &input);
        super::packet::write_quats_at(&self.quats, &at, &output);
    }
}

/// Runs `measure` with the model in the skin's bind pose, then puts every bone back.
/// Grips and palms must be measured there: a clip's curled fingers would skew them.
pub(super) fn at_bind<T>(three: &JsValue, model: &JsValue, measure: impl FnOnce() -> T) -> T {
    let _keep = ScratchPose::new(three, model);
    let hand = find(model, &["hand_l"]);
    let skeleton = descendants(model)
        .into_iter()
        .filter(|o| boolean(&get(o, "isSkinnedMesh")))
        .map(|o| get(&o, "skeleton"))
        .find(|s| values(&get(s, "bones")).contains(&hand));
    if let Some(skeleton) = skeleton {
        call(&skeleton, "pose", &[]);
        // `pose` also restores the export's root scale; the figure is built at 1.
        for bone in values(&get(&skeleton, "bones")) {
            if !boolean(&get(&get(&bone, "parent"), "isBone")) {
                call(&get(&bone, "scale"), "setScalar", &[1.into()]);
            }
        }
        update(model);
    }
    measure()
}
/// The support hand's cup, its fingers closed `fingers` times and its thumb `thumb` times as far.
pub(super) fn cupped(fingers: f64, thumb: f64) -> Curl {
    // thumb, index, middle, ring, pinky
    let cup = [
        [10., 20., 10.],
        [45., 55., 35.],
        [50., 60., 35.],
        [52., 60., 35.],
        [55., 60., 35.],
    ];
    let mut curl = [[None; 3]; 5];
    for (f, angles) in cup.into_iter().enumerate() {
        let k = if f == 0 { thumb } else { fingers };
        curl[f] = angles.map(|a| Some(a * k));
    }
    curl
}
/// The forearm's line in the hand bone's own frame, measured at the current pose.
pub(super) fn forearm(three: &JsValue, lower: &JsValue, hand: &JsValue) -> JsValue {
    let line = world_position(three, hand);
    call(&line, "sub", &[world_position(three, lower)]);
    call(&line, "normalize", &[]);
    let q = world_quat(three, hand);
    call(&q, "invert", &[]);
    call(&line, "applyQuaternion", &[q]);
    line
}

/// Scratch mixers own temporary animation state and always restore it on scope exit.
pub(super) struct ScratchPose {
    model: JsValue,
    pub mixer: JsValue,
    bones: Vec<(JsValue, JsValue, JsValue, JsValue)>,
}
impl ScratchPose {
    pub fn new(three: &JsValue, model: &JsValue) -> Self {
        let bones = descendants(model)
            .into_iter()
            .filter(|o| boolean(&get(o, "isBone")))
            .map(|o| {
                let p = clone_value(&get(&o, "position"));
                let q = clone_value(&get(&o, "quaternion"));
                let s = clone_value(&get(&o, "scale"));
                (o, p, q, s)
            })
            .collect();
        Self {
            model: model.clone(),
            mixer: make(three, "AnimationMixer", &[model.clone()]),
            bones,
        }
    }
}
impl Drop for ScratchPose {
    fn drop(&mut self) {
        call(&self.mixer, "stopAllAction", &[]);
        call(&self.mixer, "uncacheRoot", &[self.model.clone()]);
        for (o, p, q, s) in &self.bones {
            copy(&get(o, "position"), p);
            copy(&get(o, "quaternion"), q);
            copy(&get(o, "scale"), s);
        }
        update(&self.model);
    }
}
#[wasm_bindgen]
pub fn rig_gait_of(three: &JsValue, model: &JsValue, clip: &JsValue) -> JsValue {
    let l = find(model, &["foot_l", "mixamorigLeftFoot"]);
    let r = find(model, &["foot_r", "mixamorigRightFoot"]);
    if !present(&l) || !present(&r) {
        return JsValue::NULL;
    }
    let scratch = ScratchPose::new(three, model);
    let action = call(&scratch.mixer, "clipAction", &[clip.clone()]);
    call(&action, "play", &[]);
    let dt = num(&get(clip, "duration")) / 96.;
    let mut input = vec![dt];
    for i in 0..96 {
        call(&scratch.mixer, "setTime", &[(i as f64 * dt).into()]);
        update(model);
        input.extend(components(&local_position(three, model, &l)));
        input.extend(components(&local_position(three, model, &r)));
    }
    call(&action, "stop", &[]);
    drop(scratch);
    let result = math("gait", &input);
    if result[0] > 0.1 {
        object(&[("speed", result[0].into()), ("passing", result[1].into())])
    } else {
        JsValue::NULL
    }
}

#[derive(Clone)]
struct Footprint {
    at: JsValue,
    turn: JsValue,
    toe: JsValue,
}
struct Leg {
    thigh: JsValue,
    calf: JsValue,
    foot: JsValue,
    ball: JsValue,
}
#[wasm_bindgen]
pub struct RustFeet {
    three: JsValue,
    model: JsValue,
    legs: Vec<Leg>,
    prints: HashMap<String, Vec<Footprint>>,
    weight: f64,
    last: Option<Vec<Footprint>>,
}
#[wasm_bindgen]
impl RustFeet {
    #[wasm_bindgen(constructor)]
    pub fn new(three: JsValue, model: JsValue, clips: JsValue, stances: JsValue) -> Self {
        let mut result = Self {
            three,
            model,
            legs: vec![],
            prints: HashMap::new(),
            weight: 0.,
            last: None,
        };
        for side in ["l", "r"] {
            let bone = |name: &str| find(&result.model, &[&format!("{name}_{side}")]);
            let leg = Leg {
                thigh: bone("thigh"),
                calf: bone("calf"),
                foot: bone("foot"),
                ball: bone("ball"),
            };
            if [&leg.thigh, &leg.calf, &leg.foot, &leg.ball]
                .iter()
                .any(|o| !present(o))
            {
                result.legs.clear();
                return result;
            }
            result.legs.push(leg);
        }
        let three = &result.three;
        let model = &result.model;
        update(model);
        let rest = result
            .legs
            .iter()
            .map(|leg| {
                let q = world_quat(three, model);
                call(&q, "invert", &[]);
                call(&q, "multiply", &[world_quat(three, &leg.foot)]);
                (
                    local_position(three, model, &leg.foot),
                    local_position(three, model, &leg.ball),
                    q,
                )
            })
            .collect::<Vec<_>>();
        let scratch = ScratchPose::new(three, model);
        let clips = values(&clips);
        for name in values(&stances) {
            let name = string(&name);
            let Some(clip) = clips.iter().find(|clip| string(&get(clip, "name")) == name) else {
                continue;
            };
            let action = call(&scratch.mixer, "clipAction", &[clip.clone()]);
            call(&action, "play", &[]);
            let sums = [
                (vector(three, &[]), vector(three, &[])),
                (vector(three, &[]), vector(three, &[])),
            ];
            for i in 0..12 {
                call(
                    &scratch.mixer,
                    "setTime",
                    &[(num(&get(clip, "duration")) * i as f64 / 12.).into()],
                );
                update(model);
                for (j, leg) in result.legs.iter().enumerate() {
                    let a = local_position(three, model, &leg.foot);
                    let t = local_position(three, model, &leg.ball);
                    call(&sums[j].0, "add", &[a.clone()]);
                    call(&t, "sub", &[a]);
                    call(&t, "setY", &[0.into()]);
                    call(&t, "normalize", &[]);
                    call(&sums[j].1, "add", &[t]);
                }
            }
            call(&action, "stop", &[]);
            let mut print = vec![];
            for j in 0..2 {
                call(&sums[j].0, "divideScalar", &[12.into()]);
                let mut input = components(&sums[j].0);
                input.extend(components(&sums[j].1));
                input.extend(components(&rest[j].0));
                input.extend(components(&rest[j].1));
                input.extend(components(&rest[j].2));
                let p = math("footprint", &input);
                let q = quaternion(three);
                upload(&q, &p[3..7]);
                print.push(Footprint {
                    at: vector(three, &p[..3]),
                    turn: q,
                    toe: vector(three, &p[7..10]),
                });
            }
            result.prints.insert(name, print);
        }
        drop(scratch);
        result
    }
    pub fn update(&mut self, dt: f64, name: Option<String>) {
        let print = name.and_then(|name| self.prints.get(&name)).cloned();
        self.weight +=
            (if print.is_some() { 1. } else { 0. } - self.weight) * (1. - (-dt * 8.).exp());
        if self.legs.len() != 2 || self.weight < 0.01 {
            return;
        }
        let Some(print) = print.or_else(|| self.last.clone()) else {
            return;
        };
        self.last = Some(print.clone());
        // Each foot's walk up brings its leg, the hips and the figure up to date; the legs then
        // read their matrices instead of walking up for every number.
        for leg in &self.legs {
            refresh(&leg.foot);
        }
        let mq = matrix_rotation(&world_matrix(&self.model));
        for (leg, p) in self.legs.iter().zip(print) {
            let foot = world_matrix(&leg.foot);
            let mut input = vec![self.weight];
            input.extend(components(&call(
                &self.model,
                "localToWorld",
                &[clone_value(&p.at)],
            )));
            input.extend(matrix_position(&foot));
            input.extend(components(&p.toe));
            input.extend(matrix_position(&world_matrix(&leg.thigh)));
            input.extend(mq);
            input.extend(components(&p.turn));
            input.extend(matrix_rotation(&foot));
            let pose = math("feet", &input);
            reach(&leg.thigh, &leg.calf, &leg.foot, &pose[..3], &pose[3..6]);
            let mut input = matrix_rotation(&world_matrix(&leg.foot)).to_vec();
            input.extend(&pose[6..10]);
            input.push(self.weight);
            set_world_rotation(&leg.foot, &math("worldBlend", &input));
        }
    }
}
