//! The region screen: the oblast as a 3D vector map on the comms table. The map powers on
//! (the table lights, the block rises slice by slice, things grow out of the ground, the marks
//! drop in), then lives: water, current on the power line, trucks, a train, each region's
//! weather by its event, and shelling over the maps that are not open yet. Contours and a
//! faceted relief swap with a scan wipe. A tap picks a region; the card moves the base there.
use super::{base_region, bridge::object, dom_support::*, hud, platform};
use js_sys::{Float32Array, Object, Uint32Array};
use serde_json::Value;
use std::{
    cell::{OnceCell, RefCell},
    f64::consts::PI,
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, prelude::*};

const SHADERS: &str = include_str!("shaders/regionMap.glsl");

// ---------- the oblast ----------
const WATER: f64 = 0.07;
const DAM_Z: f64 = -0.36;
const FLOOR: f64 = -0.07;
type P = [f64; 2];
const UP: &[P] = &[[0.3, -0.36], [0.26, -0.58], [0.12, -0.8], [0.16, -1.04], [0.36, -1.22], [0.42, -1.7]];
const BAY: &[P] = &[[0.18, -0.86], [-0.08, -0.82], [-0.22, -0.7]];
const BAY2: &[P] = &[[0.3, -1.1], [0.55, -1.05], [0.7, -1.18]];
const RIVER: &[P] = &[[0.3, -0.36], [0.24, -0.12], [0.1, 0.14], [-0.1, 0.38], [-0.28, 0.6], [-0.36, 0.88], [-0.4, 1.18], [-0.48, 1.7]];
const RAVINE: &[P] = &[[-1.2, 0.36], [-0.86, 0.5], [-0.6, 0.57], [-0.3, 0.6]];
const ROAD: &[P] = &[[-0.16, 1.0], [-0.06, 0.74], [0.08, 0.46], [0.3, 0.12], [0.46, -0.16], [0.52, -0.34]];
const ROAD2: &[P] = &[[-0.56, 0.72], [-0.4, 0.82], [-0.16, 1.0], [-0.1, 1.4]];
const RAIL: &[P] = &[[1.0, 1.02], [0.6, 1.08], [0.32, 1.12], [0.0, 1.06], [-0.3, 0.86], [-0.56, 0.72], [-0.72, 0.56], [-1.0, 0.4]];
const LEP: &[P] = &[[0.56, -0.36], [0.42, 0.0], [0.14, 0.34], [-0.2, 0.56], [-0.56, 0.72]];
const TRICKLE: &[P] = &[[-0.28, 0.6], [-0.36, 0.88], [-0.4, 1.18], [-0.48, 1.5]];
const FLATS: [[f64; 4]; 5] = [[-0.56, 0.72, 0.14, 0.11], [-0.16, 1.0, 0.13, 0.1], [0.32, 1.12, 0.14, 0.1], [-0.78, 1.12, 0.14, 0.1], [-0.72, 0.52, 0.07, 0.11]];
/// Guns beyond the block; the nearest one shells the region's map under fire.
const GUNS: [P; 4] = [[1.6, -0.2], [1.5, 1.7], [-1.6, 1.6], [-1.5, -1.6]];
const CYAN: u32 = 0x73d3e2;
const FG: u32 = 0xece8df;
const AMBER: u32 = 0xe9ac3f;
const GLYPHS: &str = "АБВГДЕЖЗИКЛМНОПРСТУФХЦЧШЭЮЯ0123456789#/";

/// A place on the map: a catalogue map, drawn or still planned (`MAP_ACCESS`).
struct Spot {
    map: &'static str,
    at: P,
    left: bool,
}
struct Area {
    id: &'static str,
    north: bool,
    label: P,
    base: P,
    spots: [Spot; 4],
}
/// Каскад north of the border line, Белый Яр south of it.
const AREAS: [Area; 2] = [
    Area {
        id: "kaskad",
        north: true,
        label: [-0.5, -1.12],
        base: [0.47, -0.37],
        spots: [
            Spot { map: "dam", at: [0.24, -0.36], left: true },
            Spot { map: "dryBed", at: [0.14, 0.08], left: false },
            Spot { map: "intake", at: [-0.2, -0.7], left: true },
            Spot { map: "backwater", at: [0.62, -1.14], left: false },
        ],
    },
    Area {
        id: "belyYar",
        north: false,
        label: [0.56, 0.74],
        base: [-0.72, 0.52],
        spots: [
            Spot { map: "saltHall", at: [-0.56, 0.72], left: false },
            Spot { map: "blocks", at: [-0.16, 1.0], left: false },
            Spot { map: "railyard", at: [0.32, 1.12], left: false },
            Spot { map: "sludgePonds", at: [-0.78, 1.12], left: false },
        ],
    },
];

// ---------- height field ----------
fn hash(x: f64, y: f64) -> f64 {
    let h = (x * 127.1 + y * 311.7).sin() * 43758.5453;
    h - h.floor()
}
fn vnoise(x: f64, y: f64) -> f64 {
    let (xi, yi) = (x.floor(), y.floor());
    let (xf, yf) = (x - xi, y - yi);
    let (u, v) = (xf * xf * (3. - 2. * xf), yf * yf * (3. - 2. * yf));
    let (a, b, c, d) = (hash(xi, yi), hash(xi + 1., yi), hash(xi, yi + 1.), hash(xi + 1., yi + 1.));
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}
fn fbm(x: f64, y: f64, n: u32) -> f64 {
    let (mut s, mut a, mut f) = (0., 0.5, 1.);
    for _ in 0..n {
        s += a * vnoise(x * f, y * f);
        f *= 2.03;
        a *= 0.5;
    }
    s
}
fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
fn phase(t: f64, a: f64, b: f64) -> f64 {
    ((t - a) / (b - a)).clamp(0., 1.)
}
fn out_cubic(k: f64) -> f64 {
    1. - (1. - k).powi(3)
}
fn in_out_cubic(k: f64) -> f64 {
    if k < 0.5 { 4. * k * k * k } else { 1. - (-2. * k + 2.).powi(3) / 2. }
}
fn out_back(k: f64) -> f64 {
    1. + 2.70158 * (k - 1.).powi(3) + 1.70158 * (k - 1.).powi(2)
}
fn dist_poly(x: f64, z: f64, p: &[P]) -> f64 {
    p.windows(2).fold(1e9, |d, s| {
        let ([ax, az], [bx, bz]) = (s[0], s[1]);
        let (dx, dz) = (bx - ax, bz - az);
        let t = (((x - ax) * dx + (z - az) * dz) / (dx * dx + dz * dz)).clamp(0., 1.);
        d.min((x - ax - dx * t).hypot(z - az - dz * t))
    })
}
fn height(x: f64, z: f64) -> f64 {
    let mut h = 0.1 + 0.17 * fbm(x * 2.1 + 3.1, z * 2.1 + 7.7, 5) + 0.06 * x.abs() + 0.03 * smooth(-0.6, -1.6, z);
    h += 0.05 * fbm(x * 5.3 + 1.3, z * 5.3 - 2.1, 3).powi(2);
    let north = smooth(DAM_Z + 0.02, DAM_Z - 0.06, z);
    let wob = 0.06 * (fbm(x * 6. + 9., z * 6. - 4., 3) - 0.5);
    let up = dist_poly(x, z, UP).min(dist_poly(x, z, BAY) + 0.08).min(dist_poly(x, z, BAY2) + 0.1);
    h = mix(h, 0., (1. - smooth(0.1, 0.3, up + wob)) * north);
    let river = dist_poly(x, z, RIVER);
    h = mix(h, 0.02 + 0.01 * fbm(x * 30., z * 30., 2), (1. - smooth(0.025, 0.15, river + wob * 0.4)) * (1. - north));
    h = mix(h, 0.035, 1. - smooth(0.012, 0.07, dist_poly(x, z, RAVINE) + wob * 0.3));
    for [fx, fz, rx, ry] in FLATS {
        let d = ((x - fx) / rx).hypot((z - fz) / ry);
        h = mix(h, h.clamp(0.09, 0.14), 1. - smooth(0.6, 1.1, d));
    }
    h
}
fn saltiness(x: f64, z: f64) -> f64 {
    let s = 1. - smooth(0.02, 0.12, dist_poly(x, z, RAVINE));
    let s = s.max(1. - smooth(0.5, 1.1, ((x + 0.78) / 0.14).hypot((z - 1.12) / 0.1)));
    s.max(0.6 * (1. - smooth(0.3, 1., ((x + 0.56) / 0.15).hypot((z - 0.72) / 0.12))))
}
/// The ground, or the reservoir's water over it north of the dam.
fn ground_y(x: f64, z: f64) -> f64 {
    let h = height(x, z);
    if z < DAM_Z && h < WATER { WATER } else { h }
}
fn border(x: f64) -> f64 {
    0.3 + 0.18 * x
}
fn in_block(x: f64, z: f64) -> bool {
    x.abs() < 1. && z.abs() < 1.45
}
fn in_area(area: &Area, x: f64, z: f64) -> bool {
    in_block(x, z) && (z < border(x)) == area.north
}
fn circle(c: P, r: f64, n: usize) -> Vec<P> {
    (0..=n)
        .map(|i| {
            let a = i as f64 / n as f64 * PI * 2.;
            [c[0] + a.cos() * r, c[1] + a.sin() * r]
        })
        .collect()
}
/// A polyline laid on the ground, resampled every `step`.
fn drape(points: &[P], lift: f64, step: f64) -> Vec<f64> {
    let mut out = Vec::new();
    for s in points.windows(2) {
        let ([ax, az], [bx, bz]) = (s[0], s[1]);
        let n = ((bx - ax).hypot(bz - az) / step).ceil().max(1.) as usize;
        for k in 0..n {
            let t = k as f64 / n as f64;
            let (x, z) = (mix(ax, bx, t), mix(az, bz, t));
            out.extend([x, ground_y(x, z) + lift, z]);
        }
    }
    if let Some(&[x, z]) = points.last() {
        out.extend([x, ground_y(x, z) + lift, z]);
    }
    out
}

// ---------- terrain meshes, built once per session ----------
struct Terrain {
    contour_pos: Vec<f32>,
    contour_salt: Vec<f32>,
    contour_index: Vec<u32>,
    relief_pos: Vec<f32>,
    relief_salt: Vec<f32>,
    relief_bary: Vec<f32>,
    wall_pos: Vec<f32>,
    wall_ground: Vec<f32>,
    rim: Vec<f64>,
}
/// A grid over the block's ground, laid out and indexed like three's rotated PlaneGeometry.
fn grid(nx: usize, nz: usize, jitter: f64) -> (Vec<f32>, Vec<f32>, Vec<u32>) {
    let (mut pos, mut salt, mut index) = (Vec::new(), Vec::new(), Vec::new());
    for iz in 0..=nz {
        for ix in 0..=nx {
            let i = (iz * (nx + 1) + ix) as f64;
            let mut x = -1.35 + ix as f64 * 2.7 / nx as f64;
            let mut z = -1.85 + iz as f64 * 3.7 / nz as f64;
            if jitter > 0. {
                x += (hash(i, 1.) - 0.5) * jitter;
                z += (hash(i, 2.) - 0.5) * jitter;
            }
            pos.extend([x as f32, height(x, z) as f32, z as f32]);
            salt.push(saltiness(x, z) as f32);
        }
    }
    let row = (nx + 1) as u32;
    for iz in 0..nz as u32 {
        for ix in 0..nx as u32 {
            let (a, b) = (ix + row * iz, ix + row * (iz + 1));
            let (c, d) = (ix + 1 + row * (iz + 1), ix + 1 + row * iz);
            index.extend([a, b, d, b, c, d]);
        }
    }
    (pos, salt, index)
}
impl Terrain {
    fn build() -> Self {
        let (contour_pos, contour_salt, contour_index) = grid(300, 410, 0.);
        // The relief is faceted: a jittered coarse grid, every triangle with its own corners.
        let (pos, salt, index) = grid(96, 132, 0.016);
        let mut relief_pos = Vec::with_capacity(index.len() * 3);
        let mut relief_salt = Vec::with_capacity(index.len());
        let mut relief_bary = vec![0f32; index.len() * 3];
        for (k, &i) in index.iter().enumerate() {
            let i = i as usize;
            relief_pos.extend_from_slice(&pos[i * 3..i * 3 + 3]);
            relief_salt.push(salt[i]);
            relief_bary[k * 3 + k % 3] = 1.;
        }
        // The block's cut sides: strata, and the reservoir's water in the north cut.
        let (mut wall_pos, mut wall_ground, mut rim) = (Vec::new(), Vec::new(), Vec::new());
        let sides: [(P, P); 4] = [([-1., 1.45], [1., 1.45]), ([1., 1.45], [1., -1.45]), ([1., -1.45], [-1., -1.45]), ([-1., -1.45], [-1., 1.45])];
        for ([ax, az], [bx, bz]) in sides {
            let n = ((bx - ax).hypot(bz - az) / 0.006).ceil() as usize;
            for k in 0..n {
                let (k0, k1) = (k as f64 / n as f64, (k + 1) as f64 / n as f64);
                let (x0, z0, x1, z1) = (mix(ax, bx, k0), mix(az, bz, k0), mix(ax, bx, k1), mix(az, bz, k1));
                let (t0, t1, g0, g1) = (ground_y(x0, z0), ground_y(x1, z1), height(x0, z0), height(x1, z1));
                for v in [x0, FLOOR, z0, x1, FLOOR, z1, x1, t1, z1, x0, FLOOR, z0, x1, t1, z1, x0, t0, z0] {
                    wall_pos.push(v as f32);
                }
                for v in [g0, g1, g1, g0, g1, g0] {
                    wall_ground.push(v as f32);
                }
                rim.extend([x0, t0 + 0.001, z0]);
            }
        }
        rim.extend([rim[0], rim[1], rim[2]]);
        Self { contour_pos, contour_salt, contour_index, relief_pos, relief_salt, relief_bary, wall_pos, wall_ground, rim }
    }
}
thread_local! {
    static TERRAIN: OnceCell<Rc<Terrain>> = const { OnceCell::new() };
}
fn terrain() -> Rc<Terrain> {
    TERRAIN.with(|cell| cell.get_or_init(|| Rc::new(Terrain::build())).clone())
}

