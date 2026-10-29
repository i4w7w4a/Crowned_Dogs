//! Ragdoll-style authored collapse: scene constraints and floor contact stay in Rust.
use super::{bridge::*, kinematics::*};
use wasm_bindgen::prelude::*;

struct Leg {
    thigh: JsValue,
    calf: JsValue,
    foot: JsValue,
    at: JsValue,
    turn: JsValue,
}
struct Arm {
    upper: JsValue,
    lower: JsValue,
    hand: JsValue,
}
struct Solid {
    bone: JsValue,
    radius: f64,
}
struct Origin {
    at: JsValue,
    turn: JsValue,
    forward: JsValue,
    side: JsValue,
    direction: f64,
}
#[wasm_bindgen]
pub struct RustCollapse {
    three: JsValue,
    pelvis: JsValue,
    spine: Vec<JsValue>,
    head: JsValue,
    legs: Vec<Leg>,
    arms: Vec<Arm>,
    solids: Vec<Solid>,
    gun: JsValue,
    origin: Origin,
}
fn spin(three: &JsValue, bone: &JsValue, axis: &JsValue, angle: f64) {
    let mut input = components(axis);
    input.push(angle);
    input.extend(components(&world_quat(three, bone)));
    input.extend(components(&world_quat(three, &get(bone, "parent"))));
    upload(&get(bone, "quaternion"), &math("spin", &input));
    update(bone);
}
#[wasm_bindgen]
impl RustCollapse {
    #[wasm_bindgen(constructor)]
    pub fn new(three: JsValue, model: &JsValue) -> Self {
        let pelvis = find(model, &["pelvis", "mixamorigHips"]);
        let spine = [
            ["spine_01", "mixamorigSpine"],
            ["spine_02", "mixamorigSpine1"],
            ["spine_03", "mixamorigSpine2"],
        ]
        .iter()
        .map(|names| find(model, names))
        .filter(present)
        .collect::<Vec<_>>();
        let head = find(model, &["head", "mixamorigHead"]);
        let mut legs = vec![];
        let mut arms = vec![];
        for (side, mixamo) in [("l", "Left"), ("r", "Right")] {
            let bone = |name: &str, other: &str| {
                find(
                    model,
                    &[
                        &format!("{name}_{side}"),
                        &format!("mixamorig{mixamo}{other}"),
                    ],
                )
            };
            let thigh = bone("thigh", "UpLeg");
            let calf = bone("calf", "Leg");
            let foot = bone("foot", "Foot");
            if [&thigh, &calf, &foot].iter().all(|o| present(o)) {
                legs.push(Leg {
                    thigh,
                    calf,
                    foot,
                    at: vector(&three, &[]),
                    turn: quaternion(&three),
                });
            }
            let upper = bone("upperarm", "Arm");
            let lower = bone("lowerarm", "ForeArm");
            let hand = bone("hand", "Hand");
            if [&upper, &lower, &hand].iter().all(|o| present(o)) {
                arms.push(Arm { upper, lower, hand });
            }
        }
        let mut solids = vec![];
        if let Some(root) = spine.first() {
            for bone in descendants(root)
                .into_iter()
                .filter(|o| boolean(&get(o, "isBone")))
            {
                let name = string(&get(&bone, "name")).to_lowercase();
                let large = ["spine", "clavicle", "neck", "head", "upperarm"]
                    .iter()
                    .any(|part| name.contains(part))
                    || name.ends_with("arm");
                if large {
                    solids.push(Solid {
                        bone,
                        radius: if name.contains("head") { 0.1 } else { 0.12 },
                    });
                } else if name.contains("lowerarm")
                    || name.contains("forearm")
                    || name.ends_with("hand")
                {
                    solids.push(Solid { bone, radius: 0.05 });
                }
            }
        }
        let origin = Origin {
            at: vector(&three, &[]),
            turn: quaternion(&three),
            forward: vector(&three, &[]),
            side: vector(&three, &[]),
            direction: 1.,
        };
        Self {
            three,
            pelvis,
            spine,
            head,
            legs,
            arms,
            solids,
            gun: JsValue::NULL,
            origin,
        }
    }
    #[wasm_bindgen(getter)]
    pub fn gun(&self) -> JsValue {
        self.gun.clone()
    }
    #[wasm_bindgen(setter)]
    pub fn set_gun(&mut self, gun: JsValue) {
        self.gun = gun;
    }
    pub fn begin(&mut self, group: &JsValue, direction: f64) {
        if !present(&self.pelvis) {
            return;
        }
        update(group);
        let group_turn = world_quat(&self.three, group);
        copy(&self.origin.at, &world_position(&self.three, &self.pelvis));
        copy(&self.origin.turn, &world_quat(&self.three, &self.pelvis));
        call(&self.origin.forward, "set", &[0.into(), 0.into(), 1.into()]);
        call(
            &self.origin.forward,
            "applyQuaternion",
            &[group_turn.clone()],
        );
        call(&self.origin.side, "set", &[1.into(), 0.into(), 0.into()]);
        call(&self.origin.side, "applyQuaternion", &[group_turn]);
        self.origin.direction = direction;
        for leg in &self.legs {
            copy(&leg.at, &world_position(&self.three, &leg.foot));
            copy(&leg.turn, &world_quat(&self.three, &leg.foot));
        }
    }
    pub fn apply(&self, time: f64) {
        if !present(&self.pelvis) || !present(&get(&self.pelvis, "parent")) {
            return;
        }
        let f = &self.origin;
        let mut input = vec![time];
        input.extend(components(&f.at));
        input.extend(components(&f.turn));
        input.extend(components(&f.forward));
        input.extend(components(&f.side));
        input.push(f.direction);
        let frame = math("collapse", &input);
        let target = vector(&self.three, &frame[..3]);
        let turn = quaternion(&self.three);
        upload(&turn, &frame[3..7]);
        let over = frame[8];
        self.place(&target, &turn);
        for spine in &self.spine {
            spin(&self.three, spine, &f.side, frame[9]);
        }
        if present(&self.head) {
            spin(&self.three, &self.head, &f.side, frame[10]);
            spin(&self.three, &self.head, &f.forward, frame[11]);
        }
        if self.arms.len() == 2 && over > 0. {
            let arm = &self.arms[usize::from(f.direction <= 0.)];
            let shoulder = world_position(&self.three, &arm.upper);
            let now = world_position(&self.three, &arm.lower);
            call(&now, "sub", &[shoulder]);
            call(&now, "normalize", &[]);
            let out = clone_value(&f.forward);
            call(&out, "multiplyScalar", &[0.85.into()]);
            call(
                &out,
                "addScaledVector",
                &[f.side.clone(), (0.45 * f.direction).into()],
            );
            call(
                &out,
                "addScaledVector",
                &[vector(&self.three, &[0., -1., 0.]), 0.1.into()],
            );
            call(&out, "normalize", &[]);
            // The first interpolation also changes the direction used for the forearm, as in the authored pose.
            call(&out, "lerp", &[now.clone(), (1. - over).into()]);
            call(&out, "normalize", &[]);
            rig_turn(&self.three, &arm.upper, &now, &out);
            let elbow = world_position(&self.three, &arm.lower);
            let fore = world_position(&self.three, &arm.hand);
            call(&fore, "sub", &[elbow]);
            call(&fore, "normalize", &[]);
            call(
                &out,
                "addScaledVector",
                &[vector(&self.three, &[0., -1., 0.]), 0.15.into()],
            );
            call(
                &out,
                "addScaledVector",
                &[f.side.clone(), (-0.3 * f.direction).into()],
            );
            call(&out, "normalize", &[]);
            call(&out, "lerp", &[fore.clone(), (1. - over).into()]);
            call(&out, "normalize", &[]);
            rig_turn(&self.three, &arm.lower, &fore, &out);
        }
        update(&self.pelvis);
        let hand = get(&self.gun, "parent");
        if present(&self.gun) && present(&hand) && over > 0. {
            let along = clone_value(&f.forward);
            call(&along, "multiplyScalar", &[0.5.into()]);
            call(
                &along,
                "addScaledVector",
                &[f.side.clone(), (0.85 * f.direction).into()],
            );
            call(&along, "normalize", &[]);
            let up = vector(&self.three, &[0., 1., 0.]);
            let side = vector(&self.three, &[]);
            call(&side, "crossVectors", &[up.clone(), along.clone()]);
            let matrix = make(&self.three, "Matrix4", &[]);
            call(&matrix, "makeBasis", &[along, side, up]);
            let flat = quaternion(&self.three);
            call(&flat, "setFromRotationMatrix", &[matrix]);
            let now = world_quat(&self.three, &self.gun);
            call(&now, "slerp", &[flat, over.into()]);
            let inverse = clone_value(&get(&self.gun, "quaternion"));
            call(&inverse, "invert", &[]);
            call(&now, "multiply", &[inverse]);
            rig_set_world_quat(&self.three, &hand, &now);
        }
        let sink = self
            .solids
            .iter()
            .map(|solid| solid.radius - num(&get(&world_position(&self.three, &solid.bone), "y")))
            .fold(0., f64::max);
        if sink > 0. {
            set(&target, "y", &(num(&get(&target, "y")) + sink).into());
            self.place(&target, &turn);
        }
        self.plant(over);
    }
}
impl RustCollapse {
    fn place(&self, at: &JsValue, turn: &JsValue) {
        copy(
            &get(&self.pelvis, "position"),
            &call(
                &get(&self.pelvis, "parent"),
                "worldToLocal",
                &[clone_value(at)],
            ),
        );
        rig_set_world_quat(&self.three, &self.pelvis, turn);
    }
    fn plant(&self, over: f64) {
        let f = &self.origin;
        for leg in &self.legs {
            let pole = world_position(&self.three, &leg.thigh);
            call(&pole, "lerp", &[leg.at.clone(), 0.5.into()]);
            call(&pole, "addScaledVector", &[f.forward.clone(), 0.7.into()]);
            rig_reach(
                &self.three,
                &leg.thigh,
                &leg.calf,
                &leg.foot,
                &clone_value(&leg.at),
                &pole,
            );
            let q = quaternion(&self.three);
            call(
                &q,
                "setFromAxisAngle",
                &[f.forward.clone(), (-0.9 * over * f.direction).into()],
            );
            call(&q, "multiply", &[leg.turn.clone()]);
            rig_set_world_quat(&self.three, &leg.foot, &q);
        }
    }
}
