//! The «Зонт» field around a fighter. It stays hidden and only the section over the zone
//! that was hit shows when it stops a hit: a flash where the shot lands, a wave over that
//! section lighting its hex cells, deflected sparks (and the bullet going off), then a fade.
//! Flight and wave are worked out on the GPU; the CPU writes the spark buffers once per hit.
//! In heat and in rain the whole field shows faintly at rest: hot air ripples through it,
//! drops run down it. It is the whole dome, so it never tells which zones are guarded.
use super::bridge::*;
use js_sys::{Float32Array, Math, Uint16Array};
use wasm_bindgen::prelude::*;

const DOME_VERTEX: &str = include_str!("shaders/shieldDomeVertex.glsl");
const DOME_FRAGMENT: &str = include_str!("shaders/shieldDomeFragment.glsl");
const SPARK_VERTEX: &str = include_str!("shaders/shieldSparkVertex.glsl");
const SPARK_FRAGMENT: &str = include_str!("shaders/shieldSparkFragment.glsl");
/// The dome in parts of the fighter's height: centre above the feet, half-height, radius.
const CENTER: f64 = 0.5;
const HALF: f64 = 0.62;
const RADIUS: f64 = 0.52;
/// Sparks on the best setting; lower settings throw fewer of them.
const MAX_SPARKS: usize = 28;
const CYAN: u32 = 0x7fe6f2;