// ---------- shaders: one file, split at its «//# name» lines ----------
fn section(name: &str) -> &'static str {
    let mark = format!("//# {name}");
    let (mut offset, mut start) = (0, None);
    for line in SHADERS.split_inclusive('\n') {
        let bare = line.trim_end();
        match start {
            Some(from) if bare.starts_with("//# ") => return &SHADERS[from..offset],
            None if bare == mark => start = Some(offset + line.len()),
            _ => {}
        }
        offset += line.len();
    }
    start.map_or("", |from| &SHADERS[from..])
}
fn lib(name: &str) -> String {
    format!("{}{}", section("lib"), section(name))
}
fn ground(name: &str) -> String {
    format!("{}{}{}", section("lib"), section("terrain"), section(name))
}

// ---------- small JS helpers ----------
fn n(value: f64) -> JsValue {
    number(value)
}
fn uniform(value: JsValue) -> JsValue {
    object(&[("value", value)])
}
fn put(holder: &JsValue, value: f64) {
    set(holder, "value", &n(value));
}
fn put3(object: &JsValue, key: &str, p: [f64; 3]) {
    call(&get(object, key), "set", &[n(p[0]), n(p[1]), n(p[2])]);
}
fn f32s(values: &[f64]) -> JsValue {
    let values: Vec<f32> = values.iter().map(|&v| v as f32).collect();
    Float32Array::from(&values[..]).into()
}
fn random() -> f64 {
    js_sys::Math::random()
}

// ---------- the camera, in plain math so labels and taps need no round trip ----------
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt().max(1e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}
const LOOK: [f64; 3] = [0., 0.05, 0.];
/// A perspective camera at `dist` from the look point along `dir`, looking at it.
#[derive(Clone, Copy)]
struct Pose {
    pos: [f64; 3],
    f: [f64; 3],
    r: [f64; 3],
    u: [f64; 3],
    tan: f64,
    aspect: f64,
}
impl Pose {
    fn new(dir: [f64; 3], dist: f64, aspect: f64) -> Self {
        let pos = [LOOK[0] + dir[0] * dist, LOOK[1] + dir[1] * dist, LOOK[2] + dir[2] * dist];
        let f = [-dir[0], -dir[1], -dir[2]];
        let r = unit(cross(f, [0., 1., 0.]));
        Self { pos, f, r, u: cross(r, f), tan: (18f64).to_radians().tan(), aspect }
    }
    /// Normalised device coordinates of a world point; none behind the camera.
    fn ndc(&self, p: [f64; 3]) -> Option<(f64, f64)> {
        let d = sub(p, self.pos);
        let depth = dot(d, self.f);
        (depth > 1e-6).then(|| (dot(d, self.r) / depth / (self.tan * self.aspect), dot(d, self.u) / depth / self.tan))
    }
    /// Where the ray through a device point meets the plane `y`.
    fn hit(&self, nx: f64, ny: f64, y: f64) -> Option<(f64, f64)> {
        let (a, b) = (nx * self.tan * self.aspect, ny * self.tan);
        let d = [self.f[0] + self.r[0] * a + self.u[0] * b, self.f[1] + self.r[1] * a + self.u[1] * b, self.f[2] + self.r[2] * a + self.u[2] * b];
        if d[1] >= -1e-6 {
            return None;
        }
        let s = (y - self.pos[1]) / d[1];
        (s > 0.).then(|| (self.pos[0] + d[0] * s, self.pos[2] + d[2] * s))
    }
}

// ---------- the scene ----------
#[derive(Clone)]
struct Line {
    obj: JsValue,
    mat: JsValue,
    geo: JsValue,
    base: f64,
    at: f64,
    grow: bool,
    done: bool,
}
#[derive(Clone, Copy)]
struct Pen {
    color: u32,
    width: f64,
    dash: Option<(f64, f64)>,
    opacity: f64,
    order: f64,
    at: Option<f64>,
    grow: bool,
    boot: bool,
}
impl Default for Pen {
    fn default() -> Self {
        Self { color: FG, width: 1.5, dash: None, opacity: 1., order: 3., at: None, grow: false, boot: true }
    }
}
struct Glow {
    sprite: JsValue,
    mat: JsValue,
}
impl Glow {
    fn opacity(&self, value: f64) {
        set(&self.mat, "opacity", &n(value));
    }
    fn at(&self, p: [f64; 3]) {
        put3(&self.sprite, "position", p);
    }
    fn size(&self, x: f64, y: f64) {
        call(&get(&self.sprite, "scale"), "set", &[n(x), n(y), n(1.)]);
    }
}
struct Build {
    group: JsValue,
    at: f64,
    done: bool,
}
struct Path {
    pts: Vec<[f64; 3]>,
    len: Vec<f64>,
    total: f64,
}
impl Path {
    fn new(points: &[P]) -> Self {
        let pts: Vec<[f64; 3]> = points.iter().map(|&[x, z]| [x, ground_y(x, z) + 0.008, z]).collect();
        let mut len = vec![0.];
        for w in pts.windows(2) {
            let d = sub(w[1], w[0]);
            len.push(len[len.len() - 1] + dot(d, d).sqrt());
        }
        let total = len[len.len() - 1];
        Self { pts, len, total }
    }
    fn along(&self, d: f64) -> [f64; 3] {
        let d = d.clamp(0., self.total);
        let mut i = 1;
        while i < self.len.len() - 1 && self.len[i] < d {
            i += 1;
        }
        let span = self.len[i] - self.len[i - 1];
        let k = (d - self.len[i - 1]) / if span > 0. { span } else { 1. };
        let (a, b) = (self.pts[i - 1], self.pts[i]);
        let (x, z) = (mix(a[0], b[0], k), mix(a[2], b[2], k));
        [x, ground_y(x, z) + 0.008, z]
    }
}
struct Truck {
    path: usize,
    speed: f64,
    d: f64,
    glow: Glow,
}
struct Cloud {
    glow: Glow,
    x: f64,
    z: f64,
    y: f64,
    size: f64,
    speed: f64,
    north: bool,
}
struct Strike {
    bolt: Line,
    twig: Line,
    x: f64,
    z: f64,
    age: f64,
}
struct Shell {
    pts: Vec<[f64; 3]>,
    head: Glow,
    t: f64,
    speed: f64,
    at: P,
}
struct Smoke {
    glow: Glow,
    base: f64,
    phase: f64,
}
struct Blast {
    flash: Glow,
    ring: Line,
    age: f64,
}
/// What a frame hands the scene.
struct Step {
    t: f64,
    dt: f64,
    live: bool,
    boot: f64,
    cut: f64,
    sel: f64,
    wipe: f64,
    scan: f64,
    selected: usize,
    ping: [f64; 3],
}

/// Everything WebGL. Without a WebGL context the map has none, and the labels and the card
/// still work over the dark table.
struct Gl {
    host: JsValue,
    three: JsValue,
    renderer: JsValue,
    scene: JsValue,
    camera: JsValue,
    world: JsValue,
    uniforms: JsValue,
    fog: JsValue,
    ping: JsValue,
    flash: JsValue,
    water_alpha: JsValue,
    motes_alpha: JsValue,
    weather_alpha: Vec<JsValue>,
    res: Vec<JsValue>,
    solids: Vec<(u32, JsValue)>,
    edges: Vec<(u64, JsValue)>,
    lines: Vec<Line>,
    count: usize,
    size: (f64, f64),
    builds: Vec<Build>,
    flows: Vec<(JsValue, f64)>,
    tex: [JsValue; 4],
    lamps: Vec<(Glow, f64)>,
    puffs: Vec<(Glow, f64)>,
    chimney: [f64; 3],
    paths: Vec<Path>,
    trucks: Vec<Truck>,
    train: Vec<JsValue>,
    train_light: Option<Glow>,
    train_d: f64,
    pulses: Vec<(Line, f64, usize)>,
    radars: Vec<(JsValue, usize, f64)>,
    clouds: Vec<Cloud>,
    strikes: Vec<Strike>,
    stormy: Vec<usize>,
    next_strike: f64,
    flash_w: f64,
    shells: Vec<Shell>,
    smokes: Vec<Smoke>,
    blasts: Vec<Blast>,
    next_blast: f64,
}
const FLASH: usize = 0;
const SMOKE: usize = 1;
const LIGHT: usize = 2;
const CLOUD: usize = 3;

fn event_index(event: &str) -> f64 {
    match event {
        "storms" => 1.,
        "dryWind" => 2.,
        "chemical" => 3.,
        _ => 0.,
    }
}

