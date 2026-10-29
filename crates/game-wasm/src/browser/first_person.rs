//! First-person hand placement, recoil state and grip transitions.
use super::{bridge::*, kinematics::*};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct RustViewmodel {
    three: JsValue,
    hand: JsValue,
    base: JsValue,
    right: RustGrip,
    left: RustGrip,
    palm: JsValue,
    arms: Option<[[JsValue; 3]; 2]>,
    weight: f64,
    aiming: f64,
    since: f64,
    was: bool,
    kick: f64,
    time: f64,
    at: [f64; 3],
    placed: bool,
    trigger: JsValue,
    cup: JsValue,
    pistol_right: JsValue,
    pistol_left: JsValue,
}
#[wasm_bindgen]
impl RustViewmodel {
    #[wasm_bindgen(constructor)]
    pub fn new(three: JsValue, model: &JsValue, hand: JsValue, palm: JsValue) -> Self {
        let base = clone_value(&get(&get(&hand, "mount"), "position"));
        let right = RustGrip::new(&three, model, "r");
        let left = RustGrip::new(&three, model, "l");
        let arms = ["l", "r"].map(|side| {
            ["upperarm", "lowerarm", "hand"].map(|name| find(model, &[&format!("{name}_{side}")]))
        });
        let arms = if arms.iter().flatten().all(present) {
            Some(arms)
        } else {
            None
        };
        let trigger = serde_json::json!({"thumb":[15,35,25],"index":[5,10,5],"middle":[75,85,45],"ring":[78,85,45],"pinky":[80,85,45]});
        let mut pistol_right = trigger.clone();
        pistol_right["thumb"] = serde_json::json!([-25, -10, 0]);
        Self {
            three,
            hand,
            base,
            right,
            left,
            palm,
            arms,
            weight: 0.,
            aiming: 0.,
            since: -1.,
            was: false,
            kick: 0.,
            time: 0.,
            at: [0.; 3],
            placed: false,
            trigger: json(&trigger),
            pistol_right: json(&pistol_right),
            cup: json(
                &serde_json::json!({"thumb":[10,20,10],"index":[45,55,35],"middle":[50,60,35],"ring":[52,60,35],"pinky":[55,60,35]}),
            ),
            pistol_left: json(
                &serde_json::json!({"thumb":[-25,-10,0],"index":[65,75,40],"middle":[68,78,40],"ring":[70,80,40],"pinky":[72,80,40]}),
            ),
        }
    }
    pub fn shot(&mut self) {
        self.kick = 1.;
        self.since = 0.;
    }
    pub fn update(
        &mut self,
        dt: f64,
        on: bool,
        aiming: bool,
        stance: &str,
        camera: &JsValue,
        aim: &JsValue,
    ) {
        let stance_index = ["pistol", "rifle", "shotgun", "sniper"]
            .iter()
            .position(|s| *s == stance)
            .map(|n| n as f64)
            .unwrap_or(-1.);
        let frame = math(
            "viewmodel",
            &[
                dt,
                f64::from(on),
                f64::from(aiming),
                stance_index,
                self.weight,
                self.aiming,
                self.since,
                f64::from(self.was),
                self.kick,
                self.time,
                self.at[0],
                self.at[1],
                self.at[2],
                f64::from(self.placed),
            ],
        );
        self.weight = frame[0];
        self.aiming = frame[1];
        self.since = frame[2];
        self.was = frame[3] != 0.;
        self.kick = frame[4];
        self.time = frame[5];
        self.at.copy_from_slice(&frame[6..9]);
        self.placed = frame[9] != 0.;
        let mount = get(&self.hand, "mount");
        let offset = vector(&self.three, &[0., 0., -0.025 * self.weight]);
        call(&offset, "multiply", &[get(&mount, "scale")]);
        call(&offset, "applyQuaternion", &[get(&mount, "quaternion")]);
        copy(&get(&mount, "position"), &self.base);
        call(&get(&mount, "position"), "add", &[offset]);
        if self.arms.is_none() || self.weight < 0.01 {
            self.placed = false;
            return;
        }
        call(camera, "updateMatrixWorld", &[]);
        call(&mount, "updateMatrix", &[]);
        let mut input = numbers(&get(&get(camera, "matrixWorld"), "elements"));
        input.extend(components(aim));
        input.extend(&frame[10..14]);
        input.extend(components(&call(
            &mount,
            "getWorldScale",
            &[vector(&self.three, &[])],
        )));
        input.extend(numbers(&get(&get(&mount, "matrix"), "elements")));
        input.push(stance_index);
        let palm = if present(&self.palm) {
            self.palm.clone()
        } else {
            make(&self.three, "Matrix4", &[])
        };
        input.extend(numbers(&get(&palm, "elements")));
        let matrices = math("viewmodelTransforms", &input);
        self.place(1, &matrices[..16], camera);
        if present(&self.palm) {
            self.place(0, &matrices[16..32], camera);
        }
        let (right, left) = if stance == "pistol" {
            (&self.pistol_right, &self.pistol_left)
        } else {
            (&self.trigger, &self.cup)
        };
        self.right.close(right, self.weight);
        self.left.close(left, self.weight);
    }
    pub fn off(&mut self) {
        self.weight = 0.;
        self.placed = false;
        copy(&get(&get(&self.hand, "mount"), "position"), &self.base);
    }
}
impl RustViewmodel {
    fn place(&self, side: usize, matrix: &[f64], camera: &JsValue) {
        let [upper, lower, hand] = &self.arms.as_ref().unwrap()[side];
        let mut input = matrix.to_vec();
        input.extend(components(&world_position(&self.three, hand)));
        input.extend(numbers(&get(&get(camera, "matrixWorld"), "elements")));
        input.extend([self.weight, if side == 1 { 1. } else { -1. }]);
        input.extend(components(&world_quat(&self.three, hand)));
        let pose = math("placeHand", &input);
        rig_reach(
            &self.three,
            upper,
            lower,
            hand,
            &vector(&self.three, &pose[..3]),
            &vector(&self.three, &pose[3..6]),
        );
        let mut input = components(&world_quat(&self.three, hand));
        input.extend(&pose[6..10]);
        input.push(self.weight);
        let rotation = quaternion(&self.three);
        upload(&rotation, &math("worldBlend", &input));
        rig_set_world_quat(&self.three, hand, &rotation);
    }
}
