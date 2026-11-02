//! The living world on the arena: the amber lines of a warned stretch on the floor, and the
//! strike itself — a lightning bolt, a shell's or a kamikaze drone's blast (fire, smoke, flying
//! grit), a ceiling giving way in chunks and dust. Everything is drawn in the fighters' pass, so
//! the lens tones it with them; between strikes nothing covers the picture but the two lines.
use super::bridge::*;
use js_sys::{Float32Array, Math};
use wasm_bindgen::prelude::*;

const AMBER: u32 = 0xffb347;
/// Ribbon quads of one bolt: the core and its halo over the main stroke and two forks.
const BOLT_QUADS: usize = 256;
const G: f64 = 9.8;
/// How long the warned stretch's lines are across the lane, in metres.
const MARK_LENGTH: f64 = 2.4;

fn n(v: f64) -> JsValue {
    v.into()
}
fn make(three: &JsValue, class: &str, args: &[JsValue]) -> JsValue {
    construct(&get(three, class), args)
}
fn rand(a: f64, b: f64) -> f64 {
    a + Math::random() * (b - a)
}
fn put3(o: &JsValue, v: [f64; 3]) {
    call(o, "set", &[n(v[0]), n(v[1]), n(v[2])]);
}
fn add(a: [f64; 3], b: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn length(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = length(a).max(1e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn canvas_texture(three: &JsValue, w: u32, h: u32, paint: impl FnOnce(&JsValue)) -> JsValue {
    let canvas = call(&document(), "createElement", &["canvas".into()]);
    set(&canvas, "width", &w.into());
    set(&canvas, "height", &h.into());
    paint(&call(&canvas, "getContext", &["2d".into()]));
    let t = make(three, "CanvasTexture", &[canvas]);
    set(&t, "colorSpace", &get(three, "SRGBColorSpace"));
    t
}
fn blob(g: &JsValue, x: f64, y: f64, r: f64, stops: &[(f64, &str)]) {
    let gradient = call(
        g,
        "createRadialGradient",
        &[n(x), n(y), n(0.), n(x), n(y), n(r)],
    );
    for (at, color) in stops {
        call(&gradient, "addColorStop", &[n(*at), (*color).into()]);
    }
    set(g, "fillStyle", &gradient);
    call(g, "fillRect", &[n(x - r), n(y - r), n(r * 2.), n(r * 2.)]);
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Fire,
    Smoke,
    Spark,
    Grit,
    Chunk,
}

/// One flying piece: a sprite of fire, smoke or spark, or a mesh of grit or concrete.
struct Bit {
    obj: JsValue,
    mat: JsValue,
    kind: Kind,
    pos: [f64; 3],
    vel: [f64; 3],
    spin: [f64; 3],
    turn: [f64; 3],
    shape: [f64; 3],
    size: [f64; 2],
    alpha: f64,
    /// Negative while it waits to start.
    age: f64,
    life: f64,
    floor: f64,
    landed: bool,
}

struct Flight {
    from: [f64; 3],
    bend: [f64; 3],
    to: [f64; 3],
    t: f64,
    dur: f64,
    power: f64,
}

pub struct HazardFx {
    three: JsValue,
    scene: JsValue,
    /// Everything hangs off one group, so the scene gains a single child.
    root: JsValue,
    owned: Vec<JsValue>,
    bits: Vec<Bit>,
    marks: [JsValue; 2],
    mark_mat: JsValue,
    mark_k: f64,
    mark_on: bool,
    time: f64,
    bolt: JsValue,
    bolt_mat: JsValue,
    bolt_pos: Float32Array,
    bolt_col: Float32Array,
    bolt_attr: [JsValue; 2],
    bolt_age: f64,
    glow: JsValue,
    glow_mat: JsValue,
    light: JsValue,
    light_peak: f64,
    light_age: f64,
    light_bolt: bool,
    drone: JsValue,
    flight: Option<Flight>,
    /// Blasts still on their way: seconds left, where, how strong.
    later: Vec<(f64, [f64; 3], f64)>,
    /// Close flashes for the lens: colour times strength, and whether it is lightning.
    flashes: Vec<([f64; 3], bool)>,
}

impl HazardFx {
    pub fn new(three: &JsValue, scene: &JsValue) -> Self {
        let mut owned = vec![];
        let root = make(three, "Group", &[]);
        set(&root, "name", &"hazard".into());
        call(scene, "add", &[root.clone()]);
        let additive = get(three, "AdditiveBlending");
        let fire_map = canvas_texture(three, 64, 64, |g| {
            blob(
                g,
                32.,
                32.,
                32.,
                &[
                    (0., "rgba(255,246,222,1)"),
                    (0.3, "rgba(255,176,92,.85)"),
                    (0.65, "rgba(200,80,24,.35)"),
                    (1., "rgba(120,40,10,0)"),
                ],
            );
        });
        // A puff of smoke: overlapping soft blobs, so it reads as a cloud and not a disc.
        let smoke_map = canvas_texture(three, 128, 128, |g| {
            for _ in 0..16 {
                let a = rand(0., std::f64::consts::TAU);
                let d = rand(0., 30.);
                blob(
                    g,
                    64. + a.cos() * d,
                    64. + a.sin() * d,
                    rand(22., 34.),
                    &[(0., "rgba(255,255,255,.32)"), (1., "rgba(255,255,255,0)")],
                );
            }
        });
        let spark_map = canvas_texture(three, 32, 32, |g| {
            blob(
                g,
                16.,
                16.,
                16.,
                &[
                    (0., "rgba(255,250,235,1)"),
                    (0.35, "rgba(255,200,130,.6)"),
                    (1., "rgba(255,140,60,0)"),
                ],
            );
        });
        let glow_map = canvas_texture(three, 64, 64, |g| {
            blob(
                g,
                32.,
                32.,
                32.,
                &[
                    (0., "rgba(240,246,255,1)"),
                    (0.2, "rgba(190,210,255,.55)"),
                    (1., "rgba(120,150,255,0)"),
                ],
            );
        });
        let dash_map = canvas_texture(three, 64, 8, |g| {
            blob(g, 20., 4., 4., &[(0., "rgba(255,255,255,1)"), (1., "rgba(255,255,255,.6)")]);
            set(g, "fillStyle", &"rgba(255,255,255,1)".into());
            call(g, "fillRect", &[n(0.), n(2.), n(40.), n(4.)]);
            set(g, "fillStyle", &"rgba(255,255,255,.45)".into());
            call(g, "fillRect", &[n(0.), n(1.), n(40.), n(1.)]);
            call(g, "fillRect", &[n(0.), n(6.), n(40.), n(1.)]);
        });
        set(&dash_map, "wrapS", &get(three, "RepeatWrapping"));
        call(
            &get(&dash_map, "repeat"),
            "set",
            &[n(MARK_LENGTH / 0.3), n(1.)],
        );
        owned.extend([
            fire_map.clone(),
            smoke_map.clone(),
            spark_map.clone(),
            glow_map.clone(),
            dash_map.clone(),
        ]);
        let add_obj = |o: JsValue| {
            set(&o, "visible", &false.into());
            call(&root, "add", &[o]);
        };
        let sprite = |map: &JsValue, glowing: bool| {
            let mut fields = vec![
                ("map", map.clone()),
                ("transparent", true.into()),
                ("depthWrite", false.into()),
                ("fog", false.into()),
            ];
            if glowing {
                fields.push(("blending", additive.clone()));
            }
            let mat = make(three, "SpriteMaterial", &[object(&fields)]);
            (make(three, "Sprite", &[mat.clone()]), mat)
        };
        let mut bits = vec![];
        for (kind, count) in [
            (Kind::Fire, 8),
            (Kind::Smoke, 28),
            (Kind::Spark, 24),
        ] {
            for _ in 0..count {
                let (obj, mat) = match kind {
                    Kind::Fire => sprite(&fire_map, true),
                    Kind::Smoke => sprite(&smoke_map, false),
                    _ => sprite(&spark_map, true),
                };
                owned.push(mat.clone());
                add_obj(obj.clone());
                bits.push(Bit::new(obj, mat, kind));
            }
        }
        let box_geo = make(three, "BoxGeometry", &[n(1.), n(1.), n(1.)]);
        let rock_geo = make(three, "IcosahedronGeometry", &[n(0.5), n(0.)]);
        let grit_mat = make(
            three,
            "MeshStandardMaterial",
            &[object(&[
                ("color", n(0x34312d as f64)),
                ("roughness", n(1.)),
            ])],
        );
        let rock_mat = make(
            three,
            "MeshStandardMaterial",
            &[object(&[
                ("color", n(0x7a766e as f64)),
                ("roughness", n(0.95)),
                ("flatShading", true.into()),
            ])],
        );
        owned.extend([box_geo.clone(), rock_geo.clone(), grit_mat.clone(), rock_mat.clone()]);
        for (kind, count, geo, mat) in [
            (Kind::Grit, 26, &box_geo, &grit_mat),
            (Kind::Chunk, 12, &rock_geo, &rock_mat),
        ] {
            for _ in 0..count {
                let obj = make(three, "Mesh", &[geo.clone(), mat.clone()]);
                set(&obj, "castShadow", &true.into());
                add_obj(obj.clone());
                bits.push(Bit::new(obj, mat.clone(), kind));
            }
        }
        // The warned stretch: two dashed amber lines across the lane on the floor.
        let mark_geo = make(three, "PlaneGeometry", &[n(1.), n(1.)]);
        call(&mark_geo, "rotateX", &[n(-std::f64::consts::FRAC_PI_2)]);
        let mark_mat = make(
            three,
            "MeshBasicMaterial",
            &[object(&[
                ("map", dash_map),
                ("color", n(AMBER as f64)),
                ("transparent", true.into()),
                ("opacity", n(0.)),
                ("blending", additive.clone()),
                ("depthWrite", false.into()),
                ("fog", false.into()),
            ])],
        );
        owned.extend([mark_geo.clone(), mark_mat.clone()]);
        let marks = [0, 1].map(|_| {
            let m = make(three, "Mesh", &[mark_geo.clone(), mark_mat.clone()]);
            add_obj(m.clone());
            m
        });
        // The bolt: ribbons turned to the camera, written fresh for every strike.
        let bolt_pos = Float32Array::new_with_length((BOLT_QUADS * 18) as u32);
        let bolt_col = Float32Array::new_with_length((BOLT_QUADS * 18) as u32);
        let bolt_geo = make(three, "BufferGeometry", &[]);
        let pos_attr = make(three, "BufferAttribute", &[bolt_pos.clone().into(), n(3.)]);
        let col_attr = make(three, "BufferAttribute", &[bolt_col.clone().into(), n(3.)]);
        call(&bolt_geo, "setAttribute", &["position".into(), pos_attr.clone()]);
        call(&bolt_geo, "setAttribute", &["color".into(), col_attr.clone()]);
        let bolt_mat = make(
            three,
            "MeshBasicMaterial",
            &[object(&[
                ("vertexColors", true.into()),
                ("transparent", true.into()),
                ("blending", additive.clone()),
                ("depthWrite", false.into()),
                ("side", get(three, "DoubleSide")),
                ("fog", false.into()),
            ])],
        );
        let bolt = make(three, "Mesh", &[bolt_geo.clone(), bolt_mat.clone()]);
        set(&bolt, "frustumCulled", &false.into());
        add_obj(bolt.clone());
        let (glow, glow_mat) = sprite(&glow_map, true);
        add_obj(glow.clone());
        owned.extend([bolt_geo, bolt_mat.clone(), glow_mat.clone()]);
        let light = make(three, "PointLight", &[n(0xffffff as f64), n(0.), n(22.), n(1.6)]);
        call(&get(&light, "layers"), "enable", &[n(1.)]);
        call(&root, "add", &[light.clone()]);
        let drone = Self::build_drone(three, &mut owned);
        add_obj(drone.clone());
        Self {
            three: three.clone(),
            scene: scene.clone(),
            root,
            owned,
            bits,
            marks,
            mark_mat,
            mark_k: 0.,
            mark_on: false,
            time: 0.,
            bolt,
            bolt_mat,
            bolt_pos,
            bolt_col,
            bolt_attr: [pos_attr, col_attr],
            bolt_age: 99.,
            glow,
            glow_mat,
            light,
            light_peak: 0.,
            light_age: 99.,
            light_bolt: false,
            drone,
            flight: None,
            later: vec![],
            flashes: vec![],
        }
    }

    /// A small quadcopter with a grenade slung under it, nose (+Z) first.
    fn build_drone(three: &JsValue, owned: &mut Vec<JsValue>) -> JsValue {
        let group = make(three, "Group", &[]);
        let body_mat = make(
            three,
            "MeshStandardMaterial",
            &[object(&[
                ("color", n(0x2b2c2a as f64)),
                ("roughness", n(0.6)),
                ("metalness", n(0.3)),
            ])],
        );
        let load_mat = make(
            three,
            "MeshStandardMaterial",
            &[object(&[("color", n(0x4b4f3b as f64)), ("roughness", n(0.8))])],
        );
        let rotor_mat = make(
            three,
            "MeshBasicMaterial",
            &[object(&[
                ("color", n(0x151515 as f64)),
                ("transparent", true.into()),
                ("opacity", n(0.35)),
                ("depthWrite", false.into()),
            ])],
        );
        let box_geo = make(three, "BoxGeometry", &[n(1.), n(1.), n(1.)]);
        let disc = make(three, "CylinderGeometry", &[n(0.11), n(0.11), n(0.006), n(20.)]);
        let load = make(three, "CylinderGeometry", &[n(0.04), n(0.04), n(0.26), n(10.)]);
        owned.extend([
            body_mat.clone(),
            load_mat.clone(),
            rotor_mat.clone(),
            box_geo.clone(),
            disc.clone(),
            load.clone(),
        ]);
        let part = |geo: &JsValue, mat: &JsValue, at: [f64; 3], scale: [f64; 3]| {
            let m = make(three, "Mesh", &[geo.clone(), mat.clone()]);
            put3(&get(&m, "position"), at);
            put3(&get(&m, "scale"), scale);
            call(&group, "add", &[m.clone()]);
            m
        };
        part(&box_geo, &body_mat, [0., 0., 0.], [0.16, 0.07, 0.22]);
        for (x, z) in [(1., 1.), (-1., 1.), (1., -1.), (-1., -1.)] {
            let arm = part(&box_geo, &body_mat, [x * 0.11, 0., z * 0.11], [0.3, 0.02, 0.025]);
            set(
                &get(&arm, "rotation"),
                "y",
                &n(if x * z > 0. { -0.785 } else { 0.785 }),
            );
            part(&box_geo, &body_mat, [x * 0.21, 0.03, z * 0.21], [0.03, 0.05, 0.03]);
            part(&disc, &rotor_mat, [x * 0.21, 0.06, z * 0.21], [1., 1., 1.]);
        }
        let shell = part(&load, &load_mat, [0., -0.07, 0.04], [1., 1., 1.]);
        set(&get(&shell, "rotation"), "x", &n(std::f64::consts::FRAC_PI_2));
        group
    }

    fn spawn(&mut self, kind: Kind) -> &mut Bit {
        let mut pick = None;
        let mut oldest = -1.;
        for (i, b) in self.bits.iter().enumerate() {
            if b.kind != kind {
                continue;
            }
            if b.age >= b.life {
                pick = Some(i);
                break;
            }
            let k = b.age / b.life.max(1e-3);
            if k > oldest {
                oldest = k;
                pick = Some(i);
            }
        }
        let b = &mut self.bits[pick.expect("a pool for every kind")];
        b.vel = [0.; 3];
        b.spin = [0.; 3];
        b.turn = [rand(0., 6.3), rand(0., 6.3), rand(0., 6.3)];
        b.age = 0.;
        b.landed = false;
        b.alpha = 1.;
        b
    }

    pub fn show_mark(&mut self, on: bool) {
        self.mark_on = on;
    }
    /// Lays the two lines across the lane at the stretch's edges; they keep a few pixels' width
    /// however far off they lie.
    pub fn place_mark(&mut self, edges: [[f64; 3]; 2], across: [f64; 3], camera: [f64; 3]) {
        let yaw = (-across[2]).atan2(across[0]);
        for (mesh, edge) in self.marks.iter().zip(edges) {
            put3(&get(mesh, "position"), [edge[0], edge[1] + 0.015, edge[2]]);
            set(&get(mesh, "rotation"), "y", &n(yaw));
            let thick = (0.006 * length(sub(edge, camera))).max(0.025);
            put3(&get(mesh, "scale"), [MARK_LENGTH, 1., thick]);
        }
    }
    pub fn marking(&self) -> bool {
        self.mark_on || self.mark_k > 0.01
    }

    fn light_up(&mut self, at: [f64; 3], color: u32, peak: f64, bolt: bool) {
        put3(&get(&self.light, "position"), at);
        call(&get(&self.light, "color"), "set", &[n(color as f64)]);
        self.light_peak = peak;
        self.light_age = 0.;
        self.light_bolt = bolt;
    }

    /// Lightning into the floor at `at`: the bolt with two forks, a white flash, sparks.
    pub fn bolt(&mut self, at: [f64; 3], camera: [f64; 3]) {
        let top = add(at, [rand(-4., 4.), 28., rand(-6., -2.)], 1.);
        let mut strokes = vec![(jag(top, at, 6, 0.2), 1.)];
        let main = strokes[0].0.clone();
        for _ in 0..2 {
            let from = main[(rand(0.25, 0.6) * main.len() as f64) as usize];
            let reach = rand(4., 9.);
            let end = add(from, [rand(-1., 1.) * reach * 0.6, -reach, rand(-1., 1.) * reach * 0.4], 1.);
            strokes.push((jag(from, end, 4, 0.25), 0.55));
        }
        let mut pos = vec![];
        let mut col = vec![];
        // the halo first, so the core is drawn over it
        for (width, shade) in [(5., 0.12), (1., 1.)] {
            for (points, weight) in &strokes {
                for pair in points.windows(2) {
                    let (p, q) = (pair[0], pair[1]);
                    let w = (0.0035 * length(sub(camera, p))).max(0.035) * width * weight;
                    let side = unit(cross(sub(q, p), sub(camera, p)));
                    let (a, b) = (add(p, side, -w / 2.), add(p, side, w / 2.));
                    let (c, d) = (add(q, side, w / 2.), add(q, side, -w / 2.));
                    for v in [a, b, c, a, c, d] {
                        pos.extend(v.map(|x| x as f32));
                        let k = (shade * weight) as f32;
                        col.extend([0.86 * k, 0.9 * k, k]);
                    }
                }
            }
        }
        let count = (pos.len() / 3).min(BOLT_QUADS * 6);
        self.bolt_pos.subarray(0, (count * 3) as u32).copy_from(&pos[..count * 3]);
        self.bolt_col.subarray(0, (count * 3) as u32).copy_from(&col[..count * 3]);
        for attr in &self.bolt_attr {
            set(attr, "needsUpdate", &true.into());
        }
        call(
            &get(&self.bolt, "geometry"),
            "setDrawRange",
            &[n(0.), n(count as f64)],
        );
        self.bolt_age = 0.;
        put3(&get(&self.glow, "position"), add(at, [0., 0.4, 0.], 1.));
        self.light_up(add(at, [0., 2.5, 0.], 1.), 0xcfdcff, 120., true);
        self.flashes.push(([0.7, 0.78, 0.95], true));
        for _ in 0..16 {
            let b = self.spawn(Kind::Spark);
            b.pos = add(at, [0., 0.05, 0.], 1.);
            let a = rand(0., std::f64::consts::TAU);
            let out = rand(1., 4.5);
            b.vel = [a.cos() * out, rand(2., 6.), a.sin() * out];
            b.size = [0.09, 0.09];
            b.life = rand(0.45, 1.);
            b.floor = at[1];
        }
        for _ in 0..4 {
            let b = self.spawn(Kind::Smoke);
            b.pos = add(at, [rand(-0.3, 0.3), 0.2, rand(-0.3, 0.3)], 1.);
            b.vel = [rand(-0.4, 0.4), rand(0.3, 0.8), rand(-0.4, 0.4)];
            b.size = [0.4, rand(1.4, 2.)];
            b.alpha = 0.35;
            b.life = rand(1.4, 2.);
            call(&get(&b.mat, "color"), "set", &[n(0x8d8a84 as f64)]);
        }
    }

    /// A blast at `at`: a flare of fire, grit and sparks thrown out, a column of dark smoke.
    /// `power` 1 is a mortar shell, less for the drone's grenade or a blast in my face.
    pub fn blast(&mut self, at: [f64; 3], power: f64) {
        let p = power;
        self.light_up(add(at, [0., 0.9, 0.], 1.), 0xffa055, 70. * p, false);
        self.flashes.push(([0.55 * p, 0.34 * p, 0.16 * p], false));
        for _ in 0..5 {
            let b = self.spawn(Kind::Fire);
            b.pos = add(at, [rand(-0.3, 0.3) * p, rand(0.2, 0.9) * p, rand(-0.3, 0.3) * p], 1.);
            b.vel = [rand(-1., 1.) * p, rand(1., 3.) * p, rand(-1., 1.) * p];
            b.size = [0.6 * p, rand(2.2, 3.2) * p];
            b.life = rand(0.3, 0.55);
            b.age = -rand(0., 0.06);
        }
        for _ in 0..10 {
            let b = self.spawn(Kind::Smoke);
            b.pos = add(at, [rand(-0.5, 0.5) * p, rand(0.2, 0.8) * p, rand(-0.5, 0.5) * p], 1.);
            b.vel = [rand(-1.2, 1.2) * p, rand(0.6, 1.8) * p, rand(-1.2, 1.2) * p];
            b.size = [p, rand(3.5, 5.) * p];
            b.alpha = rand(0.55, 0.8);
            b.life = rand(2.2, 3.6);
            b.age = -rand(0.03, 0.2);
            let shade = [0x3d3a36, 0x4a4641, 0x57524b][(rand(0., 2.99)) as usize];
            call(&get(&b.mat, "color"), "set", &[n(shade as f64)]);
        }
        for _ in 0..14 {
            let b = self.spawn(Kind::Spark);
            b.pos = add(at, [0., 0.3 * p, 0.], 1.);
            let a = rand(0., std::f64::consts::TAU);
            let out = rand(3., 8.) * p;
            b.vel = [a.cos() * out, rand(2., 7.) * p, a.sin() * out];
            b.size = [0.1, 0.1];
            b.life = rand(0.4, 0.9);
            b.floor = at[1];
        }
        for _ in 0..14 {
            let b = self.spawn(Kind::Grit);
            b.pos = add(at, [rand(-0.2, 0.2), 0.15, rand(-0.2, 0.2)], 1.);
            b.vel = [rand(-3.5, 3.5) * p, rand(3., 8.) * p, rand(-3.5, 3.5) * p];
            b.spin = [rand(-12., 12.), rand(-12., 12.), rand(-12., 12.)];
            b.shape = [rand(0.04, 0.13), rand(0.04, 0.1), rand(0.04, 0.13)];
            b.life = rand(1.6, 2.4);
            b.floor = at[1];
        }
    }

    /// A shell still in the air: it lands at `at` in `delay` seconds.
    pub fn blast_in(&mut self, delay: f64, at: [f64; 3], power: f64) {
        self.later.push((delay, at, power));
    }

    /// The ceiling gives way over `at`: dust trickles, then chunks of concrete drop and land
    /// `lead` seconds from now, each raising its own cloud.
    pub fn collapse(&mut self, at: [f64; 3], lead: f64) {
        let fall = 0.5 * G * lead * lead;
        for _ in 0..4 {
            let b = self.spawn(Kind::Smoke);
            b.pos = add(at, [rand(-1., 1.), fall + rand(0.3, 1.2), rand(-1., 1.)], 1.);
            b.vel = [0., -1.6, 0.];
            b.size = [0.5, 2.];
            b.alpha = 0.35;
            b.life = 1.6;
            call(&get(&b.mat, "color"), "set", &[n(0xb9b5ad as f64)]);
        }
        for i in 0..9 {
            let late = if i == 0 { 0. } else { rand(0., 0.3) };
            let b = self.spawn(Kind::Chunk);
            b.shape = [rand(0.2, 0.55), rand(0.14, 0.38), rand(0.18, 0.5)];
            b.pos = add(at, [rand(-0.9, 0.9), fall + b.shape[1] / 2., rand(-0.9, 0.9)], 1.);
            b.spin = [rand(-4., 4.), rand(-4., 4.), rand(-4., 4.)];
            b.life = rand(3.5, 4.5);
            b.age = -late;
            b.floor = at[1];
        }
        for _ in 0..10 {
            let b = self.spawn(Kind::Grit);
            b.shape = [rand(0.04, 0.1), rand(0.03, 0.08), rand(0.04, 0.1)];
            b.pos = add(at, [rand(-1.2, 1.2), fall + rand(0., 0.6), rand(-1.2, 1.2)], 1.);
            b.spin = [rand(-8., 8.), rand(-8., 8.), rand(-8., 8.)];
            b.life = rand(2.2, 3.);
            b.age = -rand(0., 0.4);
            b.floor = at[1];
        }
    }

    /// The kamikaze drone dives from high and far onto `to` and blows up there after `dur`.
    pub fn drone(&mut self, from: [f64; 3], to: [f64; 3], dur: f64, power: f64) {
        let mid = add(from, sub(to, from), 0.55);
        self.flight = Some(Flight {
            from,
            bend: add(mid, [0., 2.5, 0.], 1.),
            to,
            t: 0.,
            dur: dur.max(0.2),
            power,
        });
        put3(&get(&self.drone, "position"), from);
        set(&self.drone, "visible", &true.into());
    }

    pub fn take_flashes(&mut self) -> Vec<([f64; 3], bool)> {
        std::mem::take(&mut self.flashes)
    }

    pub fn update(&mut self, dt: f64) {
        self.time += dt;
        let goal = if self.mark_on { 1. } else { 0. };
        self.mark_k += (goal - self.mark_k) * (1. - (-dt * 8.).exp());
        let lit = self.mark_k > 0.01;
        for m in &self.marks {
            set(m, "visible", &lit.into());
        }
        set(
            &self.mark_mat,
            "opacity",
            &n(self.mark_k * (0.55 + 0.25 * (self.time * 5.5).sin())),
        );
        if self.bolt_age < 1. {
            self.bolt_age += dt;
            let k = super::lens::flicker(self.bolt_age);
            set(&self.bolt_mat, "opacity", &n(k));
            set(&self.glow_mat, "opacity", &n(k));
            call(
                &get(&self.glow, "scale"),
                "setScalar",
                &[n(2.5 + 2. * k)],
            );
            let on = self.bolt_age < 0.8;
            set(&self.bolt, "visible", &on.into());
            set(&self.glow, "visible", &on.into());
        }
        if self.light_age < 3. {
            self.light_age += dt;
            let k = if self.light_bolt {
                super::lens::flicker(self.light_age)
            } else {
                (-self.light_age * 7.).exp()
            };
            set(&self.light, "intensity", &n(self.light_peak * k));
        }
        for (left, at, power) in std::mem::take(&mut self.later) {
            if left - dt <= 0. {
                self.blast(at, power);
            } else {
                self.later.push((left - dt, at, power));
            }
        }
        if let Some(f) = &mut self.flight {
            f.t = (f.t + dt / f.dur).min(1.);
            let e = f.t.powf(1.6);
            let at = |e: f64| {
                let a = add(f.from, sub(f.bend, f.from), e);
                let b = add(f.bend, sub(f.to, f.bend), e);
                add(a, sub(b, a), e)
            };
            let here = at(e);
            let ahead = at((e + 0.02).min(1.));
            put3(&get(&self.drone, "position"), here);
            if length(sub(ahead, here)) > 1e-4 {
                call(&self.drone, "lookAt", &[make(&self.three, "Vector3", &[n(ahead[0]), n(ahead[1]), n(ahead[2])])]);
            }
            if f.t >= 1. {
                let (to, power) = (f.to, f.power);
                self.flight = None;
                set(&self.drone, "visible", &false.into());
                self.blast(to, power);
            }
        }
        let mut landings = vec![];
        for b in &mut self.bits {
            if b.age >= b.life {
                continue;
            }
            b.age += dt;
            if b.age < 0. || b.age >= b.life {
                set(&b.obj, "visible", &false.into());
                continue;
            }
            let t = b.age / b.life;
            match b.kind {
                Kind::Fire | Kind::Smoke => {
                    b.pos = add(b.pos, b.vel, dt);
                    let drag = if b.kind == Kind::Fire { 3. } else { 1.2 };
                    b.vel = b.vel.map(|v| v * (-dt * drag).exp());
                    let grow = 1. - (1. - t).powi(3);
                    let size = b.size[0] + (b.size[1] - b.size[0]) * grow;
                    call(&get(&b.obj, "scale"), "set", &[n(size), n(size), n(1.)]);
                    let fade = if b.kind == Kind::Fire {
                        (1. - t).powf(1.5)
                    } else {
                        (t / 0.08).min(1.) * (1. - t).powf(1.3)
                    };
                    set(&b.mat, "opacity", &n(b.alpha * fade));
                }
                Kind::Spark => {
                    b.vel[1] -= G * dt;
                    b.pos = add(b.pos, b.vel, dt);
                    if b.pos[1] < b.floor {
                        b.pos[1] = b.floor;
                        b.vel = [b.vel[0] * 0.5, -b.vel[1] * 0.3, b.vel[2] * 0.5];
                    }
                    call(&get(&b.obj, "scale"), "setScalar", &[n(b.size[0])]);
                    set(&b.mat, "opacity", &n(1. - t));
                }
                Kind::Grit | Kind::Chunk => {
                    b.vel[1] -= G * dt;
                    b.pos = add(b.pos, b.vel, dt);
                    b.turn = add(b.turn, b.spin, dt);
                    let rest = b.floor + b.shape[1] / 2.;
                    if b.pos[1] < rest {
                        b.pos[1] = rest;
                        if b.kind == Kind::Chunk && !b.landed {
                            landings.push(b.pos);
                        }
                        b.landed = true;
                        if b.vel[1] < -1.5 {
                            b.vel = [b.vel[0] * 0.6, -b.vel[1] * 0.25, b.vel[2] * 0.6];
                            b.spin = b.spin.map(|s| s * 0.5);
                        } else {
                            b.vel = [0.; 3];
                            b.spin = [0.; 3];
                        }
                    }
                    let k = ((1. - t) / 0.15).min(1.);
                    put3(&get(&b.obj, "scale"), b.shape.map(|s| s * k));
                    put3(&get(&b.obj, "rotation"), b.turn);
                }
            }
            put3(&get(&b.obj, "position"), b.pos);
            set(&b.obj, "visible", &true.into());
        }
        for at in landings {
            for _ in 0..2 {
                let b = self.spawn(Kind::Smoke);
                b.pos = add(at, [0., 0.1, 0.], 1.);
                b.vel = [rand(-1.5, 1.5), rand(0.2, 0.6), rand(-1.5, 1.5)];
                b.size = [0.6, rand(2.2, 3.2)];
                b.alpha = 0.5;
                b.life = rand(2., 3.);
                call(&get(&b.mat, "color"), "set", &[n(0xa9a49a as f64)]);
            }
        }
    }

    pub fn stop(&mut self) {
        for b in &mut self.bits {
            b.age = b.life;
            set(&b.obj, "visible", &false.into());
        }
        self.mark_on = false;
        self.mark_k = 0.;
        self.bolt_age = 99.;
        self.light_age = 99.;
        set(&self.light, "intensity", &n(0.));
        self.flight = None;
        self.later.clear();
        self.flashes.clear();
        for o in [&self.bolt, &self.glow, &self.drone, &self.marks[0], &self.marks[1]] {
            set(o, "visible", &false.into());
        }
    }

    pub fn dispose(&self) {
        call(&self.scene, "remove", &[self.root.clone()]);
        for r in &self.owned {
            if get(r, "dispose").is_function() {
                let _ = try_call(r, "dispose", &[]);
            }
        }
    }
}

impl Bit {
    fn new(obj: JsValue, mat: JsValue, kind: Kind) -> Self {
        if kind == Kind::Smoke {
            set(&mat, "rotation", &n(rand(0., std::f64::consts::TAU)));
        }
        Self {
            obj,
            mat,
            kind,
            pos: [0.; 3],
            vel: [0.; 3],
            spin: [0.; 3],
            turn: [0.; 3],
            shape: [0.1; 3],
            size: [1., 1.],
            alpha: 1.,
            age: 0.,
            life: 0.,
            floor: 0.,
            landed: false,
        }
    }
}

/// A jagged stroke from `a` to `b`: each pass splits every segment and pushes its middle aside.
fn jag(a: [f64; 3], b: [f64; 3], passes: u32, spread: f64) -> Vec<[f64; 3]> {
    let mut points = vec![a, b];
    for _ in 0..passes {
        let mut next = Vec::with_capacity(points.len() * 2);
        for pair in points.windows(2) {
            let (p, q) = (pair[0], pair[1]);
            let s = length(sub(q, p)) * spread;
            let mid = add(p, sub(q, p), 0.5);
            next.push(p);
            next.push(add(mid, [rand(-1., 1.) * s, rand(-0.3, 0.3) * s, rand(-1., 1.) * s], 1.));
        }
        next.push(*points.last().unwrap());
        points = next;
    }
    points
}