impl Gl {
    fn new(host: &JsValue, canvas: &JsValue, events: &[String]) -> Result<Self, JsValue> {
        let three = get(host, "THREE");
        let renderer = construct(
            &three,
            "WebGLRenderer",
            &[object(&[("canvas", canvas.clone()), ("antialias", flag(true)), ("alpha", flag(true))])],
        )?;
        let dpr = num(&global(), "devicePixelRatio").max(1.);
        call(&renderer, "setPixelRatio", &[n(dpr.min(2.))]);
        call(&renderer, "setClearColor", &[n(0.), n(0.)]);
        let scene = construct(&three, "Scene", &[])?;
        let camera = construct(&three, "PerspectiveCamera", &[n(36.), n(1.), n(0.05), n(50.)])?;
        let world = construct(&three, "Group", &[])?;
        set(&get(&world, "scale"), "y", &n(1.7));
        call(&scene, "add", &[world.clone()]);
        let vector = |name: &str, values: &[f64]| {
            let values: Vec<JsValue> = values.iter().map(|&v| n(v)).collect();
            construct(&three, name, &values).unwrap_or(JsValue::UNDEFINED)
        };
        let (fog, ping, flash) = (vector("Vector2", &[3., 6.]), vector("Vector3", &[0., 0., -99.]), vector("Vector4", &[0., 0., 0.3, 0.]));
        let events = vector("Vector2", &[event_index(&events[0]), event_index(&events[1])]);
        // One set of uniforms shared by every map shader.
        let uniforms = object(&[
            ("uTime", uniform(n(0.))),
            ("uSel", uniform(n(1.))),
            ("uWipe", uniform(n(0.))),
            ("uScan", uniform(n(0.))),
            ("uCut", uniform(n(FLOOR))),
            ("uBoot", uniform(n(0.))),
            ("uFog", uniform(fog.clone())),
            ("uPing", uniform(ping.clone())),
            ("uFlash", uniform(flash.clone())),
            ("uEvents", uniform(events)),
            ("uPx", uniform(n(400.))),
        ]);
        Ok(Self {
            host: host.clone(),
            three,
            renderer,
            scene,
            camera,
            world,
            uniforms,
            fog,
            ping,
            flash,
            water_alpha: JsValue::UNDEFINED,
            motes_alpha: JsValue::UNDEFINED,
            weather_alpha: Vec::new(),
            res: Vec::new(),
            solids: Vec::new(),
            edges: Vec::new(),
            lines: Vec::new(),
            count: 0,
            size: (1., 1.),
            builds: Vec::new(),
            flows: Vec::new(),
            tex: Default::default(),
            lamps: Vec::new(),
            puffs: Vec::new(),
            chimney: [0.; 3],
            paths: Vec::new(),
            trucks: Vec::new(),
            train: Vec::new(),
            train_light: None,
            train_d: 0.4,
            pulses: Vec::new(),
            radars: Vec::new(),
            clouds: Vec::new(),
            strikes: Vec::new(),
            stormy: Vec::new(),
            next_strike: 1.,
            flash_w: 0.,
            shells: Vec::new(),
            smokes: Vec::new(),
            blasts: Vec::new(),
            next_blast: 0.3,
        })
    }
    fn make(&self, name: &str, values: &[JsValue]) -> JsValue {
        construct(&self.three, name, values).unwrap_or_else(|error| {
            call(&get(&global(), "console"), "error", &[js(&format!("crowned-dogs: region map `{name}` failed")), error]);
            JsValue::UNDEFINED
        })
    }
    /// Keeps a geometry, material or texture for the final dispose.
    fn keep(&mut self, value: JsValue) -> JsValue {
        self.res.push(value.clone());
        value
    }
    fn add(&self, object: &JsValue) {
        call(&self.world, "add", &[object.clone()]);
    }
    fn shader(&mut self, vertex: &str, fragment: &str, extra: &[(&str, JsValue)], options: &[(&str, JsValue)]) -> JsValue {
        let uniforms = Object::assign(&Object::new(), self.uniforms.unchecked_ref());
        for (key, value) in extra {
            set(&uniforms, key, &uniform(value.clone()));
        }
        let params = object(options);
        set(&params, "uniforms", &uniforms);
        set(&params, "vertexShader", &js(vertex));
        set(&params, "fragmentShader", &js(fragment));
        let material = self.make("ShaderMaterial", &[params]);
        self.keep(material)
    }
    fn geometry(&mut self, attributes: &[(&str, &[f32], u32)]) -> JsValue {
        let geometry = self.make("BufferGeometry", &[]);
        for &(name, values, size) in attributes {
            let attribute = self.make("BufferAttribute", &[Float32Array::from(values).into(), n(size as f64)]);
            call(&geometry, "setAttribute", &[js(name), attribute]);
        }
        self.keep(geometry)
    }
    fn mesh(&self, geometry: &JsValue, material: &JsValue, order: f64) -> JsValue {
        let mesh = self.make("Mesh", &[geometry.clone(), material.clone()]);
        set(&mesh, "renderOrder", &n(order));
        self.add(&mesh);
        mesh
    }
    fn plane(&mut self, w: f64, d: f64, y: f64, z: f64) -> JsValue {
        let geometry = self.make("PlaneGeometry", &[n(w), n(d), n(1.), n(1.)]);
        call(&geometry, "rotateX", &[n(-PI / 2.)]);
        call(&geometry, "translate", &[n(0.), n(y), n(z)]);
        self.keep(geometry)
    }
    fn uniform_of(material: &JsValue, key: &str) -> JsValue {
        get(&get(material, "uniforms"), key)
    }

    // ---------- vector lines ----------
    fn draw(&mut self, points: &[f64], pen: Pen) -> Line {
        let geo = construct(&self.host, "LineGeometry", &[]).unwrap_or(JsValue::UNDEFINED);
        call(&geo, "setPositions", &[f32s(points)]);
        let (dash, gap) = pen.dash.unwrap_or((1., 0.));
        let mat = construct(
            &self.host,
            "LineMaterial",
            &[object(&[
                ("color", n(pen.color as f64)),
                ("linewidth", n(pen.width)),
                ("dashed", flag(pen.dash.is_some())),
                ("dashSize", n(dash)),
                ("gapSize", n(gap)),
                ("transparent", flag(true)),
                ("opacity", n(if pen.boot { 0. } else { pen.opacity })),
                ("worldUnits", flag(false)),
            ])],
        )
        .unwrap_or(JsValue::UNDEFINED);
        call(&get(&mat, "resolution"), "set", &[n(self.size.0), n(self.size.1)]);
        let obj = construct(&self.host, "Line2", &[geo.clone(), mat.clone()]).unwrap_or(JsValue::UNDEFINED);
        if pen.dash.is_some() {
            call(&obj, "computeLineDistances", &[]);
        }
        set(&obj, "renderOrder", &n(pen.order));
        self.add(&obj);
        let at = pen.at.unwrap_or_else(|| {
            self.count += 1;
            1.35 + ((self.count - 1) % 9) as f64 * 0.05
        });
        Line { obj, mat, geo, base: pen.opacity, at, grow: pen.grow, done: !pen.boot }
    }
    /// A line that stays for the life of the map.
    fn line(&mut self, points: &[f64], pen: Pen) -> Line {
        let line = self.draw(points, pen);
        self.lines.push(line.clone());
        line
    }
    /// A ring lying on the ground, centred on its own origin so it can be scaled.
    fn ring_points(c: P, r: f64) -> Vec<f64> {
        let mut pts = drape(&circle(c, r, 48), 0.008, 0.01);
        for p in pts.chunks_mut(3) {
            p[0] -= c[0];
            p[2] -= c[1];
        }
        pts
    }
    fn ring(&mut self, c: P, r: f64, color: u32, opacity: f64, pen: Pen) -> Line {
        let line = self.line(&Self::ring_points(c, r), Pen { color, width: 1.4, opacity, ..pen });
        put3(&line.obj, "position", [c[0], 0., c[1]]);
        line
    }
    fn drop_line(&self, line: &Line) {
        call(&self.world, "remove", &[line.obj.clone()]);
        call(&line.geo, "dispose", &[]);
        call(&line.mat, "dispose", &[]);
    }
    /// A stem standing up from the ground to a mark; it grows out with the boot.
    fn stem(&mut self, c: P, h: f64, color: u32, opacity: f64, at: f64) {
        let line = self.line(&[0., 0., 0., 0., h, 0.], Pen { color, width: 1.2, opacity, order: 5., at: Some(at), grow: true, ..Pen::default() });
        put3(&line.obj, "position", [c[0], ground_y(c[0], c[1]), c[1]]);
    }

    // ---------- built things: they grow out of the ground after the block rises ----------
    fn solid(&mut self, color: u32) -> JsValue {
        if let Some((_, m)) = self.solids.iter().find(|(c, _)| *c == color) {
            return m.clone();
        }
        let tint = self.make("Color", &[n(color as f64)]);
        let m = self.shader(section("solid-vertex"), &lib("solid"), &[("uColor", tint)], &[]);
        self.solids.push((color, m.clone()));
        m
    }
    fn edge(&mut self, opacity: f64) -> JsValue {
        if let Some((_, m)) = self.edges.iter().find(|(o, _)| *o == opacity.to_bits()) {
            return m.clone();
        }
        let m = self.shader(section("edge-vertex"), &lib("edge"), &[("uOpacity", n(opacity))], &[("transparent", flag(true))]);
        self.edges.push((opacity.to_bits(), m.clone()));
        m
    }
    fn boxed(&mut self, w: f64, h: f64, d: f64) -> JsValue {
        let geometry = self.make("BoxGeometry", &[n(w), n(h), n(d)]);
        call(&geometry, "translate", &[n(0.), n(h / 2.), n(0.)]);
        self.keep(geometry)
    }
    /// A solid with lit edges in a group of its own.
    fn shape(&mut self, geometry: &JsValue, color: u32, edge: f64) -> JsValue {
        let group = self.make("Group", &[]);
        let solid = self.solid(color);
        let edge = self.edge(edge);
        let edges = self.make("EdgesGeometry", &[geometry.clone()]);
        let edges = self.keep(edges);
        call(&group, "add", &[self.make("Mesh", &[geometry.clone(), solid])]);
        call(&group, "add", &[self.make("LineSegments", &[edges, edge])]);
        group
    }
    #[allow(clippy::too_many_arguments)]
    fn build(&mut self, geometry: JsValue, x: f64, z: f64, y: Option<f64>, rot: f64, color: u32, edge: f64) {
        let group = self.shape(&geometry, color, edge);
        put3(&group, "position", [x, y.unwrap_or_else(|| ground_y(x, z)), z]);
        set(&get(&group, "rotation"), "y", &n(rot));
        set(&get(&group, "scale"), "y", &n(0.001));
        set(&group, "visible", &flag(false));
        self.add(&group);
        self.builds.push(Build { group, at: 1.45 + x.hypot(z * 0.7) * 0.32, done: false });
    }
    #[allow(clippy::too_many_arguments)]
    fn block(&mut self, x: f64, z: f64, w: f64, d: f64, h: f64, rot: f64, color: u32, edge: f64) {
        let geometry = self.boxed(w, h, d);
        self.build(geometry, x, z, None, rot, color, edge);
    }

    // ---------- glows: lights, smoke, flashes ----------
    fn glow_texture(&mut self, stops: &[(f64, &str)]) -> JsValue {
        let canvas = create("canvas", "");
        set(&canvas, "width", &n(64.));
        set(&canvas, "height", &n(64.));
        let g = call(&canvas, "getContext", &[js("2d")]);
        let gradient = call(&g, "createRadialGradient", &[n(32.), n(32.), n(0.), n(32.), n(32.), n(32.)]);
        for &(at, color) in stops {
            call(&gradient, "addColorStop", &[n(at), js(color)]);
        }
        set(&g, "fillStyle", &gradient);
        call(&g, "fillRect", &[n(0.), n(0.), n(64.), n(64.)]);
        let texture = self.make("CanvasTexture", &[canvas]);
        self.keep(texture)
    }
    fn sprite(&mut self, texture: usize, color: u32, additive: bool, keep: bool) -> Glow {
        let blending = get(&self.three, if additive { "AdditiveBlending" } else { "NormalBlending" });
        let mat = self.make(
            "SpriteMaterial",
            &[object(&[
                ("map", self.tex[texture].clone()),
                ("color", n(color as f64)),
                ("transparent", flag(true)),
                ("depthWrite", flag(false)),
                ("blending", blending),
                ("opacity", n(0.)),
            ])],
        );
        if keep {
            self.keep(mat.clone());
        }
        let sprite = self.make("Sprite", &[mat.clone()]);
        self.add(&sprite);
        Glow { sprite, mat }
    }

    /// Points scattered over one region, with a seed each.
    fn region_points(&mut self, area: &Area, count: usize, salt: f64) -> JsValue {
        let (mut pos, mut seed) = (vec![0.; count * 3], vec![0.; count]);
        let (mut i, mut k) = (0, 0);
        while i < count && k < count * 50 {
            let x = (hash(k as f64, salt) - 0.5) * 2.;
            let z = (hash(k as f64, salt + 1.) - 0.5) * 2.9;
            k += 1;
            if !in_area(area, x, z) {
                continue;
            }
            pos[i * 3] = x;
            pos[i * 3 + 2] = z;
            seed[i] = hash(k as f64, salt + 2.);
            i += 1;
        }
        let (pos, seed): (Vec<f32>, Vec<f32>) = (pos.iter().map(|&v| v as f32).collect(), seed.iter().map(|&v| v as f32).collect());
        self.geometry(&[("position", &pos, 3), ("seed", &seed, 1)])
    }
    fn points(&mut self, geometry: &JsValue, vertex: &str, fragment: &str, extra: &[(&str, JsValue)], additive: bool, order: f64) -> JsValue {
        let mut options = vec![("transparent", flag(true)), ("depthWrite", flag(false))];
        if additive {
            options.push(("blending", get(&self.three, "AdditiveBlending")));
        }
        let mut extra = extra.to_vec();
        extra.push(("uAlpha", n(0.)));
        let material = self.shader(vertex, fragment, &extra, &options);
        let points = self.make("Points", &[geometry.clone(), material.clone()]);
        set(&points, "renderOrder", &n(order));
        set(&points, "frustumCulled", &flag(false));
        self.add(&points);
        Self::uniform_of(&material, "uAlpha")
    }

