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
    /// Each forearm's line in its hand's frame at bind (left, right): a wrist on it is straight.
    forearms: [JsValue; 2],
    spine: JsValue,
    clavicle: JsValue,
    weight: f64,
    aiming: f64,
    since: f64,
    was: bool,
    kick: f64,
    time: f64,
    at: [f64; 3],
    placed: bool,
    trigger: JsValue,
    pistol_right: JsValue,
}
#[wasm_bindgen]
impl RustViewmodel {
    #[wasm_bindgen(constructor)]
    pub fn new(three: JsValue, model: &JsValue, hand: JsValue, palm: JsValue) -> Self {
        let base = clone_value(&get(&get(&hand, "mount"), "position"));
        let arms = ["l", "r"].map(|side| {
            ["upperarm", "lowerarm", "hand"].map(|name| find(model, &[&format!("{name}_{side}")]))
        });
        let arms = if arms.iter().flatten().all(present) {
            Some(arms)
        } else {
            None
        };
        let (right, left, palm, forearms) = at_bind(&three, model, || {
            let host = object(&[("THREE", three.clone())]);
            let bind = super::scene_assets::grip_of(host, model.clone(), "l".into());
            let forearms = match &arms {
                Some(arms) => arms.clone().map(|[_, lower, hand]| forearm(&three, &lower, &hand)),
                None => [JsValue::NULL, JsValue::NULL],
            };
            (
                RustGrip::new(&three, model, "r"),
                RustGrip::new(&three, model, "l"),
                if present(&bind) { bind } else { palm },
                forearms,
            )
        });
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
            forearms,
            spine: find(model, &["spine_03"]),
            clavicle: find(model, &["clavicle_l"]),
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
        let view = numbers(&get(&get(camera, "matrixWorld"), "elements"));
        // The body stands turned, left shoulder forward; squared to the camera, the elbows
        // no longer fold and the wrists stay straight. The camera is already placed.
        if present(&self.spine) {
            let [left, right] = self.arms.as_ref().unwrap();
            let mut input = components(&world_position(&self.three, &left[0]));
            input.extend(components(&world_position(&self.three, &right[0])));
            input.extend(&view);
            input.push(self.weight);
            let turn = math("squareShoulders", &input);
            let (from, to) = (vector(&self.three, &turn[..3]), vector(&self.three, &turn[3..]));
            rig_turn(&self.three, &self.spine, &from, &to);
        }
        call(&mount, "updateMatrix", &[]);
        let mut input = view;
        input.extend(components(aim));
        input.extend(&frame[10..14]);
        input.extend(components(&call(
            &mount,
            "getWorldScale",
            &[vector(&self.three, &[])],
        )));
        input.extend(numbers(&get(&get(&mount, "matrix"), "elements")));
        // Arms held out at a pistol bend their elbows out to the sides, not in under the chest:
        // how much the elbow follows that pull (against a straight wrist), and how far out it swings.
        let pole = if stance == "pistol" { (1., 2.5) } else { (0.5, 1.) };
        self.place(1, &math("viewmodelTransforms", &input), camera, pole);
        if present(&self.palm) {
            // The left hand goes where the gun really is once the right arm has reached.
            call(&mount, "updateWorldMatrix", &[true.into(), false.into()]);
            let mut input = numbers(&get(&get(&mount, "matrixWorld"), "elements"));
            input.extend(numbers(&get(&self.palm, "elements")));
            let foregrip = boolean(&get(&self.hand, "foregrip"));
            // An unknown gun is held like the rifle.
            let held = if stance_index < 0. { 1. } else { stance_index };
            input.extend([held, f64::from(foregrip)]);
            let grip = math("support", &input);
            // A left arm too short for the pump or the forend brings its shoulder forward, as seen
            // from the side; squared to the camera it hung in the air beside the pump.
            if present(&self.clavicle) {
                let [upper, lower, hand] = &self.arms.as_ref().unwrap()[0];
                let mut input = grip[..16].to_vec();
                input.push(self.weight);
                for bone in [&self.clavicle, upper, lower, hand] {
                    input.extend(components(&world_position(&self.three, bone)));
                }
                input.extend(components(&world_quat(&self.three, hand)));
                input.extend([0., 0., 0., 1.]);
                let reach = math("supportArm", &input);
                let (from, to) = (vector(&self.three, &reach[..3]), vector(&self.three, &reach[3..6]));
                rig_turn(&self.three, &self.clavicle, &from, &to);
            }
            self.place(0, &grip[..16], camera, pole);
            self.left.close(&cupped(grip[16], grip[17]), self.weight);
        }
        let right = if stance == "pistol" {
            &self.pistol_right
        } else {
            &self.trigger
        };
        self.right.close(right, self.weight);
    }
    pub fn off(&mut self) {
        self.weight = 0.;
        self.placed = false;
        copy(&get(&get(&self.hand, "mount"), "position"), &self.base);
    }
}
impl RustViewmodel {
    fn place(&self, side: usize, matrix: &[f64], camera: &JsValue, (bend, spread): (f64, f64)) {
        let [upper, lower, hand] = &self.arms.as_ref().unwrap()[side];
        let mut input = matrix.to_vec();
        input.extend(components(&world_position(&self.three, hand)));
        input.extend(numbers(&get(&get(camera, "matrixWorld"), "elements")));
        input.extend([self.weight, if side == 1 { spread } else { -spread }]);
        input.extend(components(&world_quat(&self.three, hand)));
        input.extend(components(&world_position(&self.three, upper)));
        input.extend(components(&self.forearms[side]));
        let forearm = world_position(&self.three, lower);
        input.push(num(&call(&forearm, "distanceTo", &[world_position(&self.three, hand)])));
        input.push(bend);
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
