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
    // one typed-array copy instead of a host call per number
    Float64Array::new(&call(value, "toArray", &[])).to_vec()
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

#[wasm_bindgen]
pub fn rig_set_world_quat(three: &JsValue, bone: &JsValue, rotation: &JsValue) {
    let mut input = components(&world_quat(three, &get(bone, "parent")));
    input.extend(components(rotation));
    upload(&get(bone, "quaternion"), &math("worldQuat", &input));
    update(bone);
}
#[wasm_bindgen]
pub fn rig_turn(three: &JsValue, bone: &JsValue, from: &JsValue, to: &JsValue) {
    let mut input = components(from);
    input.extend(components(to));
    input.extend(components(&world_quat(three, bone)));
    input.extend(components(&world_quat(three, &get(bone, "parent"))));
    upload(&get(bone, "quaternion"), &math("turn", &input));
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
    upload(&get(a, "quaternion"), &pose[1..5]);
    upload(&get(b, "quaternion"), &pose[5..9]);
    update(a);
}

struct Joint {
    bone: JsValue,
    rest: JsValue,
    axis: JsValue,
    finger: String,
    n: usize,
}
#[wasm_bindgen]
pub struct RustGrip {
    joints: Vec<Joint>,
}
#[wasm_bindgen]
impl RustGrip {
    #[wasm_bindgen(constructor)]
    pub fn new(three: &JsValue, model: &JsValue, side: &str) -> Self {
        let bone = |name: &str| find(model, &[&format!("{name}_{side}")]);
        let mut result = Self { joints: vec![] };
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
        for finger in ["thumb", "index", "middle", "ring", "pinky"] {
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
                result.joints.push(Joint {
                    rest: clone_value(&get(&b, "quaternion")),
                    bone: b,
                    axis,
                    finger: finger.into(),
                    n,
                });
            }
        }
        result
    }
    pub fn close(&self, curl: &JsValue, weight: f64) {
        let joints = self
            .joints
            .iter()
            .filter(|j| {
                present(&get(curl, &j.finger)) && present(&at(&get(curl, &j.finger), j.n - 1))
            })
            .collect::<Vec<_>>();
        let mut input = Vec::with_capacity(joints.len() * 13);
        for j in &joints {
            input.extend(components(&get(&j.bone, "quaternion")));
            input.extend(components(&j.rest));
            input.extend(components(&j.axis));
            input.extend([num(&at(&get(curl, &j.finger), j.n - 1)), weight]);
        }
        let output = math("grip", &input);
        for (i, j) in joints.iter().enumerate() {
            upload(&get(&j.bone, "quaternion"), &output[i * 4..i * 4 + 4]);
        }
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
pub(super) fn cupped(fingers: f64, thumb: f64) -> JsValue {
    let cup = [
        ("thumb", [10., 20., 10.]),
        ("index", [45., 55., 35.]),
        ("middle", [50., 60., 35.]),
        ("ring", [52., 60., 35.]),
        ("pinky", [55., 60., 35.]),
    ];
    let pairs = cup.map(|(finger, angles)| {
        let k = if finger == "thumb" { thumb } else { fingers };
        (finger, array(&angles.map(|a| (a * k).into())).into())
    });
    object(&pairs)
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
        update(&self.model);
        let mq = world_quat(&self.three, &self.model);
        for (leg, p) in self.legs.iter().zip(print) {
            let mut input = vec![self.weight];
            input.extend(components(&call(
                &self.model,
                "localToWorld",
                &[clone_value(&p.at)],
            )));
            input.extend(components(&world_position(&self.three, &leg.foot)));
            input.extend(components(&p.toe));
            input.extend(components(&world_position(&self.three, &leg.thigh)));
            input.extend(components(&mq));
            input.extend(components(&p.turn));
            input.extend(components(&world_quat(&self.three, &leg.foot)));
            let pose = math("feet", &input);
            rig_reach(
                &self.three,
                &leg.thigh,
                &leg.calf,
                &leg.foot,
                &vector(&self.three, &pose[..3]),
                &vector(&self.three, &pose[3..6]),
            );
            let mut input = components(&world_quat(&self.three, &leg.foot));
            input.extend(&pose[6..10]);
            input.push(self.weight);
            let q = quaternion(&self.three);
            upload(&q, &math("worldBlend", &input));
            rig_set_world_quat(&self.three, &leg.foot, &q);
        }
    }
}