    fn build_scene(&mut self, views: &[AreaView]) {
        let events: Vec<String> = views.iter().map(|v| v.event.clone()).collect();
        let ter = terrain();
        // The maps under fire, one per region at most.
        let shelled: Vec<P> = AREAS
            .iter()
            .zip(views)
            .filter_map(|(a, v)| v.shelled.map(|i| a.spots[i].at))
            .collect();
        // Terrain: a fine smooth mesh for the contours, a faceted one for the relief.
        // The shader takes four scorched zones; the spare ones sit off the block.
        let zones = js_sys::Array::new();
        for k in 0..4 {
            let [x, z, r] = shelled.get(k).map_or([9., 9., 0.01], |p| [p[0], p[1], 0.085]);
            zones.push(&self.make("Vector3", &[n(x), n(z), n(r)]));
        }
        let zones: JsValue = zones.into();
        let contour = self.geometry(&[("position", &ter.contour_pos, 3), ("salt", &ter.contour_salt, 1)]);
        let index = self.make("BufferAttribute", &[Uint32Array::from(&ter.contour_index[..]).into(), n(1.)]);
        call(&contour, "setIndex", &[index]);
        let m = self.shader(section("terrain-vertex"), &ground("contour"), &[("uZones", zones.clone())], &[]);
        self.mesh(&contour, &m, 0.);
        let relief = self.geometry(&[("position", &ter.relief_pos, 3), ("salt", &ter.relief_salt, 1), ("bary", &ter.relief_bary, 3)]);
        let m = self.shader(section("terrain-vertex"), &ground("relief"), &[("uZones", zones)], &[("defines", object(&[("BARY", js(""))]))]);
        self.mesh(&relief, &m, 0.);
        let walls = self.geometry(&[("position", &ter.wall_pos, 3), ("ground", &ter.wall_ground, 1)]);
        let m = self.shader(section("wall-vertex"), &lib("wall"), &[], &[("side", get(&self.three, "DoubleSide"))]);
        self.mesh(&walls, &m, 0.);
        // The table under the block: a faint grid lit by the power-on ring.
        let table = self.plane(12., 12., FLOOR, 0.);
        let m = self.shader(section("flat-vertex"), &lib("table"), &[], &[("transparent", flag(true)), ("depthWrite", flag(false))]);
        self.mesh(&table, &m, -1.);
        // The reservoir north of the dam.
        let water = self.plane(2.2, 1.6, WATER, DAM_Z - 0.8);
        let m = self.shader(section("flat-vertex"), &lib("water"), &[("uAlpha", n(0.))], &[("transparent", flag(true)), ("depthWrite", flag(false))]);
        self.water_alpha = Self::uniform_of(&m, "uAlpha");
        self.mesh(&water, &m, 2.);

        // The rim of the block and the region border.
        self.line(&ter.rim, Pen { width: 1.3, opacity: 0.55, at: Some(1.25), ..Pen::default() });
        let edge: Vec<P> = (0..=100)
            .map(|i| {
                let x = -1. + i as f64 * 0.02;
                [x, border(x) + 0.03 * (fbm(x * 7., 1.1, 3) - 0.5)]
            })
            .collect();
        self.line(&drape(&edge, 0.012, 0.02), Pen { width: 1.4, dash: Some((0.02, 0.016)), opacity: 0.55, at: Some(1.5), ..Pen::default() });
        // The dry river bed below the dam with a thin living trickle, the roads, the rail.
        self.line(&drape(&RIVER[..7], 0.004, 0.02), Pen { color: 0xb5a77e, width: 1.3, dash: Some((0.012, 0.01)), opacity: 0.85, ..Pen::default() });
        let trickle = drape(TRICKLE, 0.005, 0.02);
        self.line(&trickle, Pen { color: CYAN, width: 1., opacity: 0.35, ..Pen::default() });
        let flow = self.line(&trickle, Pen { color: 0x9fe6f0, width: 1.6, dash: Some((0.02, 0.09)), opacity: 0.7, order: 4., ..Pen::default() });
        self.flows.push((flow.mat, 0.05));
        self.line(&drape(ROAD, 0.006, 0.02), Pen { width: 1.4, opacity: 0.6, ..Pen::default() });
        self.line(&drape(ROAD2, 0.006, 0.02), Pen { width: 1.4, opacity: 0.6, ..Pen::default() });
        self.line(&drape(&[[-0.16, 1.], [-0.5, 1.08], [-0.78, 1.12], [-1., 1.2]], 0.006, 0.02), Pen { width: 1., opacity: 0.35, ..Pen::default() });
        self.line(&drape(RAIL, 0.007, 0.02), Pen { width: 2.6, opacity: 0.5, ..Pen::default() });
        self.line(&drape(RAIL, 0.009, 0.02), Pen { color: 0x0a0b0b, width: 1.2, dash: Some((0.018, 0.018)), opacity: 0.9, order: 4., ..Pen::default() });

        // The power line from the dam to the salt plant: towers, wires, current running down them.
        let mut towers = Vec::new();
        for s in LEP.windows(2) {
            let ([ax, az], [bx, bz]) = (s[0], s[1]);
            let count = ((bx - ax).hypot(bz - az) / 0.09).ceil() as usize;
            for k in 0..count {
                let k = k as f64 / count as f64;
                towers.push([mix(ax, bx, k), mix(az, bz, k)]);
            }
        }
        towers.push(LEP[LEP.len() - 1]);
        let mut wire = Vec::new();
        for s in towers.windows(2) {
            let ([ax, az], [bx, bz]) = (s[0], s[1]);
            let (ya, yb) = (ground_y(ax, az) + 0.05, ground_y(bx, bz) + 0.05);
            for k in 0..=8 {
                let t = k as f64 / 8.;
                wire.extend([mix(ax, bx, t), mix(ya, yb, t) - (t * PI).sin() * 0.008, mix(az, bz, t)]);
            }
        }
        self.line(&wire, Pen { color: CYAN, width: 1., opacity: 0.45, ..Pen::default() });
        let current = self.line(&wire, Pen { color: 0xc8f4fa, width: 2., dash: Some((0.012, 0.16)), opacity: 0.9, order: 4., at: Some(2.1), ..Pen::default() });
        self.flows.push((current.mat, 0.12));
        let mut lamps: Vec<([f64; 3], f64, f64)> = Vec::new();
        for (i, &[x, z]) in towers.iter().enumerate() {
            let y = ground_y(x, z);
            self.line(&[x - 0.01, y, z, x, y + 0.05, z, x + 0.01, y, z], Pen { color: CYAN, width: 1., opacity: 0.55, ..Pen::default() });
            if i % 2 == 0 {
                lamps.push(([x, y + 0.053, z], i as f64 * 0.13, 0.022));
            }
        }

        // The dam: find the wet span on its line and close it with a wall.
        let (mut a, mut b) = (0.3, 0.3);
        while a > -0.9 && height(a, DAM_Z - 0.015) < WATER + 0.01 {
            a -= 0.005;
        }
        while b < 0.9 && height(b, DAM_Z - 0.015) < WATER + 0.01 {
            b += 0.005;
        }
        let (a, b) = (a - 0.03, b + 0.03);
        let top = WATER + 0.035;
        let wall = self.boxed(b - a, top, 0.035);
        self.build(wall, (a + b) / 2., DAM_Z + 0.005, Some(0.), 0., 0x9b9a92, 0.9);
        for i in 0..6 {
            let block = self.boxed(0.014, 0.016, 0.024);
            self.build(block, mix(a + 0.06, b - 0.06, i as f64 / 5.), DAM_Z + 0.005, Some(top), 0., 0xb4b2a8, 0.6);
        }
        let mid = (a + b) / 2.;
        for i in 0..3 {
            let i = i as f64;
            let intake = [[mid - 0.04 + i * 0.04, DAM_Z + 0.03], [mid - 0.05 + i * 0.05, DAM_Z + 0.12]];
            self.line(&drape(&intake, 0.004, 0.02), Pen { color: CYAN, width: 1., opacity: 0.4, ..Pen::default() });
        }
        lamps.push(([a + 0.01, top + 0.004, DAM_Z + 0.005], 0., 0.024));
        lamps.push(([b - 0.01, top + 0.004, DAM_Z + 0.005], 0.5, 0.024));
        lamps.push(([-0.72, ground_y(-0.72, 0.52) + 0.078, 0.52], 0.3, 0.026));
        // The salt plant: halls, a headframe, silos.
        self.block(-0.58, 0.69, 0.12, 0.05, 0.035, 0.5, 0xa9a9a2, 0.75);
        self.block(-0.52, 0.76, 0.08, 0.04, 0.028, 0.5, 0xa9a9a2, 0.75);
        self.block(-0.66, 0.78, 0.05, 0.05, 0.05, 0.5, 0x8f8f88, 0.75);
        self.block(-0.72, 0.52, 0.018, 0.018, 0.075, 0.3, 0xc8c8c0, 0.75);
        for i in 0..3 {
            let silo = self.make("CylinderGeometry", &[n(0.012), n(0.012), n(0.05), n(12.)]);
            call(&silo, "translate", &[n(0.), n(0.025), n(0.)]);
            let silo = self.keep(silo);
            let i = i as f64;
            self.build(silo, -0.46 + i * 0.03, 0.66 - i * 0.012, Some(ground_y(-0.46, 0.66)), 0., 0xbdbdb5, 0.3);
        }
        self.chimney = [-0.66, ground_y(-0.66, 0.78) + 0.05, 0.78];
        // Five-storey blocks.
        for r in 0..3 {
            for c in 0..4 {
                let (r, c) = (r as f64, c as f64);
                if hash(r, c) >= 0.15 {
                    self.block(-0.25 + c * 0.055 + r * 0.012, 0.95 + r * 0.04, 0.042, 0.012, 0.022, -0.35, 0x8a8a84, 0.55);
                }
            }
        }
        // The rail yard: sidings and standing wagons.
        for i in 0..4 {
            let i = i as f64;
            self.line(&drape(&[[0.18, 1.07 + i * 0.016], [0.48, 1.12 + i * 0.016]], 0.006, 0.02), Pen { width: 1., opacity: 0.35, ..Pen::default() });
        }
        for i in 0..7 {
            self.block(0.22 + (i % 4) as f64 * 0.06, 1.085 + (i / 4) as f64 * 0.032, 0.045, 0.01, 0.01, -0.16, 0x77756d, 0.45);
        }
        // Sludge ponds: white flat cells.
        for i in 0..3 {
            for j in 0..2 {
                let (x, z) = (-0.86 + i as f64 * 0.065, 1.08 + j as f64 * 0.06);
                let y = ground_y(x, z) + 0.004;
                let cell: Vec<f64> = [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.], [-1., -1.]]
                    .iter()
                    .flat_map(|[sx, sz]| [x + sx * 0.028, y, z + sz * 0.024])
                    .collect();
                self.line(&cell, Pen { width: 1.1, opacity: 0.6, ..Pen::default() });
            }
        }
        self.block(-0.2, -0.7, 0.03, 0.03, 0.06, 0., 0xa0a098, 0.75);
        // A sunk village: roofs under the reservoir.
        for i in 0..6 {
            let i = i as f64;
            let c = [0.56 + (hash(i, 7.) - 0.5) * 0.12, -1.14 + (hash(i, 8.) - 0.5) * 0.08];
            let roof: Vec<f64> = circle(c, 0.008, 4).iter().flat_map(|&[x, z]| [x, WATER + 0.002, z]).collect();
            self.line(&roof, Pen { color: CYAN, width: 1., opacity: 0.4, ..Pen::default() });
        }

