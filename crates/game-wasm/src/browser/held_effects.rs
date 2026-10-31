//! Two-hand holds (pistol reach, the support hand on the gun) and shot effects (flash, smoke,
//! tracer, bursts), independent of the renderer loop.
use super::{bridge::*, kinematics::*};
use wasm_bindgen::prelude::*;

struct Arm {
    upper: JsValue,
    lower: JsValue,
    hand: JsValue,
    out: f64,
    /// The forearm's line in the hand's frame at bind: a wrist lined up with it is straight.
    forearm: JsValue,
}
/// The left hand's grip on the gun: its clavicle, finger curls and palm frame, all from bind.
struct Support {
    clavicle: JsValue,
    grip: RustGrip,
    palm: JsValue,
}
#[wasm_bindgen]
pub struct RustHoldOut {
    three: JsValue,
    model: JsValue,
    arms: Vec<Arm>,
    support: Option<Support>,
}
#[wasm_bindgen]
impl RustHoldOut {
    #[wasm_bindgen(constructor)]
    pub fn new(three: JsValue, model: &JsValue) -> Self {
        let mut arms = vec![];
        let support = at_bind(&three, model, || {
            for (side, out) in [("l", 1.), ("r", -1.)] {
                let bone = |name: &str| find(model, &[&format!("{name}_{side}")]);
                let (upper, lower, hand) = (bone("upperarm"), bone("lowerarm"), bone("hand"));
                if [&upper, &lower, &hand].iter().all(|o| present(o)) {
                    arms.push(Arm {
                        forearm: forearm(&three, &lower, &hand),
                        upper,
                        lower,
                        hand,
                        out,
                    });
                }
            }
            let host = object(&[("THREE", three.clone())]);
            let palm = super::scene_assets::grip_of(host, model.clone(), "l".into());
            let clavicle = find(model, &["clavicle_l"]);
            (present(&palm) && present(&clavicle)).then(|| Support {
                clavicle,
                grip: RustGrip::new(&three, model, "l"),
                palm,
            })
        });
        Self {
            three,
            model: model.clone(),
            arms,
            support,
        }
    }
    pub fn apply(&self, group: &JsValue, weight: f64) {
        if weight < 0.01 || self.arms.len() < 2 {
            return;
        }
        let group_turn = components(&world_quat(&self.three, group));
        for arm in &self.arms {
            let keep = world_quat(&self.three, &arm.hand);
            let mut input = group_turn.clone();
            input.extend([weight, arm.out]);
            for bone in [&arm.hand, &arm.lower, &arm.upper] {
                input.extend(components(&world_position(&self.three, bone)));
            }
            let pose = math("holdOut", &input);
            rig_reach(
                &self.three,
                &arm.upper,
                &arm.lower,
                &arm.hand,
                &vector(&self.three, &pose[..3]),
                &vector(&self.three, &pose[3..6]),
            );
            rig_set_world_quat(&self.three, &arm.hand, &keep);
        }
    }
    /// Puts the left hand on the gun that `hand` shows (a wrap round the pistol hand, the pump,
    /// the forend or the foregrip) and, on the pistol, lines the right forearm up with its wrist.
    /// `weight` is how far the body holds the gun up.
    pub fn support(&self, hand: &JsValue, weight: f64) {
        let held = string(&get(hand, "held"));
        let stance = ["pistol", "rifle", "shotgun", "sniper"]
            .iter()
            .position(|s| *s == held);
        let (Some(stance), Some(support)) = (stance, self.support.as_ref()) else {
            return;
        };
        if weight < 0.01 || self.arms.len() < 2 {
            return;
        }
        let three = &self.three;
        if stance == 0 {
            let arm = &self.arms[1];
            let keep = world_quat(three, &arm.hand);
            let mut input = vec![];
            for bone in [&arm.upper, &arm.lower, &arm.hand] {
                input.extend(components(&world_position(three, bone)));
            }
            input.extend(components(&keep));
            input.extend(components(&arm.forearm));
            input.push(weight);
            let pole = math("wristPole", &input);
            let at = world_position(three, &arm.hand);
            rig_reach(three, &arm.upper, &arm.lower, &arm.hand, &at, &vector(three, &pole));
            rig_set_world_quat(three, &arm.hand, &keep);
        }
        let mount = get(hand, "mount");
        call(&mount, "updateWorldMatrix", &[true.into(), false.into()]);
        let mut input = numbers(&get(&get(&mount, "matrixWorld"), "elements"));
        input.extend(numbers(&get(&support.palm, "elements")));
        input.extend([stance as f64, f64::from(boolean(&get(hand, "foregrip")))]);
        let grip = math("support", &input);
        let arm = &self.arms[0];
        let mut input = grip[..16].to_vec();
        input.push(weight);
        for bone in [&support.clavicle, &arm.upper, &arm.lower, &arm.hand] {
            input.extend(components(&world_position(three, bone)));
        }
        input.extend(components(&world_quat(three, &arm.hand)));
        input.extend(components(&world_quat(three, &self.model)));
        let pose = math("supportArm", &input);
        if pose[..3] != pose[3..6] {
            let (from, to) = (vector(three, &pose[..3]), vector(three, &pose[3..6]));
            rig_turn(three, &support.clavicle, &from, &to);
        }
        let (target, pole) = (vector(three, &pose[6..9]), vector(three, &pose[9..12]));
        rig_reach(three, &arm.upper, &arm.lower, &arm.hand, &target, &pole);
        let rotation = quaternion(three);
        upload(&rotation, &pose[12..16]);
        rig_set_world_quat(three, &arm.hand, &rotation);
        support.grip.close(&cupped(grip[16], grip[17]), weight);
    }
}