fn n(v: f64) -> JsValue {
    v.into()
}
fn make(three: &JsValue, class: &str, args: &[JsValue]) -> JsValue {
    construct(&get(three, class), args)
}
fn uniforms(values: &[(&str, JsValue)]) -> JsValue {
    let out = object(&[]);
    for (key, value) in values {
        set(&out, key, &object(&[("value", value.clone())]));
    }
    out
}
fn put(material: &JsValue, key: &str, value: JsValue) {
    set(&get(&get(material, "uniforms"), key), "value", &value);
}
fn uniform(material: &JsValue, key: &str) -> JsValue {
    get(&get(&get(material, "uniforms"), key), "value")
}
/// Adds light without covering what is behind it: the lens composes premultiplied layers.
fn additive(three: &JsValue, material: &[(&str, JsValue)]) -> JsValue {
    let mut fields = vec![
        ("transparent", true.into()),
        ("depthWrite", false.into()),
        ("toneMapped", false.into()),
        ("blending", get(three, "CustomBlending")),
        ("blendSrc", get(three, "OneFactor")),
        ("blendDst", get(three, "OneFactor")),
        ("blendSrcAlpha", get(three, "ZeroFactor")),
        ("blendDstAlpha", get(three, "OneFactor")),
    ];
    fields.extend(material.iter().cloned());
    make(three, "ShaderMaterial", &[object(&fields)])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt().max(1e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}
fn add(a: [f64; 3], b: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn random_unit() -> [f64; 3] {
    let z = Math::random() * 2. - 1.;
    let a = Math::random() * std::f64::consts::TAU;
    let r = (1. - z * z).sqrt();
    [r * a.cos(), z, r * a.sin()]
}

pub struct ShieldFx {
    dome: JsValue,
    sparks: JsValue,
    dome_mat: JsValue,
    spark_mat: JsValue,
    dir: Float32Array,
    spark: Float32Array,
    age: f64,
    life: f64,
    count: usize,
    /// How much the resting field shows (heat, rain); zero hides it between hits.
    rest: f64,
    time: f64,
}

impl ShieldFx {
    /// `layer` picks the render pass (the far scene or the near one around the camera);
    /// `wide_k`/`wide_y` are the first-person lens uniforms shared with that pass.
    pub fn new(three: &JsValue, scene: &JsValue, layer: f64, wide_k: &JsValue, wide_y: &JsValue) -> Self {
        let theta = (-CENTER / HALF).acos();
        let geometry = make(
            three,
            "SphereGeometry",
            &[
                n(1.),
                n(48.),
                n(28.),
                n(0.),
                n(std::f64::consts::TAU),
                n(0.),
                n(theta),
            ],
        );
        let color = |hex: u32| make(three, "Color", &[hex.into()]);
        let dome_mat = additive(
            three,
            &[
                ("vertexShader", DOME_VERTEX.into()),
                ("fragmentShader", DOME_FRAGMENT.into()),
                ("side", get(three, "DoubleSide")),
                (
                    "uniforms",
                    uniforms(&[
                        ("uHit", make(three, "Vector3", &[n(0.), n(0.), n(1.)])),
                        ("uAge", n(0.)),
                        // no hit yet: nothing lit until the first one
                        ("uLife", n(0.)),
                        ("uPower", n(1.)),
                        ("uBroken", n(0.)),
                        ("uCells", n(30.)),
                        ("uReach", n(0.4)),
                        ("uDetail", n(1.)),
                        ("uHeat", n(0.)),
                        ("uWet", n(0.)),
                        ("uTime", n(0.)),
                        ("uColor", color(CYAN)),
                    ]),
                ),
            ],
        );
        let dome = make(three, "Mesh", &[geometry, dome_mat.clone()]);
        let dir = Float32Array::new_with_length((MAX_SPARKS * 12) as u32);
        let spark = Float32Array::new_with_length((MAX_SPARKS * 16) as u32);
        let corner = Float32Array::new_with_length((MAX_SPARKS * 8) as u32);
        let index = Uint16Array::new_with_length((MAX_SPARKS * 6) as u32);
        for s in 0..MAX_SPARKS {
            for (c, (along, across)) in [(0., -1.), (0., 1.), (1., -1.), (1., 1.)].iter().enumerate() {
                corner.set_index((s * 8 + c * 2) as u32, *along);
                corner.set_index((s * 8 + c * 2 + 1) as u32, *across);
            }
            let v = (s * 4) as u16;
            for (i, k) in [0u16, 1, 2, 2, 1, 3].iter().enumerate() {
                index.set_index((s * 6 + i) as u32, v + k);
            }
        }
        let buffers = make(three, "BufferGeometry", &[]);
        let attribute = |array: &JsValue, size: f64| make(three, "BufferAttribute", &[array.clone(), n(size)]);
        call(&buffers, "setAttribute", &["aDir".into(), attribute(&dir.clone().into(), 3.)]);
        call(&buffers, "setAttribute", &["aSpark".into(), attribute(&spark.clone().into(), 4.)]);
        call(&buffers, "setAttribute", &["aCorner".into(), attribute(&corner.into(), 2.)]);
        call(&buffers, "setIndex", &[attribute(&index.into(), 1.)]);
        let spark_mat = additive(
            three,
            &[
                ("vertexShader", SPARK_VERTEX.into()),
                ("fragmentShader", SPARK_FRAGMENT.into()),
                // The streak is turned to the screen in the shader, so its winding may flip.
                ("side", get(three, "DoubleSide")),
                (
                    "uniforms",
                    uniforms(&[
                        ("uOrigin", make(three, "Vector3", &[])),
                        ("uAge", n(0.)),
                        ("uWidth", n(0.0045)),
                        ("uAspect", n(1.)),
                        ("uCool", color(CYAN)),
                        ("uWarm", color(0xffb36b)),
                    ]),
                ),
            ],
        );
        let sparks = make(three, "Mesh", &[buffers, spark_mat.clone()]);
        set(&sparks, "frustumCulled", &false.into());
        for (mesh, material, order) in [(&dome, &dome_mat, 20.), (&sparks, &spark_mat, 21.)] {
            set(&get(material, "uniforms"), "wideK", wide_k);
            set(&get(material, "uniforms"), "wideY", wide_y);
            set(mesh, "name", &"shield".into());
            set(mesh, "visible", &false.into());
            set(mesh, "renderOrder", &n(order));
            call(&get(mesh, "layers"), "set", &[n(layer)]);
            call(scene, "add", &[mesh.clone()]);
        }
        Self {
            dome,
            sparks,
            dome_mat,
            spark_mat,
            dir,
            spark,
            age: 0.,
            life: 0.,
            count: MAX_SPARKS,
            rest: 0.,
            time: 0.,
        }
    }
    /// Sparks and the echo wave follow the graphics setting; the dome itself is cheap everywhere.
    pub fn set_quality(&mut self, id: &str) {
        let (count, detail) = match id {
            "minimal" => (4, 0.),
            "low" => (8, 0.),
            "medium" => (14, 0.6),
            "high" => (20, 1.),
            _ => (MAX_SPARKS, 1.),
        };
        self.count = count;
        put(&self.dome_mat, "uDetail", n(detail));
        call(&get(&self.sparks, "geometry"), "setDrawRange", &[n(0.), n((count * 6) as f64)]);
    }
    /// How much the field shows at rest: `heat` ripples it, `wet` runs drops down it.
    pub fn set_weather(&mut self, heat: f64, wet: f64) {
        put(&self.dome_mat, "uHeat", n(heat));
        put(&self.dome_mat, "uWet", n(wet));
        self.rest = heat + wet;
        if self.rest <= 0. && self.age >= self.life {
            set(&self.dome, "visible", &false.into());
        }
    }
    /// Keeps the resting field round the fighter standing on `feet`, `height` tall.
    pub fn follow(&mut self, feet: [f64; 3], height: f64) {
        if self.rest <= 0. {
            return;
        }
        let c = [feet[0], feet[1] + CENTER * height, feet[2]];
        call(&get(&self.dome, "position"), "set", &[n(c[0]), n(c[1]), n(c[2])]);
        call(
            &get(&self.dome, "scale"),
            "set",
            &[n(RADIUS * height), n(HALF * height), n(RADIUS * height)],
        );
        put(&self.dome_mat, "uCells", n((height * 36.).round().max(40.)));
        set(&self.dome, "visible", &true.into());
    }
    /// A blocked hit on the fighter standing on `feet` (world), `height` tall.
    /// The shot runs from `from` towards the zone point `to`; `weapon` is the catalogue id.
    pub fn fire(&mut self, feet: [f64; 3], height: f64, from: [f64; 3], to: [f64; 3], weapon: &str, broken: bool) {
        let c = [feet[0], feet[1] + CENTER * height, feet[2]];
        let r = [RADIUS * height, HALF * height, RADIUS * height];
        let local = |p: [f64; 3]| [(p[0] - c[0]) / r[0], (p[1] - c[1]) / r[1], (p[2] - c[2]) / r[2]];
        // Where the shot's line enters the dome (a unit sphere in its own space).
        let a = local(from);
        let d = add(local(to), a, -1.);
        let (qa, qb, qc) = (dot(d, d), 2. * dot(a, d), dot(a, a) - 1.);
        let disc = qb * qb - 4. * qa * qc;
        let hit = if qa > 1e-9 && disc >= 0. && qc > 0. {
            unit(add(a, d, (-qb - disc.sqrt()) / (2. * qa)))
        } else {
            unit([a[0], a[1].max(-0.2), a[2]])
        };
        let point = [c[0] + hit[0] * r[0], c[1] + hit[1] * r[1], c[2] + hit[2] * r[2]];
        let normal = unit([hit[0] / r[0], hit[1] / r[1], hit[2] / r[2]]);
        let incoming = unit(add(to, from, -1.));
        // The bullet glances off along the surface, the way the shot was going; a shot
        // straight on picks a side, so the streak still crosses the screen.
        let along = add(incoming, normal, -dot(incoming, normal));
        let flip = if Math::random() < 0.5 { -1. } else { 1. };
        let glance = if dot(along, along) > 0.04 {
            unit(along)
        } else {
            unit(cross(normal, [0., flip, 0.]))
        };
        let knife = weapon == "knife";
        let gun = !knife && weapon != "grenade";
        self.life = if broken { 1.05 } else if knife { 0.6 } else { 0.8 };
        self.age = 0.;
        put(&self.dome_mat, "uLife", n(self.life));
        put(&self.dome_mat, "uAge", n(0.));
        put(&self.dome_mat, "uPower", n(if broken { 1.5 } else if knife { 0.8 } else { 1. }));
        put(&self.dome_mat, "uBroken", n(if broken { 1. } else { 0. }));
        put(&self.dome_mat, "uCells", n((height * 36.).round().max(40.)));
        // Only the section over the hit zone lights up (radians of the dome).
        put(&self.dome_mat, "uReach", n(if broken { 0.55 } else if knife { 0.34 } else { 0.42 }));
        call(&uniform(&self.dome_mat, "uHit"), "set", &[n(hit[0]), n(hit[1]), n(hit[2])]);
        call(&get(&self.dome, "position"), "set", &[n(c[0]), n(c[1]), n(c[2])]);
        call(&get(&self.dome, "scale"), "set", &[n(r[0]), n(r[1]), n(r[2])]);
        call(&uniform(&self.spark_mat, "uOrigin"), "set", &[n(point[0]), n(point[1]), n(point[2])]);
        put(&self.spark_mat, "uAge", n(0.));
        // Sparks fan out across the surface (thrown at the camera they would read as dots);
        // a knife scrapes them downwards, and the bullet itself glances off as a long streak.
        for s in 0..MAX_SPARKS {
            let tracer = gun && s == 0;
            let side = unit(cross(normal, random_unit()));
            let dir = if tracer {
                unit(add(add(glance, normal, 0.35), [0., 1., 0.], 0.12))
            } else if knife {
                unit(add(add(side, normal, 0.3), [0., -1., 0.], 0.3))
            } else {
                unit(add(add(side, normal, 0.3 + Math::random() * 0.4), [0., 1., 0.], 0.25))
            };
            let speed = if tracer {
                24.
            } else {
                (if knife { 2.2 } else { 4. }) + Math::random() * if knife { 2.6 } else { 5. }
            };
            let life = if tracer { 0.3 } else { 0.2 + Math::random() * 0.3 };
            let width = if tracer { 1.4 } else { 0.6 + Math::random() * 0.5 };
            for v in 0..4 {
                let i = (s * 4 + v) as u32;
                for k in 0..3 {
                    self.dir.set_index(i * 3 + k, dir[k as usize] as f32);
                }
                for (k, value) in [speed, life, width, if tracer { 1. } else { 0. }].iter().enumerate() {
                    self.spark.set_index(i * 4 + k as u32, *value as f32);
                }
            }
        }
        let geometry = get(&self.sparks, "geometry");
        for name in ["aDir", "aSpark"] {
            set(&call(&geometry, "getAttribute", &[name.into()]), "needsUpdate", &true.into());
        }
        set(&self.dome, "visible", &true.into());
        set(&self.sparks, "visible", &true.into());
    }
    pub fn update(&mut self, dt: f64, aspect: f64) {
        self.time += dt;
        if self.rest > 0. {
            put(&self.dome_mat, "uTime", n(self.time % 1000.));
        }
        if self.age >= self.life {
            return;
        }
        self.age += dt;
        let on = self.age < self.life;
        put(&self.dome_mat, "uAge", n(self.age));
        put(&self.spark_mat, "uAge", n(self.age));
        put(&self.spark_mat, "uAspect", n(aspect));
        set(&self.dome, "visible", &(on || self.rest > 0.).into());
        set(&self.sparks, "visible", &on.into());
    }
    pub fn stop(&mut self) {
        self.age = self.life;
        set(&self.dome, "visible", &(self.rest > 0.).into());
        set(&self.sparks, "visible", &false.into());
    }
    pub fn dispose(&self) {
        for mesh in [&self.dome, &self.sparks] {
            call(&get(mesh, "geometry"), "dispose", &[]);
            call(&get(mesh, "material"), "dispose", &[]);
            let parent = get(mesh, "parent");
            if present(&parent) {
                call(&parent, "remove", &[mesh.clone()]);
            }
        }
    }
}