        // Glows.
        self.tex = [
            self.glow_texture(&[(0., "rgba(255,214,150,1)"), (1., "rgba(233,120,40,0)")]),
            self.glow_texture(&[(0., "rgba(150,150,145,.55)"), (1., "rgba(90,90,88,0)")]),
            self.glow_texture(&[(0., "rgba(255,255,255,1)"), (0.25, "rgba(255,255,255,.5)"), (1., "rgba(255,255,255,0)")]),
            self.glow_texture(&[(0., "rgba(40,46,50,.9)"), (0.5, "rgba(30,34,38,.55)"), (1., "rgba(20,22,24,0)")]),
        ];
        // Blinking red lamps on the power line, the dam and the headframe.
        for (at, phase, size) in lamps {
            let glow = self.sprite(LIGHT, 0xff5a40, true, true);
            glow.at(at);
            glow.size(size, size);
            self.lamps.push((glow, phase));
        }
        for i in 0..9 {
            let glow = self.sprite(SMOKE, 0xffffff, false, true);
            self.puffs.push((glow, i as f64 / 9.));
        }
        // Trucks with warm headlights crawling along the roads, a freight train on the main line.
        self.paths = vec![Path::new(ROAD), Path::new(ROAD2), Path::new(RAIL)];
        for (path, speed, d) in [(0, 0.05, 0.3), (0, -0.04, 1.1), (1, 0.035, 0.1)] {
            let glow = self.sprite(LIGHT, 0xffd8a0, true, true);
            glow.size(0.03, 0.03);
            self.trucks.push(Truck { path, speed, d, glow });
        }
        for i in 0..6 {
            let head = i == 0;
            let car = self.boxed(if head { 0.03 } else { 0.036 }, if head { 0.014 } else { 0.011 }, 0.011);
            let group = self.shape(&car, if head { 0x9b9a92 } else { 0x77756d }, 0.6);
            set(&group, "visible", &flag(false));
            self.add(&group);
            self.train.push(group);
        }
        let light = self.sprite(LIGHT, 0xfff0d0, true, true);
        light.size(0.04, 0.04);
        self.train_light = Some(light);
        // Motes of dust hanging over the table, catching the light.
        let (mut pos, mut seed) = (Vec::new(), Vec::new());
        for i in 0..260 {
            let i = i as f64;
            pos.extend([((hash(i, 61.) - 0.5) * 2.6) as f32, (0.05 + hash(i, 62.) * 0.75) as f32, ((hash(i, 63.) - 0.5) * 3.4) as f32]);
            seed.push(hash(i, 64.) as f32);
        }
        let motes = self.geometry(&[("position", &pos, 3), ("seed", &seed, 1)]);
        self.motes_alpha = self.points(&motes, section("motes-vertex"), section("motes"), &[], true, 6.);

        // Each region's weather by its event: storms rain and strike, dry wind carries dust.
        for (ai, area) in AREAS.iter().enumerate() {
            match events[ai].as_str() {
                "storms" => {
                    let rain = self.region_points(area, 900, 71.);
                    let alpha = self.points(&rain, section("rain-vertex"), section("rain"), &[], false, 5.);
                    self.weather_alpha.push(alpha);
                    self.stormy.push(ai);
                    for i in 0..6 {
                        let i = i as f64 + ai as f64 * 6.;
                        let glow = self.sprite(CLOUD, 0xffffff, false, true);
                        let z = if area.north { -1.15 + hash(i, 82.) * 1.1 } else { 0.45 + hash(i, 82.) * 0.85 };
                        self.clouds.push(Cloud {
                            glow,
                            x: (hash(i, 81.) - 0.5) * 1.6,
                            z,
                            y: 0.5 + hash(i, 83.) * 0.1,
                            size: 0.3 + hash(i, 84.) * 0.22,
                            speed: 0.012 + hash(i, 85.) * 0.01,
                            north: area.north,
                        });
                    }
                }
                "dryWind" => {
                    let dust = self.region_points(area, 520, 91.);
                    let north = n(if area.north { 1. } else { 0. });
                    let alpha = self.points(&dust, section("dust-vertex"), section("dust"), &[("uNorth", north)], false, 5.);
                    self.weather_alpha.push(alpha);
                }
                _ => {}
            }
        }

        // Marks on the ground: each base with a radar, open maps, maps under shelling.
        let radar = self.make("CircleGeometry", &[n(0.16), n(64.)]);
        call(&radar, "rotateX", &[n(-PI / 2.)]);
        let radar = self.keep(radar);
        for (ai, area) in AREAS.iter().enumerate() {
            let [bx, bz] = area.base;
            let at = 1.75 + ai as f64 * 0.1;
            self.stem(area.base, 0.17, CYAN, 0.9, at);
            let hex: Vec<P> = (0..=6)
                .map(|i| {
                    let a = i as f64 / 6. * PI * 2. + PI / 6.;
                    [bx + a.cos() * 0.035, bz + a.sin() * 0.035]
                })
                .collect();
            self.line(&drape(&hex, 0.01, 0.005), Pen { color: CYAN, width: 1.6, opacity: 0.95, at: Some(at), ..Pen::default() });
            for k in 0..2 {
                let pulse = self.ring(area.base, 0.05, CYAN, 0.8, Pen { boot: false, ..Pen::default() });
                self.pulses.push((pulse, k as f64 * 0.5, ai));
            }
            let m = self.shader(
                section("radar-vertex"),
                &lib("radar"),
                &[("uOn", n(0.))],
                &[("transparent", flag(true)), ("depthWrite", flag(false))],
            );
            let sweep = self.mesh(&radar, &m, 4.);
            put3(&sweep, "position", [bx, ground_y(bx, bz) + 0.012, bz]);
            self.radars.push((Self::uniform_of(&m, "uOn"), ai, 0.));
            for (si, spot) in area.spots.iter().enumerate() {
                let at = at + 0.12 + si as f64 * 0.08;
                if views[ai].maps[si].locked() {
                    self.ring(spot.at, 0.085, AMBER, 0.7, Pen { dash: Some((0.014, 0.01)), at: Some(at), ..Pen::default() });
                    self.ring(spot.at, 0.05, 0xe0583f, 0.35, Pen { dash: Some((0.006, 0.008)), at: Some(at), ..Pen::default() });
                    self.stem(spot.at, 0.08, AMBER, 0.6, at);
                } else {
                    self.ring(spot.at, 0.03, FG, 0.85, Pen { at: Some(at), ..Pen::default() });
                    self.stem(spot.at, 0.11, FG, 0.9, at);
                }
            }
        }