/// Seconds between rounds of an automatic burst; the shot cues in `game-client` use the same gap.
const ROUND_GAP: f64 = 0.09;
const TRACER_LIFE: f64 = 0.08;
const SMOKE_LIFE: f64 = 0.7;
#[wasm_bindgen]
pub struct RustFlash {
    three: JsValue,
    sprite: JsValue,
    smoke: JsValue,
    tracer: JsValue,
    life: f64,
    hand: JsValue,
    size: f64,
    /// Rounds still to fire in this burst and the time to the next one.
    rounds: u32,
    gap: f64,
    target: JsValue,
    from: JsValue,
    reach: f64,
    trace: f64,
    puff: f64,
    fired: bool,
    disposed: bool,
}
#[wasm_bindgen]
impl RustFlash {
    #[wasm_bindgen(constructor)]
    pub fn new(three: &JsValue, map: JsValue, layer: u32) -> Self {
        let material = make(
            three,
            "SpriteMaterial",
            &[object(&[
                ("map", map.clone()),
                ("blending", get(three, "AdditiveBlending")),
                ("depthWrite", false.into()),
                ("transparent", true.into()),
                ("fog", false.into()),
            ])],
        );
        let sprite = make(three, "Sprite", &[material]);
        // Powder smoke: the same soft disc, dimmed to grey and drawn normally.
        let smoke = make(
            three,
            "Sprite",
            &[make(
                three,
                "SpriteMaterial",
                &[object(&[
                    ("map", map),
                    ("color", 0x7d8184.into()),
                    ("depthWrite", false.into()),
                    ("transparent", true.into()),
                ])],
            )],
        );
        // A hot streak from the muzzle to the target; the box spans z 0..1, so scale.z is its length.
        let geometry = make(three, "BoxGeometry", &[1.into(), 1.into(), 1.into()]);
        call(&geometry, "translate", &[0.into(), 0.into(), 0.5.into()]);
        let tracer = make(
            three,
            "Mesh",
            &[
                geometry,
                make(
                    three,
                    "MeshBasicMaterial",
                    &[object(&[
                        ("color", 0xffd49a.into()),
                        ("blending", get(three, "AdditiveBlending")),
                        ("depthWrite", false.into()),
                        ("transparent", true.into()),
                        ("fog", false.into()),
                    ])],
                ),
            ],
        );
        set(&tracer, "frustumCulled", &false.into());
        for part in [&sprite, &smoke, &tracer] {
            call(&get(part, "layers"), "set", &[layer.into()]);
            set(part, "visible", &false.into());
        }
        Self {
            three: three.clone(),
            sprite,
            smoke,
            tracer,
            life: 0.,
            hand: JsValue::NULL,
            size: 0.2,
            rounds: 0,
            gap: 0.,
            target: make(three, "Vector3", &[]),
            from: make(three, "Vector3", &[]),
            reach: 0.,
            trace: 0.,
            puff: 0.,
            fired: false,
            disposed: false,
        }
    }
    #[wasm_bindgen(getter)]
    pub fn sprite(&self) -> JsValue {
        self.sprite.clone()
    }
    /// Everything the scene must hold: flash, smoke and tracer.
    #[wasm_bindgen(getter)]
    pub fn parts(&self) -> js_sys::Array {
        array(&[self.sprite.clone(), self.smoke.clone(), self.tracer.clone()])
    }
    /// True when the last update fired a round (for the first-person kick).
    #[wasm_bindgen(getter)]
    pub fn fired(&self) -> bool {
        self.fired
    }
    /// Starts a shot of `rounds` rounds toward `target` (a world point, or null for no tracer).
    pub fn fire(&mut self, hand: JsValue, size: f64, rounds: u32, target: JsValue) {
        if self.disposed {
            return;
        }
        self.hand = hand;
        self.size = size;
        self.rounds = rounds.max(1);
        self.gap = 0.;
        if present(&target) {
            call(&self.target, "copy", &[target]);
            self.reach = 1.;
        } else {
            self.reach = 0.;
        }
    }
    pub fn update(&mut self, dt: f64) -> f64 {
        if self.disposed {
            return 0.;
        }
        self.fired = false;
        if present(&self.hand) {
            // A plain hand without recoil (tests, props) is fine too.
            let _ = try_call(&self.hand, "settle", &[dt.into()]);
        }
        if self.rounds > 0 {
            self.gap -= dt;
            if self.gap <= 0. {
                self.round();
            }
        }
        self.update_smoke(dt);
        self.update_tracer(dt);
        let frame = math("flash", &[self.life, dt, self.size]);
        self.life = frame[0];
        let k = frame[1];
        let visible = k > 0.
            && present(&self.hand)
            && boolean(&call(
                &self.hand,
                "muzzle",
                &[get(&self.sprite, "position")],
            ));
        set(&self.sprite, "visible", &visible.into());
        if !visible {
            return 0.;
        }
        call(&get(&self.sprite, "scale"), "setScalar", &[frame[2].into()]);
        let material = get(&self.sprite, "material");
        set(
            &material,
            "rotation",
            &(js_sys::Math::random() * std::f64::consts::TAU).into(),
        );
        set(&material, "opacity", &k.into());
        k
    }
    pub fn stop(&mut self) {
        self.life = 0.;
        self.rounds = 0;
        self.trace = 0.;
        self.puff = 0.;
        for part in [&self.sprite, &self.smoke, &self.tracer] {
            set(part, "visible", &false.into());
        }
    }
    pub fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.stop();
        self.disposed = true;
        self.hand = JsValue::NULL;
        for part in [&self.sprite, &self.smoke, &self.tracer] {
            call(part, "removeFromParent", &[]);
            call(&get(part, "material"), "dispose", &[]);
        }
        call(&get(&self.tracer, "geometry"), "dispose", &[]);
    }
}
impl RustFlash {
    fn round(&mut self) {
        self.rounds -= 1;
        self.gap = ROUND_GAP;
        self.life = 0.07;
        self.fired = true;
        if !present(&self.hand) || !boolean(&call(&self.hand, "muzzle", &[self.from.clone()])) {
            return;
        }
        let _ = try_call(&self.hand, "kick", &[]);
        call(&get(&self.smoke, "position"), "copy", &[self.from.clone()]);
        self.puff = SMOKE_LIFE;
        if self.reach > 0. {
            // Each round strays a little, so a burst fans out instead of stacking on one line.
            let spread = |v: f64| v + (js_sys::Math::random() - 0.5) * 0.12;
            let t = &self.target;
            let aim = make(
                &self.three,
                "Vector3",
                &[
                    spread(num(&get(t, "x"))).into(),
                    spread(num(&get(t, "y"))).into(),
                    spread(num(&get(t, "z"))).into(),
                ],
            );
            self.reach = num(&call(&self.from, "distanceTo", &[aim.clone()])).max(0.01);
            call(&get(&self.tracer, "position"), "copy", &[self.from.clone()]);
            call(&self.tracer, "lookAt", &[aim]);
            self.trace = TRACER_LIFE;
        }
    }
    fn update_smoke(&mut self, dt: f64) {
        if self.puff <= 0. {
            set(&self.smoke, "visible", &false.into());
            return;
        }
        self.puff = (self.puff - dt).max(0.);
        let k = self.puff / SMOKE_LIFE;
        let position = get(&self.smoke, "position");
        set(&position, "y", &(num(&get(&position, "y")) + dt * 0.12).into());
        call(
            &get(&self.smoke, "scale"),
            "setScalar",
            &[(self.size * (0.6 + 1.6 * (1. - k))).into()],
        );
        set(&get(&self.smoke, "material"), "opacity", &(0.22 * k).into());
        set(&self.smoke, "visible", &(k > 0.).into());
    }
    fn update_tracer(&mut self, dt: f64) {
        if self.trace <= 0. {
            set(&self.tracer, "visible", &false.into());
            return;
        }
        self.trace = (self.trace - dt).max(0.);
        // The head flies from the muzzle to the target; the tail trails up to 1.2 m behind it.
        let head = self.reach * (1. - self.trace / TRACER_LIFE);
        let tail = (head - 1.2).max(0.);
        let dir = call(
            &make(&self.three, "Vector3", &[0.into(), 0.into(), 1.into()]),
            "applyQuaternion",
            &[get(&self.tracer, "quaternion")],
        );
        let start = call(&self.from, "clone", &[]);
        call(&start, "addScaledVector", &[dir, tail.into()]);
        call(&get(&self.tracer, "position"), "copy", &[start]);
        let w = 0.006 + 0.01 * self.size;
        call(
            &get(&self.tracer, "scale"),
            "set",
            &[w.into(), w.into(), (head - tail).max(0.001).into()],
        );
        set(
            &get(&self.tracer, "material"),
            "opacity",
            &(0.35 + 0.65 * self.trace / TRACER_LIFE).into(),
        );
        set(&self.tracer, "visible", &(self.trace > 0.).into());
    }
}
impl Drop for RustFlash {
    fn drop(&mut self) {
        self.dispose();
    }
}
