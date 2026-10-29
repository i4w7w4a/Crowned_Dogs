//! Dust geometry capacity, shader uniforms and GPU-resource ownership.
use super::{bridge::*, kinematics::*};
use js_sys::{Float32Array, Uint32Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct RustDust {
    three: JsValue,
    points: JsValue,
    material: JsValue,
    capacity: u32,
    disposed: bool,
}
fn uniform(value: JsValue) -> JsValue {
    object(&[("value", value)])
}
#[wasm_bindgen]
impl RustDust {
    #[wasm_bindgen(constructor)]
    pub fn new(three: JsValue) -> Self {
        let shaders = game_motion::dispatch("motion.dustShaders", serde_json::json!([]))
            .expect("dust shaders");
        let sun = vector(&three, &[0., 0.5, -1.]);
        call(&sun, "normalize", &[]);
        let color = make(&three, "Color", &[0xe9eef2_u32.into()]);
        call(&color, "multiplyScalar", &[0.55.into()]);
        let uniforms = object(&[
            ("time", uniform(0.into())),
            ("pxPerMetre", uniform(800.into())),
            ("boxMin", uniform(vector(&three, &[-4., 0.1, -16.]))),
            ("boxSize", uniform(vector(&three, &[8., 4.6, 15.]))),
            ("sunDir", uniform(sun)),
            ("tRays", uniform(JsValue::NULL)),
            ("hasRays", uniform(0.into())),
            ("focusInv", uniform(0.into())),
            ("dofPx", uniform(0.into())),
            ("color", uniform(color)),
        ]);
        let material = make(
            &three,
            "ShaderMaterial",
            &[object(&[
                ("vertexShader", json(&shaders["vertex"])),
                ("fragmentShader", json(&shaders["fragment"])),
                ("uniforms", uniforms),
                ("transparent", true.into()),
                ("depthWrite", false.into()),
                ("fog", false.into()),
                ("blending", get(&three, "CustomBlending")),
                ("blendSrc", get(&three, "OneFactor")),
                ("blendDst", get(&three, "OneFactor")),
                ("blendSrcAlpha", get(&three, "ZeroFactor")),
                ("blendDstAlpha", get(&three, "OneFactor")),
            ])],
        );
        let points = make(
            &three,
            "Points",
            &[make(&three, "BufferGeometry", &[]), material.clone()],
        );
        set(&points, "frustumCulled", &false.into());
        set(&points, "visible", &false.into());
        Self {
            three,
            points,
            material,
            capacity: 0,
            disposed: false,
        }
    }
    #[wasm_bindgen(getter)]
    pub fn points(&self) -> JsValue {
        self.points.clone()
    }
    #[wasm_bindgen(js_name = setCount)]
    pub fn set_count(&mut self, count: u32) {
        if self.disposed {
            return;
        }
        set(&self.points, "visible", &(count > 0).into());
        if count > self.capacity {
            let random = Uint32Array::new_with_length(1);
            call(
                &get(&global(), "crypto"),
                "getRandomValues",
                &[random.clone().into()],
            );
            let seed = random.get_index(0).max(1);
            let packed = math("dust", &[count as f64, seed as f64]);
            let split = count as usize * 3;
            let positions = packed[..split]
                .iter()
                .map(|v| *v as f32)
                .collect::<Vec<_>>();
            let seeds = packed[split..]
                .iter()
                .map(|v| *v as f32)
                .collect::<Vec<_>>();
            call(&get(&self.points, "geometry"), "dispose", &[]);
            let geometry = make(&self.three, "BufferGeometry", &[]);
            for (name, values, size) in [("position", positions, 3), ("seed", seeds, 4)] {
                let attribute = make(
                    &self.three,
                    "BufferAttribute",
                    &[Float32Array::from(values.as_slice()).into(), size.into()],
                );
                call(&geometry, "setAttribute", &[name.into(), attribute]);
            }
            set(&self.points, "geometry", &geometry);
            self.capacity = count;
        }
        call(
            &get(&self.points, "geometry"),
            "setDrawRange",
            &[0.into(), count.into()],
        );
    }
    pub fn update(
        &self,
        time: f64,
        px_per_metre: f64,
        sun: &JsValue,
        rays: JsValue,
        focus: f64,
        dof: f64,
    ) {
        if self.disposed {
            return;
        }
        let uniforms = get(&self.material, "uniforms");
        for (name, value) in [
            ("time", time),
            ("pxPerMetre", px_per_metre),
            ("hasRays", f64::from(present(&rays))),
            ("focusInv", focus),
            ("dofPx", dof),
        ] {
            set(&get(&uniforms, name), "value", &value.into());
        }
        copy(&get(&get(&uniforms, "sunDir"), "value"), sun);
        set(&get(&uniforms, "tRays"), "value", &rays);
    }
    pub fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        set(&self.points, "visible", &false.into());
        call(&self.points, "removeFromParent", &[]);
        call(&get(&self.points, "geometry"), "dispose", &[]);
        call(&self.material, "dispose", &[]);
    }
}
impl Drop for RustDust {
    fn drop(&mut self) {
        self.dispose();
    }
}