        // Shelling: an arc from the nearest gun beyond the border, shells riding
        // it, smoke and craters.
        for (i, &[tx, tz]) in shelled.iter().enumerate() {
            let far = |g: &P| (g[0] - tx).hypot(g[1] - tz);
            let [gx, gz] = GUNS.into_iter().min_by(|a, b| far(a).total_cmp(&far(b))).unwrap_or(GUNS[0]);
            let fi = i as f64;
            let pts: Vec<[f64; 3]> = (0..=40)
                .map(|k| {
                    let t = k as f64 / 40.;
                    [mix(gx, tx, t), mix(0.15, ground_y(tx, tz), t) + (t * PI).sin() * 0.55, mix(gz, tz, t)]
                })
                .collect();
            let flat: Vec<f64> = pts.iter().flatten().copied().collect();
            self.line(&flat, Pen { color: AMBER, width: 1., dash: Some((0.02, 0.03)), opacity: 0.3, at: Some(2.2 + fi * 0.1), ..Pen::default() });
            let head = self.sprite(FLASH, 0xffffff, true, true);
            head.size(0.035, 0.035);
            self.shells.push(Shell { pts, head, t: hash(fi, 3.), speed: 0.35 + 0.1 * hash(fi, 4.), at: [tx, tz] });
            for k in 0..3 {
                let k = k as f64;
                let glow = self.sprite(SMOKE, 0xffffff, false, true);
                let (x, z) = (tx + (hash(fi, k + 10.) - 0.5) * 0.1, tz + (hash(fi, k + 20.) - 0.5) * 0.08);
                let base = ground_y(x, z) + 0.04 + k * 0.02;
                glow.at([x, base, z]);
                let size = 0.08 + 0.03 * k;
                glow.size(size, size);
                self.smokes.push(Smoke { glow, base, phase: hash(fi, k) * 6. });
            }
            for k in 0..5 {
                let k = k as f64;
                let c = [tx + (hash(fi, k + 30.) - 0.5) * 0.12, tz + (hash(fi, k + 40.) - 0.5) * 0.1];
                self.ring(c, 0.006 + 0.006 * hash(fi, k + 50.), 0x2a2622, 0.9, Pen { at: Some(1.6), ..Pen::default() });
            }
        }
    }

    // ---------- the living map ----------
    fn blast(&mut self, x: f64, z: f64) {
        let flash = self.sprite(FLASH, 0xffffff, true, false);
        flash.at([x, ground_y(x, z) + 0.012, z]);
        let ring = self.draw(&Self::ring_points([x, z], 0.03), Pen { color: AMBER, width: 1.4, boot: false, ..Pen::default() });
        put3(&ring.obj, "position", [x, 0., z]);
        self.blasts.push(Blast { flash, ring, age: 0. });
    }
    /// A lightning bolt: a jagged line from the cloud base to the ground with a side branch.
    fn strike(&mut self, ai: usize) {
        let area = &AREAS[ai];
        let (mut x, mut z) = (0., 0.);
        for _ in 0..50 {
            x = (random() - 0.5) * 1.9;
            z = (random() - 0.5) * 2.8;
            if in_area(area, x, z) {
                break;
            }
        }
        let (gy, top) = (ground_y(x, z), 0.66);
        let (px, pz) = (x + (random() - 0.5) * 0.12, z + (random() - 0.5) * 0.12);
        let (mut pts, mut branch) = (Vec::new(), Vec::new());
        for i in 0..=14 {
            let t = i as f64 / 14.;
            let y = mix(top, gy, t);
            let jx = mix(px, x, t) + (random() - 0.5) * 0.05 * (1. - t);
            let jz = mix(pz, z, t) + (random() - 0.5) * 0.05 * (1. - t);
            pts.extend([jx, y, jz]);
            if (5..=9).contains(&i) {
                let k = (i - 5) as f64;
                branch.extend([jx + k * 0.018, y - k * 0.004, jz + k * 0.01]);
            }
        }
        let bolt = self.draw(&pts, Pen { color: 0xe6f6ff, width: 2.2, boot: false, order: 7., ..Pen::default() });
        let twig = self.draw(&branch, Pen { color: 0xbfe6f5, width: 1.2, opacity: 0.7, boot: false, order: 7., ..Pen::default() });
        self.strikes.push(Strike { bolt, twig, x, z, age: 0. });
    }
    fn step(&mut self, s: &Step) {
        let (t, dt, live) = (s.t, s.dt, s.live);
        let u = &self.uniforms;
        for (key, value) in [("uTime", t), ("uBoot", s.boot), ("uCut", s.cut), ("uSel", s.sel), ("uWipe", s.wipe), ("uScan", s.scan)] {
            put(&get(u, key), value);
        }
        call(&self.ping, "set", &[n(s.ping[0]), n(s.ping[1]), n(s.ping[2])]);
        put(&self.water_alpha, phase(t, 1., 1.5));
        put(&self.motes_alpha, phase(t, 1.2, 2.6));
        // Power-on: lines come in, stems and buildings grow out of the ground.
        for line in self.lines.iter_mut().filter(|l| !l.done) {
            let k = phase(t, line.at, line.at + 0.4);
            set(&line.mat, "opacity", &n(line.base * k));
            set(&line.obj, "visible", &flag(k > 0.));
            if line.grow {
                set(&get(&line.obj, "scale"), "y", &n(out_back(k).max(0.001)));
            }
            line.done = k >= 1.;
        }
        for build in self.builds.iter_mut().filter(|b| !b.done) {
            let k = phase(t, build.at, build.at + 0.45);
            set(&build.group, "visible", &flag(k > 0.));
            set(&get(&build.group, "scale"), "y", &n(out_back(k).max(0.001)));
            build.done = k >= 1.;
        }
        // The world goes on: current, the trickle, trucks, the train, lamps, smoke.
        for (mat, speed) in &self.flows {
            set(mat, "dashOffset", &n(-t * speed));
        }
        for truck in &mut self.trucks {
            let path = &self.paths[truck.path];
            truck.d += truck.speed * dt;
            if truck.d > path.total + 0.1 {
                truck.d = -0.1;
            }
            if truck.d < -0.1 {
                truck.d = path.total + 0.1;
            }
            truck.glow.at(path.along(truck.d));
            let fade = ((truck.d.min(path.total - truck.d)) * 10. + 0.2).min(1.);
            truck.glow.opacity(if live { 0.85 * fade } else { 0. });
        }
        let rail = &self.paths[2];
        self.train_d += dt * 0.07;
        if self.train_d > rail.total + 0.6 {
            self.train_d = -0.25;
        }
        for (i, car) in self.train.iter().enumerate() {
            let d = self.train_d - i as f64 * 0.042;
            let on = live && d > 0. && d < rail.total;
            set(car, "visible", &flag(on));
            if on {
                let mut p = rail.along(d);
                p[1] -= 0.002;
                let ahead = rail.along(d + 0.01);
                put3(car, "position", p);
                set(&get(car, "rotation"), "y", &n((-(ahead[2] - p[2])).atan2(ahead[0] - p[0])));
            }
        }
        if let Some(light) = &self.train_light {
            light.at(rail.along(self.train_d + 0.02));
            light.opacity(if live && self.train_d > 0. && self.train_d < rail.total { 0.9 } else { 0. });
        }
        for (glow, phase) in &self.lamps {
            glow.opacity(if !live { 0. } else if (t * 0.7 + phase).rem_euclid(1.) < 0.18 { 1. } else { 0.12 });
        }
        let c = self.chimney;
        for (glow, k0) in &self.puffs {
            let k = (t * 0.1 + k0).rem_euclid(1.);
            glow.at([c[0] + k * 0.08, c[1] + k * 0.16, c[2] - k * 0.02]);
            let size = 0.02 + k * 0.09;
            glow.size(size, size);
            glow.opacity(if live { (PI * k).sin() * 0.45 } else { 0. });
            set(&glow.mat, "rotation", &n(k0 * 6. + t * 0.1));
        }
        for (pulse, phase, ai) in &self.pulses {
            let k = (t * 0.5 + phase).rem_euclid(1.);
            let size = 0.6 + k * 1.6;
            call(&get(&pulse.obj, "scale"), "set", &[n(size), n(1.), n(size)]);
            let strength = if *ai == s.selected { 0.8 } else { 0.3 };
            set(&pulse.mat, "opacity", &n(if live { (1. - k) * strength } else { 0. }));
        }
        for (on, ai, value) in &mut self.radars {
            let target = if !live { 0. } else if *ai == s.selected { 1. } else { 0.35 };
            *value += (target - *value) * (1. - (-dt * 4.).exp());
            put(on, *value);
        }

        // Weather: rain and dust fade in, clouds drift, lightning strikes.
        for alpha in &self.weather_alpha {
            put(alpha, phase(t, 2., 3.));
        }
        for cloud in &mut self.clouds {
            cloud.x += cloud.speed * dt;
            if cloud.x > 0.9 {
                cloud.x = -0.9;
            }
            let z = if cloud.north { cloud.z.min(border(cloud.x) - 0.15) } else { cloud.z.max(border(cloud.x) + 0.15) };
            cloud.glow.at([cloud.x, cloud.y, z]);
            cloud.glow.size(cloud.size * 1.6, cloud.size);
            cloud.glow.opacity(phase(t, 1.8, 3.) * 0.3 * ((0.9 - cloud.x.abs()) * 4.).min(1.));
            call(&get(&cloud.glow.mat, "color"), "setScalar", &[n(1. + self.flash_w * 4.)]);
        }
        if live {
            self.next_strike -= dt;
        }
        if self.next_strike < 0. && !self.stormy.is_empty() {
            let ai = self.stormy[(random() * self.stormy.len() as f64) as usize % self.stormy.len()];
            self.strike(ai);
            self.next_strike = 2.5 + random() * 4.;
        }
        let mut flash = [0., 0., 0.];
        for strike in &mut self.strikes {
            strike.age += dt;
            let a = strike.age;
            let flick = if a < 0.06 { 1. } else if a < 0.1 { 0.15 } else if a < 0.17 { 0.85 } else { (1. - (a - 0.17) / 0.25).max(0.) };
            set(&strike.bolt.mat, "opacity", &n(flick));
            set(&strike.twig.mat, "opacity", &n(flick * 0.7));
            flash = [strike.x, strike.z, flash[2].max(flick)];
        }
        self.flash_w = flash[2];
        call(&self.flash, "set", &[n(flash[0]), n(flash[1]), n(0.35), n(flash[2])]);
        let (old, kept): (Vec<Strike>, Vec<Strike>) = std::mem::take(&mut self.strikes).into_iter().partition(|s| s.age > 0.45);
        self.strikes = kept;
        for strike in old {
            self.drop_line(&strike.bolt);
            self.drop_line(&strike.twig);
        }

        // Shelling.
        let mut hits = Vec::new();
        for shell in &mut self.shells {
            shell.t += dt * shell.speed;
            if shell.t >= 1. {
                shell.t -= 1.;
                if live {
                    hits.push([shell.at[0] + (random() - 0.5) * 0.09, shell.at[1] + (random() - 0.5) * 0.07]);
                }
            }
            let i = ((shell.t * 40.).floor() as usize).min(39);
            let f = shell.t * 40. - (shell.t * 40.).floor();
            let (a, b) = (shell.pts[i], shell.pts[i + 1]);
            shell.head.at([mix(a[0], b[0], f), mix(a[1], b[1], f), mix(a[2], b[2], f)]);
            shell.head.opacity(if live { 1. } else { 0. });
        }
        if live {
            self.next_blast -= dt;
        }
        if self.next_blast < 0. && !self.shells.is_empty() {
            let at = self.shells[(random() * self.shells.len() as f64) as usize % self.shells.len()].at;
            hits.push([at[0] + (random() - 0.5) * 0.1, at[1] + (random() - 0.5) * 0.08]);
            self.next_blast = 0.6 + random() * 1.1;
        }
        for [x, z] in hits {
            self.blast(x, z);
        }
        for blast in &mut self.blasts {
            blast.age += dt;
            let size = 0.03 + blast.age * 0.25;
            blast.flash.size(size, size);
            blast.flash.opacity((1. - blast.age / 0.35).max(0.));
            let ring = 0.3 + blast.age * 2.2;
            call(&get(&blast.ring.obj, "scale"), "set", &[n(ring), n(1.), n(ring)]);
            set(&blast.ring.mat, "opacity", &n((1. - blast.age / 1.1).max(0.)));
        }
        let (old, kept): (Vec<Blast>, Vec<Blast>) = std::mem::take(&mut self.blasts).into_iter().partition(|b| b.age > 1.2);
        self.blasts = kept;
        for blast in old {
            call(&self.world, "remove", &[blast.flash.sprite.clone()]);
            call(&blast.flash.mat, "dispose", &[]);
            self.drop_line(&blast.ring);
        }
        for smoke in &self.smokes {
            put3(&smoke.glow.sprite, "position", [num(&get(&smoke.glow.sprite, "position"), "x"), smoke.base + 0.01 * (t * 0.4 + smoke.phase).sin(), num(&get(&smoke.glow.sprite, "position"), "z")]);
            set(&smoke.glow.mat, "rotation", &n(t * 0.05 + smoke.phase));
            smoke.glow.opacity(if live { 0.6 * phase(t, 2., 2.8) } else { 0. });
        }
    }
    fn resize(&mut self, w: f64, h: f64) {
        self.size = (w, h);
        call(&self.renderer, "setSize", &[n(w), n(h), flag(false)]);
        set(&self.camera, "aspect", &n(w / h));
        call(&self.camera, "updateProjectionMatrix", &[]);
        let ratio = call(&self.renderer, "getPixelRatio", &[]).as_f64().unwrap_or(1.);
        put(&get(&self.uniforms, "uPx"), h * ratio / 2. / (18f64).to_radians().tan());
        let live = self.lines.iter().chain(self.strikes.iter().flat_map(|s| [&s.bolt, &s.twig])).chain(self.blasts.iter().map(|b| &b.ring));
        for line in live {
            call(&get(&line.mat, "resolution"), "set", &[n(w), n(h)]);
        }
    }
    fn render(&self, pose: &Pose, offset: (f64, f64), dist: f64) {
        let (w, h) = self.size;
        put3(&self.camera, "position", pose.pos);
        call(&self.camera, "lookAt", &[n(LOOK[0]), n(LOOK[1]), n(LOOK[2])]);
        call(&self.camera, "setViewOffset", &[n(w), n(h), n(offset.0), n(offset.1), n(w), n(h)]);
        call(&self.fog, "set", &[n(dist - 1.), n(dist + 2.)]);
        call(&self.renderer, "render", &[self.scene.clone(), self.camera.clone()]);
    }
    fn dispose(&mut self) {
        for line in self.lines.iter().chain(self.strikes.iter().flat_map(|s| [&s.bolt, &s.twig])).chain(self.blasts.iter().map(|b| &b.ring)) {
            call(&line.geo, "dispose", &[]);
            call(&line.mat, "dispose", &[]);
        }
        for blast in &self.blasts {
            call(&blast.flash.mat, "dispose", &[]);
        }
        for resource in self.res.drain(..) {
            call_optional(&resource, "dispose", &[]);
        }
        self.lines.clear();
        self.strikes.clear();
        self.blasts.clear();
        call(&self.scene, "clear", &[]);
        call(&self.renderer, "dispose", &[]);
    }
}

// ---------- the region card and the marks over the map ----------
struct MapLine {
    name: String,
    /// Weather and clock of an open map, none while the clock keeps it shut.
    now: Option<(String, String)>,
    /// Why it is shut and for how long, or the warning before it shuts.
    note: String,
    /// Still being drawn: shut even while the clock keeps it open.
    planned: bool,
}
impl MapLine {
    fn locked(&self) -> bool {
        self.now.is_none() || self.planned
    }
}
/// What the card says about a region right now.
struct AreaView {
    name: String,
    base: String,
    event: String,
    event_name: String,
    odds: Vec<(String, f64)>,
    maps: Vec<MapLine>,
    /// The spot under fire now, and where the guns go next and when.
    shelled: Option<usize>,
    shelling: String,
}
fn mins(ms: f64) -> String {
    format!("{} мин", (ms / 60_000.).ceil().max(1.))
}
fn world_at(now: f64, map: &str) -> Value {
    crate::dispatch("client.world.at", &serde_json::json!([now, map]).to_string())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}
