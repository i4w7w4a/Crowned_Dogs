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
    trigger: Curl,
    pistol_right: Curl,
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
        // thumb, index, middle, ring, pinky
        let trigger = [[15., 35., 25.], [5., 10., 5.], [75., 85., 45.], [78., 85., 45.], [80., 85., 45.]]
            .map(|finger| finger.map(Some));
        let mut pistol_right = trigger;
        pistol_right[0] = [Some(-25.), Some(-10.), Some(0.)];
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
            trigger,
            pistol_right,
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
        let stocked = stance == "pistol" && on && self.stocked();
        let stance_index = if stocked {
            4.
        } else {
            ["pistol", "rifle", "shotgun", "sniper"]
                .iter()
                .position(|s| *s == stance)
                .map(|n| n as f64)
                .unwrap_or(-1.)
        };
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
        let view = world_matrix(camera);
        // The body stands turned, left shoulder forward; squared to the camera, the elbows
        // no longer fold and the wrists stay straight. The camera is already placed.
        if present(&self.spine) {
            let [left, right] = self.arms.as_ref().unwrap();
            // Each upper arm's walk up brings the spine above it up to date too.
            refresh(&left[0]);
            refresh(&right[0]);
            let mut input = matrix_position(&world_matrix(&left[0])).to_vec();
            input.extend(matrix_position(&world_matrix(&right[0])));
            input.extend(&view);
            input.push(self.weight);
            let square = math("squareShoulders", &input);
            turn(&self.spine, &square[..3], &square[3..]);
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
        input.extend(elements(&get(&mount, "matrix")));
        // Arms held out at a pistol bend their elbows out to the sides, not in under the chest:
        // how much the elbow follows that pull (against a straight wrist), and how far out it swings.
        let pole = if stance == "pistol" { (1., 2.5) } else { (0.5, 1.) };
        self.place(1, &math("viewmodelTransforms", &input), camera, pole);
        if present(&self.palm) {
            // The left hand goes where the gun really is once the right arm has reached.
            call(&mount, "updateWorldMatrix", &[true.into(), false.into()]);
            let mut input = world_matrix(&mount);
            input.extend(elements(&self.palm));
            let foregrip = boolean(&get(&self.hand, "foregrip"));
            // An unknown gun is held like the rifle.
            let held = if stance_index < 0. { 1. } else { stance_index };
            input.extend([held, f64::from(foregrip)]);
            let grip = math("support", &input);
            // A left arm too short for the pump or the forend brings its shoulder forward, as seen
            // from the side; squared to the camera it hung in the air beside the pump.
            if present(&self.clavicle) {
                let [upper, lower, hand] = &self.arms.as_ref().unwrap()[0];
                refresh(hand);
                let mut input = grip[..16].to_vec();
                input.push(self.weight);
                for bone in [&self.clavicle, upper, lower, hand] {
                    input.extend(matrix_position(&world_matrix(bone)));
                }
                input.extend(matrix_rotation(&world_matrix(hand)));
                input.extend([0., 0., 0., 1.]);
                let shoulder = math("supportArm", &input);
                turn(&self.clavicle, &shoulder[..3], &shoulder[3..6]);
            }
            self.place(0, &grip[..16], camera, pole);
            self.left.close_curl(&cupped(grip[16], grip[17]), self.weight);
        }
        let right = if stance == "pistol" {
            &self.pistol_right
        } else {
            &self.trigger
        };
        self.right.close_curl(right, self.weight);
    }
    pub fn off(&mut self) {
        self.weight = 0.;
        self.placed = false;
        copy(&get(&get(&self.hand, "mount"), "position"), &self.base);
    }
}
impl RustViewmodel {
    /// A pistol with a shoulder stock on (the polymer stock clamped under the grip heel) is held
    /// as its own stance: butt in the shoulder, gun at the eye, instead of out at arm's length.
    fn stocked(&self) -> bool {
        super::view_utils::nodes(&get(&self.hand, "mount"), true)
            .iter()
            .any(|node| {
                let data = get(node, "userData");
                string(&get(&data, "slot")) == "stock"
                    && get(&data, "factory").as_bool() == Some(false)
                    && shoulder_stock(node)
            })
    }
    fn place(&self, side: usize, matrix: &[f64], camera: &JsValue, (bend, spread): (f64, f64)) {
        let [upper, lower, hand] = &self.arms.as_ref().unwrap()[side];
        // One walk up from the hand brings the whole arm up to date; the rest reads matrices.
        refresh(hand);
        let at = world_matrix(hand);
        let (wrist, elbow) = (matrix_position(&at), matrix_position(&world_matrix(lower)));
        let mut input = matrix.to_vec();
        input.extend(wrist);
        input.extend(world_matrix(camera));
        input.extend([self.weight, if side == 1 { spread } else { -spread }]);
        input.extend(matrix_rotation(&at));
        input.extend(matrix_position(&world_matrix(upper)));
        input.extend(components(&self.forearms[side]));
        let (dx, dy, dz) = (elbow[0] - wrist[0], elbow[1] - wrist[1], elbow[2] - wrist[2]);
        input.push((dx * dx + dy * dy + dz * dz).sqrt());
        input.push(bend);
        let pose = math("placeHand", &input);
        reach(upper, lower, hand, &pose[..3], &pose[3..6]);
        let mut input = matrix_rotation(&world_matrix(hand)).to_vec();
        input.extend(&pose[6..10]);
        input.push(self.weight);
        set_world_rotation(hand, &math("worldBlend", &input));
    }
}
/// Longer than 20 cm: a stock to shoulder, not panels on the grip (checked once per node).
fn shoulder_stock(node: &JsValue) -> bool {
    let data = get(node, "userData");
    let known = get(&data, "shoulderStock");
    if let Some(known) = known.as_bool() {
        return known;
    }
    let mut radius: f64 = 0.;
    for mesh in super::view_utils::nodes(node, false) {
        let geometry = get(&mesh, "geometry");
        if !present(&geometry) {
            continue;
        }
        call(&geometry, "computeBoundingSphere", &[]);
        radius = radius.max(num(&get(&get(&geometry, "boundingSphere"), "radius")));
    }
    let scale = get(&get(node, "matrixWorld"), "elements");
    let scale = num(&call(&scale, "at", &[0.into()])).hypot(num(&call(&scale, "at", &[1.into()]))).hypot(num(&call(&scale, "at", &[2.into()])));
    let long = radius * scale > 0.1;
    set(&data, "shoulderStock", &long.into());
    long
}
