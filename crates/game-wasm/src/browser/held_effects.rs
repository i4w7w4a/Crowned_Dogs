//! Pistol reach correction and muzzle-flash lifetime, independent of the renderer loop.
use super::{bridge::*, kinematics::*};
use wasm_bindgen::prelude::*;

struct Arm {
    upper: JsValue,
    lower: JsValue,
    hand: JsValue,
    out: f64,
}
#[wasm_bindgen]
pub struct RustHoldOut {
    three: JsValue,
    arms: Vec<Arm>,
}
#[wasm_bindgen]
impl RustHoldOut {
    #[wasm_bindgen(constructor)]
    pub fn new(three: JsValue, model: &JsValue) -> Self {
        let mut arms = vec![];
        for (side, out) in [("l", 1.), ("r", -1.)] {
            let bone = |name: &str| find(model, &[&format!("{name}_{side}")]);
            let arm = Arm {
                upper: bone("upperarm"),
                lower: bone("lowerarm"),
                hand: bone("hand"),
                out,
            };
            if [&arm.upper, &arm.lower, &arm.hand]
                .iter()
                .all(|o| present(o))
            {
                arms.push(arm);
            }
        }
        Self { three, arms }
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
}

#[wasm_bindgen]
pub struct RustFlash {
    sprite: JsValue,
    life: f64,
    hand: JsValue,
    size: f64,
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
                ("map", map),
                ("blending", get(three, "AdditiveBlending")),
                ("depthWrite", false.into()),
                ("transparent", true.into()),
                ("fog", false.into()),
            ])],
        );
        let sprite = make(three, "Sprite", &[material]);
        call(&get(&sprite, "layers"), "set", &[layer.into()]);
        set(&sprite, "visible", &false.into());
        Self {
            sprite,
            life: 0.,
            hand: JsValue::NULL,
            size: 0.2,
            disposed: false,
        }
    }
    #[wasm_bindgen(getter)]
    pub fn sprite(&self) -> JsValue {
        self.sprite.clone()
    }
    pub fn fire(&mut self, hand: JsValue, size: f64) {
        if !self.disposed {
            self.hand = hand;
            self.size = size;
            self.life = 0.07;
        }
    }
    pub fn update(&mut self, dt: f64) -> f64 {
        if self.disposed {
            return 0.;
        }
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
        set(&self.sprite, "visible", &false.into());
    }
    pub fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.stop();
        self.disposed = true;
        self.hand = JsValue::NULL;
        call(&self.sprite, "removeFromParent", &[]);
        call(&get(&self.sprite, "material"), "dispose", &[]);
    }
}
impl Drop for RustFlash {
    fn drop(&mut self) {
        self.dispose();
    }
}