impl AreaView {
    fn read(area: &Area, now: f64) -> Self {
        let c = game_core::constants();
        let first = base_region::maps(area.id).first().cloned().unwrap_or_default();
        let event = world_at(now, &first)["event"].as_str().unwrap_or("calm").to_string();
        let odds = c["WEATHERS"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|w| w.as_str())
            .map(|w| (w.to_string(), c["REGION_EVENTS"][&event]["odds"][w].as_f64().unwrap_or(0.)))
            .collect();
        let worlds: Vec<Value> = area.spots.iter().map(|spot| world_at(now, spot.map)).collect();
        let word = |w: &Value, key: &str| w[key].as_str().unwrap_or("").to_string();
        let left = |w: &Value| w["changeInMs"].as_f64().map(mins);
        let maps = area
            .spots
            .iter()
            .zip(&worlds)
            .map(|(spot, w)| {
                let open = w["open"].as_bool().unwrap_or(true);
                let planned = !c["MAPS"][spot.map].is_object();
                let note = match (open, left(w)) {
                    (false, Some(left)) => format!("{} · {left}", word(w, "closedName")),
                    (false, None) => word(w, "closedName"),
                    (true, _) if planned => "разминирование".into(),
                    (true, Some(left)) if w["changeInMs"].as_f64().unwrap_or(f64::MAX) <= 300_000. => {
                        format!("{} через {left}", word(w, "nextClosedName"))
                    }
                    _ => String::new(),
                };
                MapLine {
                    name: word(w, "mapName"),
                    now: open.then(|| (word(w, "weather"), word(w, "clock"))),
                    note,
                    planned,
                }
            })
            .collect();
        let named = |key: &str| area.spots.iter().position(|s| Some(s.map) == worlds[0][key].as_str());
        let name_of = |i: Option<usize>| i.map(|i| word(&worlds[i], "mapName")).unwrap_or_default();
        let (shelled, next) = (named("shelled"), named("nextShelled"));
        let moves = worlds[0]["shellingMovesInMs"].as_f64().map(mins).unwrap_or_default();
        let shelling = match (shelled, next) {
            (Some(_), Some(n)) if Some(n) != shelled => {
                format!("Обстрел: {} · через {moves} → {}", name_of(shelled), name_of(next))
            }
            (Some(_), _) => format!("Обстрел: {} · ещё {moves}", name_of(shelled)),
            (None, Some(_)) => format!("Обстрел через {moves}: {}", name_of(next)),
            _ => String::new(),
        };
        Self {
            name: base_region::name(area.id),
            base: base_region::REGIONS.iter().find(|r| r.id == area.id).map(|r| r.title()).unwrap_or_default(),
            event_name: c["REGION_EVENTS"][&event]["name"].as_str().unwrap_or("").into(),
            event,
            odds,
            maps,
            shelled,
            shelling,
        }
    }
}
fn icon(id: &str) -> String {
    let path = match id {
        "clear" => r#"<circle cx="12" cy="12" r="4"/><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4"/>"#,
        "dust" => r#"<path d="M3 8h11a3 3 0 1 0-3-3M3 12h15a3 3 0 1 1-3 3M3 16h8"/>"#,
        "storm" => r#"<path d="M7 15a5 5 0 1 1 1-9.9A6 6 0 0 1 19 9a4 4 0 0 1-1 7.9"/><path d="M12 12l-2 4h4l-2 4"/>"#,
        "fog" => r#"<path d="M4 9h16M2 13h20M5 17h14"/>"#,
        "chemical" => r#"<circle cx="12" cy="12" r="2"/><path d="M12 10V4a8 8 0 0 0-6.9 4M13.7 13l5.2 3a8 8 0 0 0 0-8M10.3 13l-5.2 3a8 8 0 0 0 13.8 0"/>"#,
        _ => "",
    };
    format!(r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{path}</svg>"#)
}
fn event_icon(event: &str) -> &'static str {
    match event {
        "storms" => "storm",
        "dryWind" => "dust",
        "chemical" => "chemical",
        _ => "clear",
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Area,
    Base,
    Map,
    Locked,
}
struct Mark {
    kind: Kind,
    area: usize,
    anchor: [f64; 3],
    left: bool,
    text: Vec<char>,
    el: JsValue,
    inner: JsValue,
    word: JsValue,
    a: f64,
    drawn: f64,
    want: bool,
    from: f64,
    on: bool,
    shown: String,
}
impl Mark {
    fn new(layer: &JsValue, kind: Kind, area: usize, anchor: [f64; 3], left: bool, text: &str) -> Self {
        let (tag, class, inner) = match kind {
            Kind::Area => ("button", "cd-mark cd-mark-area", "<b></b>".to_string()),
            Kind::Base => (
                "div",
                "cd-mark cd-mark-base",
                r#"<i><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M4 11l8-6 8 6v9H4z"/></svg></i><strong></strong>"#.to_string(),
            ),
            Kind::Map => ("div", "cd-mark cd-mark-map", r#"<span class="cd-mark-dot"></span><strong></strong>"#.to_string()),
            Kind::Locked => (
                "div",
                "cd-mark cd-mark-map cd-mark-locked",
                r#"<span class="cd-mark-cross"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="8"/><path d="M12 1v6M12 17v6M1 12h6M17 12h6"/></svg></span><strong></strong>"#.to_string(),
            ),
        };
        let el = create(tag, &format!("{class}{}", if left { " cd-mark-left" } else { "" }));
        if kind == Kind::Area {
            attr(&el, "type", "button");
            attr(&el, "aria-label", text);
            set(&get(&el, "dataset"), "area", &js(&area.to_string()));
        } else {
            attr(&el, "aria-hidden", "true");
        }
        html(&el, &format!(r#"<span class="cd-mark-in">{inner}</span>"#));
        call(layer, "append", &[el.clone()]);
        let inner = query(&el, ".cd-mark-in");
        style(&inner, "opacity", "0");
        Self {
            kind,
            area,
            anchor,
            left,
            text: text.chars().collect(),
            word: query(&el, "b, strong"),
            el,
            inner,
            a: 0.,
            drawn: -1.,
            want: false,
            from: 0.,
            on: false,
            shown: String::new(),
        }
    }
    /// The text decodes from scrambled glyphs, left to right.
    fn decrypt(&mut self, t: f64) {
        let len = self.text.len();
        let k = (t - self.from) / (0.3 + len as f64 * 0.03);
        let next: String = if k >= 1. {
            self.text.iter().collect()
        } else {
            let done = ((k * len as f64).floor().max(0.)) as usize;
            let glyphs: Vec<char> = GLYPHS.chars().collect();
            self.text
                .iter()
                .enumerate()
                .map(|(i, &ch)| {
                    if i < done || ch == ' ' {
                        ch
                    } else {
                        glyphs[((hash(i as f64, (t * 24.).floor()) * glyphs.len() as f64) as usize).min(glyphs.len() - 1)]
                    }
                })
                .collect()
        };
        if next != self.shown {
            content(&self.word, &next);
            self.shown = next;
        }
    }
}

struct Drag {
    x: f64,
    y: f64,
    last: f64,
    moved: f64,
    yaw: f64,
}

struct State {
    section: JsValue,
    canvas: JsValue,
    card: JsValue,
    body: JsValue,
    styles: JsValue,
    gl: Option<Gl>,
    views: Vec<AreaView>,
    marks: Vec<Mark>,
    selected: usize,
    current: usize,
    t: f64,
    last: Option<f64>,
    booted: bool,
    width: f64,
    height: f64,
    wide: bool,
    yaw: f64,
    pitch: f64,
    yaw_target: f64,
    user_yaw: f64,
    user_vel: f64,
    look: (f64, f64),
    drag: Option<Drag>,
    pose: Pose,
    offset: (f64, f64),
    dist: f64,
    sel: f64,
    relief: bool,
    wipe: f64,
    scan: f64,
    wiping: Option<(f64, f64, f64)>,
    ping: [f64; 3],
    card_k: f64,
    fill_at: Option<f64>,
    grow_at: Option<f64>,
    leave_at: Option<f64>,
    left: bool,
    disposed: bool,
}

fn yaw_for(area: usize) -> f64 {
    if AREAS[area].north { -0.26 } else { -0.1 }
}

impl State {
    fn new(host: &JsValue, section: JsValue) -> Result<Self, JsValue> {
        let canvas = query(&section, ".cd-region-canvas");
        let labels = query(&section, ".cd-region-labels");
        let card = query(&section, ".cd-region-card");
        let body = query(&card, ".cd-region-card-body");
        let styles = query(&section, ".cd-region-styles");
        if [&canvas, &labels, &card, &body, &styles].iter().any(|el| !exists(el)) {
            return Err(js("The region map markup is missing"));
        }
        let now = js_sys::Date::now();
        let views: Vec<AreaView> = AREAS.iter().map(|a| AreaView::read(a, now)).collect();
        let events: Vec<String> = views.iter().map(|v| v.event.clone()).collect();
        let gl = match Gl::new(host, &canvas, &events) {
            Ok(mut gl) => {
                gl.build_scene(&views);
                Some(gl)
            }
            Err(error) => {
                call(&get(&global(), "console"), "warn", &[js("crowned-dogs: the region map has no WebGL"), error]);
                None
            }
        };
        let current = AREAS.iter().position(|a| a.id == base_region::current().id).unwrap_or(0);
        html(&labels, "");
        let mut marks = Vec::new();
        for (ai, area) in AREAS.iter().enumerate() {
            let view = &views[ai];
            let top = |c: P, h: f64| [c[0], ground_y(c[0], c[1]) + h, c[1]];
            marks.push(Mark::new(&labels, Kind::Base, ai, top(area.base, 0.17), false, &view.base));
            for (si, spot) in area.spots.iter().enumerate() {
                let (kind, h) = if view.maps[si].locked() { (Kind::Locked, 0.08) } else { (Kind::Map, 0.11) };
                marks.push(Mark::new(&labels, kind, ai, top(spot.at, h), spot.left, &view.maps[si].name));
            }
            marks.push(Mark::new(&labels, Kind::Area, ai, top(area.label, 0.05), false, &view.name));
        }
        let reduced = boolean(&call(&global(), "matchMedia", &[js("(prefers-reduced-motion: reduce)")]), "matches");
        let relief = platform::store_get("cd-map-style").as_string().as_deref() == Some("relief");
        let mut state = Self {
            section,
            canvas,
            card,
            body,
            styles,
            gl,
            views,
            marks,
            selected: current,
            current,
            t: if reduced { 2.4 } else { 0. },
            last: None,
            booted: false,
            width: 0.,
            height: 0.,
            wide: false,
            yaw: if reduced { yaw_for(current) } else { -0.95 },
            pitch: 1.3,
            yaw_target: yaw_for(current),
            user_yaw: 0.,
            user_vel: 0.,
            look: (0., 0.),
            drag: None,
            pose: Pose::new([0., 1., 0.], 5., 1.),
            offset: (0., 0.),
            dist: 5.,
            sel: if AREAS[current].north { 1. } else { 0. },
            relief,
            wipe: 0.,
            scan: 0.,
            wiping: None,
            ping: [0., 0., -99.],
            card_k: -1.,
            fill_at: None,
            grow_at: None,
            leave_at: None,
            left: false,
            disposed: false,
        };
        state.set_style(relief, false);
        state.fill();
        state.grow_at = Some(1.7);
        state.mark_selection();
        hud::liquid_glass(state.section.clone());
        Ok(state)
    }

    fn card_html(&self) -> String {
        let view = &self.views[self.selected];
        let best = view.odds.iter().fold(None::<&(String, f64)>, |best, o| match best {
            Some(b) if b.1 >= o.1 => Some(b),
            _ => Some(o),
        });
        let odds: String = view
            .odds
            .iter()
            .map(|(id, p)| {
                let top = if best.is_some_and(|b| &b.0 == id) { " class=\"cd-top\"" } else { "" };
                format!(r#"<div{top}>{}<span class="cd-region-pct">{p:.0}%</span><span class="cd-region-bar"><i style="--w:{p:.0}%"></i></span></div>"#, icon(id))
            })
            .collect();
        let maps: String = view
            .maps
            .iter()
            .map(|m| match &m.now {
                Some((weather, clock)) if !m.locked() => {
                    let soon = if m.note.is_empty() { String::new() } else { format!(r#"<small class="cd-soon">{}</small>"#, m.note) };
                    format!(r#"<li><span class="cd-region-dot"></span><div>{}<small>{}{clock}</small>{soon}</div></li>"#, m.name, icon(weather))
                }
                _ => format!(r#"<li class="cd-locked"><span class="cd-region-dot"></span><div>{}<small>{}</small></div></li>"#, m.name, m.note),
            })
            .collect();
        let shelling = if view.shelling.is_empty() {
            String::new()
        } else {
            format!(r#"<p class="cd-region-shelling">{}</p>"#, view.shelling)
        };
        let go = if self.selected == self.current {
            r#"<button class="cd-region-go glass cd-here" type="button">Вы здесь</button>"#.to_string()
        } else {
            format!(r#"<button class="cd-region-go glass" type="button">Перейти на базу «{}»</button>"#, view.base)
        };
        format!(
            r#"<div class="cd-region-top"><h2>{}</h2><span class="cd-region-chip">База «{}»</span></div><div class="cd-region-event">{}<b>{}</b></div><div class="cd-region-odds">{odds}</div><ul class="cd-region-maps">{maps}</ul>{shelling}{go}"#,
            view.name,
            view.base,
            icon(event_icon(&view.event)),
            view.event_name
        )
    }
    fn fill(&mut self) {
        html(&self.body, &self.card_html());
        toggle(&self.card, "cd-swap", false);
        toggle(&self.card, "cd-grown", false);
        hud::liquid_glass(self.card.clone());
        self.fill_at = None;
        self.grow_at = Some(self.t);
    }
    fn render_card(&mut self, animate: bool) {
        if animate {
            toggle(&self.card, "cd-swap", true);
            self.fill_at = Some(self.t + 0.17);
        } else {
            self.fill();
        }
    }
    fn mark_selection(&self) {
        for mark in &self.marks {
            toggle(&mark.el, "cd-mark-on", mark.area == self.selected);
            if mark.kind == Kind::Area {
                attr(&mark.el, "aria-pressed", &(mark.area == self.selected).to_string());
            }
        }
    }
    fn ping(&mut self, area: usize) {
        let [x, z] = AREAS[area].base;
        self.ping = [x, z, self.t];
    }
    fn select(&mut self, area: usize) {
        if area == self.selected && self.booted {
            return;
        }
        self.selected = area;
        self.render_card(self.booted);
        let t = self.t;
        if t > 1.9 {
            self.ping(area);
        }
        self.mark_selection();
        for (k, mark) in self.marks.iter_mut().enumerate() {
            let want = mark.kind == Kind::Area || mark.area == area;
            if want != mark.want && t > 1.9 {
                mark.want = want;
                mark.from = t + if want { (0.05 * k as f64) % 0.3 } else { 0. };
            }
        }
        self.yaw_target = yaw_for(area);
    }
    /// The card's button: stay, or move the base to the picked region and leave the map.
    fn go(&mut self) {
        if self.leave_at.is_some() {
            return;
        }
        if self.selected == self.current {
            self.leave_at = Some(self.t);
            return;
        }
        self.current = self.selected;
        self.render_card(true);
        self.ping(self.current);
        self.leave_at = Some(self.t + 0.7);
    }
    fn set_style(&mut self, relief: bool, animate: bool) {
        let name = if relief { "relief" } else { "contour" };
        set(&get(&self.styles, "dataset"), "style", &js(name));
        for button in all(&self.styles, "button[data-style]") {
            attr(&button, "aria-pressed", &(data(&button, "style") == name).to_string());
        }
        if !animate {
            self.relief = relief;
            self.wipe = if relief { 1. } else { 0. };
            self.wiping = None;
            return;
        }
        if relief == self.relief {
            return;
        }
        self.relief = relief;
        platform::store_set("cd-map-style", name);
        self.wiping = Some((self.wipe, if relief { 1. } else { 0. }, self.t));
    }

    /// Fits the block into the part of the screen the card leaves free.
    fn place(&mut self, zoom: f64, card: (f64, f64)) {
        let (w, h) = (self.width, self.height);
        let aspect = w / h;
        let (l, r, t, b) = if self.wide { (0.02, (card.0 - 16.) / w, 70. / h, 0.98) } else { (0.02, 0.98, 70. / h, (card.1 - 6.) / h) };
        let (r, b) = (r.max(l + 0.2), b.max(t + 0.2));
        let dir = [self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), self.yaw.cos() * self.pitch.cos()];
        let (mut lo, mut hi, mut sx, mut sy) = (1., 40., 0., 0.);
        for _ in 0..26 {
            let d = (lo + hi) / 2.;
            let pose = Pose::new(dir, d, aspect);
            let mut ext = [f64::MAX, f64::MIN, f64::MAX, f64::MIN];
            let mut seen = true;
            for x in [-1., 1.] {
                for z in [-1.45, 1.45] {
                    for y in [-0.12, 0.42] {
                        match pose.ndc([x, y, z]) {
                            Some((px, py)) => ext = [ext[0].min(px), ext[1].max(px), ext[2].min(py), ext[3].max(py)],
                            None => seen = false,
                        }
                    }
                }
            }
            if seen && (ext[1] - ext[0]) / 2. <= r - l && (ext[3] - ext[2]) / 2. <= b - t {
                hi = d;
                sx = (ext[0] + ext[1]) / 2.;
                sy = (ext[2] + ext[3]) / 2.;
            } else {
                lo = d;
            }
        }
        self.dist = hi * zoom;
        self.pose = Pose::new(dir, self.dist, aspect);
        let (cx, cy) = ((sx + 1.) / 2. * w, (1. - sy) / 2. * h);
        self.offset = (cx - (l + r) / 2. * w, cy - (t + b) / 2. * h);
    }
    fn resize(&mut self, w: f64, h: f64) {
        self.width = w;
        self.height = h;
        self.wide = w > h;
        toggle(&self.section, "cd-region-wide", self.wide);
        if let Some(gl) = self.gl.as_mut() {
            gl.resize(w, h);
        }
    }

    fn frame(&mut self, now: f64) -> Option<String> {
        if self.disposed {
            return None;
        }
        let dt = self.last.map_or(0., |last| ((now - last) / 1000.).clamp(0., 0.1));
        self.last = Some(now);
        let (w, h) = (num(&self.section, "clientWidth"), num(&self.section, "clientHeight"));
        if w < 2. || h < 2. {
            return None;
        }
        if w != self.width || h != self.height {
            self.resize(w, h);
        }
        let card = (num(&self.card, "offsetLeft"), num(&self.card, "offsetTop"));
        self.t += dt;
        let t = self.t;
        let live = t > 2.;
        // Power-on: the table lights, the block rises, things grow, marks drop in, the card arrives.
        let boot = out_cubic(phase(t, 0., 0.8));
        let cut = if t < 1.75 { mix(FLOOR, 0.42, in_out_cubic(phase(t, 0.3, 1.7))) } else { 9. };
        let cam = in_out_cubic(phase(t, 0., 2.3));
        if !self.booted && t > 1.9 {
            self.booted = true;
            self.ping(self.selected);
            let selected = self.selected;
            for (k, mark) in self.marks.iter_mut().enumerate() {
                mark.want = mark.kind == Kind::Area || mark.area == selected;
                mark.from = t + k as f64 * 0.04;
            }
        }
        let ck = out_cubic(phase(t, 1.55, 2.15));
        if ck != self.card_k {
            self.card_k = ck;
            style(&self.card, "opacity", &format!("{ck:.3}"));
            let slide = if self.wide { format!("translateX({:.1}px)", (1. - ck) * 40.) } else { format!("translateY({:.1}px)", (1. - ck) * 50.) };
            style(&self.card, "transform", &slide);
        }
        // The region dimming follows the pick; a style switch wipes the block from one look to the other.
        let north = if AREAS[self.selected].north { 1. } else { 0. };
        self.sel += (north - self.sel) * (1. - (-dt * 6.).exp());
        if let Some((from, to, t0)) = self.wiping {
            let k = phase(t, t0, t0 + 0.9);
            self.wipe = mix(from, to, in_out_cubic(k));
            self.scan = (PI * k).sin() * 0.9;
            if k >= 1. {
                self.wiping = None;
                self.scan = 0.;
            }
        }
        // The camera swoops down from above, then breathes; a drag adds a turn that drifts back.
        if self.drag.is_none() {
            self.user_yaw += self.user_vel;
            self.user_vel *= 0.92;
            self.user_yaw *= 0.995;
        }
        let sway = (t * 0.21).sin() * 0.035 + (t * 0.13 + 1.).sin() * 0.02;
        let settle = self.yaw_target + sway + self.user_yaw + self.look.0 * 0.06;
        self.yaw = if cam < 1. { mix(-0.95, settle, cam) } else { self.yaw + (settle - self.yaw) * (1. - (-dt * 3.).exp()) };
        self.pitch = mix(1.3, 0.74 + (t * 0.17).sin() * 0.012 + self.look.1 * 0.04, cam);
        self.place(mix(1.25, 1., cam), card);

        for mark in &mut self.marks {
            let target = if mark.want && t >= mark.from { 1. } else { 0. };
            let rate = if target > 0. { 9. } else { 14. };
            mark.a += (target - mark.a) * (1. - (-dt * rate).exp());
            mark.on = target > 0.;
        }
        if let Some(gl) = self.gl.as_mut() {
            gl.step(&Step { t, dt, live, boot, cut, sel: self.sel, wipe: self.wipe, scan: self.scan, selected: self.selected, ping: self.ping });
            gl.render(&self.pose, self.offset, self.dist);
        }
        // Labels follow their anchors; they fade and slide in, their text decodes.
        for mark in &mut self.marks {
            let (x, y) = match self.pose.ndc([mark.anchor[0], mark.anchor[1] * 1.7, mark.anchor[2]]) {
                Some((nx, ny)) => ((nx + 1.) / 2. * w - self.offset.0, (1. - ny) / 2. * h - self.offset.1),
                None => (-999., -999.),
            };
            let shift = match (mark.kind, mark.left) {
                (Kind::Area, _) => "translate(-50%, -50%)",
                (_, true) => "translate(-100%, -50%) translateX(4px)",
                _ => "translate(-4px, -50%)",
            };
            style(&mark.el, "transform", &format!("translate({x:.1}px, {y:.1}px) {shift}"));
            if (mark.a - mark.drawn).abs() > 0.002 {
                mark.drawn = mark.a;
                style(&mark.inner, "opacity", &format!("{:.3}", mark.a));
                style(&mark.inner, "transform", &format!("translateY({:.2}px)", (1. - mark.a) * 8.));
            }
            if mark.on {
                mark.decrypt(t);
            }
        }
        // The card: its bars grow after a fill; a swap fills it once the old body has faded.
        if self.grow_at.is_some_and(|at| t >= at) && self.fill_at.is_none() {
            self.grow_at = None;
            toggle(&self.card, "cd-grown", true);
        }
        if self.fill_at.is_some_and(|at| t >= at) {
            self.fill();
        }
        if !self.left && self.leave_at.is_some_and(|at| t >= at) {
            self.left = true;
            return Some(AREAS[self.current].id.into());
        }
        None
    }

    fn local(&self, event: &JsValue) -> (f64, f64) {
        let rect = call(&self.canvas, "getBoundingClientRect", &[]);
        (num(event, "clientX") - num(&rect, "left"), num(event, "clientY") - num(&rect, "top"))
    }
    fn pointer_down(&mut self, event: &JsValue) {
        let (x, y) = self.local(event);
        self.drag = Some(Drag { x, y, last: x, moved: 0., yaw: self.user_yaw });
        call(&self.canvas, "setPointerCapture", &[get(event, "pointerId")]);
    }
    fn pointer_move(&mut self, event: &JsValue) {
        let (x, y) = self.local(event);
        match self.drag.as_mut() {
            None => self.look = (x / self.width.max(1.) - 0.5, y / self.height.max(1.) - 0.5),
            Some(drag) => {
                drag.moved = drag.moved.max((x - drag.x).hypot(y - drag.y));
                self.user_vel = -(x - drag.last) * 0.004;
                drag.last = x;
                self.user_yaw = (drag.yaw - (x - drag.x) * 0.004).clamp(-0.8, 0.8);
            }
        }
    }
    /// A tap picks the region under the finger; a tap during the power-on skips it.
    fn pointer_up(&mut self, event: &JsValue) {
        let Some(drag) = self.drag.take() else { return };
        if drag.moved >= 6. {
            return;
        }
        self.user_vel = 0.;
        if !self.booted {
            self.t = self.t.max(2.05);
            return;
        }
        let (x, y) = self.local(event);
        let nx = (x + self.offset.0) / self.width.max(1.) * 2. - 1.;
        let ny = 1. - (y + self.offset.1) / self.height.max(1.) * 2.;
        if let Some((hx, hz)) = self.pose.hit(nx, ny, 0.2)
            && let Some(area) = AREAS.iter().position(|a| in_area(a, hx, hz))
        {
            self.select(area);
        }
    }
    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        if let Some(mut gl) = self.gl.take() {
            gl.dispose();
        }
        for mark in self.marks.drain(..) {
            call(&mark.el, "remove", &[]);
        }
        html(&self.body, "");
    }
}

fn listen(weak: &Weak<RefCell<State>>, target: &JsValue, kind: &str, handler: fn(&mut State, &JsValue)) -> Listener {
    let weak = weak.clone();
    Listener::new(target, kind, move |event| {
        if let Some(state) = weak.upgrade()
            && let Ok(mut state) = state.try_borrow_mut()
        {
            handler(&mut state, &event);
        }
    })
}

#[wasm_bindgen]
pub struct RustRegionMap {
    state: Rc<RefCell<State>>,
    listeners: Vec<Listener>,
}
#[wasm_bindgen]
impl RustRegionMap {
    /// `host` carries THREE and the fat-line classes Line2, LineGeometry and LineMaterial.
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, section: JsValue) -> Result<RustRegionMap, JsValue> {
        let state = Rc::new(RefCell::new(State::new(&host, section)?));
        let weak = Rc::downgrade(&state);
        let (canvas, labels, card, styles) = {
            let s = state.borrow();
            (s.canvas.clone(), query(&s.section, ".cd-region-labels"), s.card.clone(), s.styles.clone())
        };
        let listeners = vec![
            listen(&weak, &canvas, "pointerdown", State::pointer_down),
            listen(&weak, &canvas, "pointermove", State::pointer_move),
            listen(&weak, &canvas, "pointerup", State::pointer_up),
            listen(&weak, &canvas, "pointercancel", |s, _| s.drag = None),
            listen(&weak, &labels, "click", |s, event| {
                let mark = closest(event, "[data-area]");
                if let Ok(area) = data(&mark, "area").parse::<usize>()
                    && area < AREAS.len()
                {
                    s.select(area);
                }
            }),
            listen(&weak, &card, "click", |s, event| {
                if exists(&closest(event, ".cd-region-go")) {
                    s.go();
                }
            }),
            listen(&weak, &styles, "click", |s, event| {
                let button = closest(event, "button[data-style]");
                if exists(&button) {
                    s.set_style(data(&button, "style") == "relief", true);
                }
            }),
        ];
        Ok(Self { state, listeners })
    }
    /// Draws a frame; gives the region's id once the player picked a base to move to.
    pub fn frame(&self, now: f64) -> JsValue {
        match self.state.try_borrow_mut() {
            Ok(mut state) => state.frame(now).map_or(JsValue::UNDEFINED, |id| js(&id)),
            Err(_) => JsValue::UNDEFINED,
        }
    }
    pub fn dispose(&mut self) {
        self.listeners.clear();
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.dispose();
        }
    }
}
impl Drop for RustRegionMap {
    fn drop(&mut self) {
        self.dispose();
    }
}
