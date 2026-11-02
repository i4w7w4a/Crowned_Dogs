//! Arena ownership and frame orchestration. Three.js objects are opaque renderer handles.
use super::{bridge::*, hazard_fx::HazardFx, shield_fx::ShieldFx};
use js_sys::{Float64Array, Promise};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, future_to_promise, spawn_local};

const NEAR: f64 = 1.;
const ZONES: [&str; 4] = ["head", "body", "groin", "legs"];
fn n(v: f64) -> JsValue {
    v.into()
}
fn p(o: &JsValue, k: &str) -> f64 {
    num(&get(o, k))
}
fn sdk(host: &JsValue, key: &str, args: &[JsValue]) -> JsValue {
    invoke(&get(host, key), args)
}
fn make(host: &JsValue, key: &str, args: &[JsValue]) -> JsValue {
    construct(&get(&get(host, "THREE"), key), args)
}
fn create(host: &JsValue, key: &str, args: &[JsValue]) -> JsValue {
    construct(&get(host, key), args)
}
fn policy(method: &str, args: serde_json::Value) -> serde_json::Value {
    game_client::dispatch(method, args).expect("arena policy")
}
fn math(kind: &str, args: &[f64]) -> Vec<f64> {
    game_client::arena_compute(&format!("arena.{kind}"), args).expect("arena arithmetic")
}
fn arr(v: &[f64]) -> JsValue {
    Float64Array::from(v).into()
}
fn xyz(o: &JsValue) -> [f64; 3] {
    [p(o, "x"), p(o, "y"), p(o, "z")]
}
fn vec_set(o: &JsValue, v: &[f64]) {
    call(o, "set", &v.iter().copied().map(n).collect::<Vec<_>>());
}
fn copy(to: &JsValue, from: &JsValue) {
    call(to, "copy", &[from.clone()]);
}
fn dispose(o: &JsValue) {
    if present(o) {
        call(o, "dispose", &[]);
    }
}
fn traverse(root: &JsValue) -> Vec<JsValue> {
    let objects = Rc::new(RefCell::new(vec![]));
    let target = objects.clone();
    let visitor = Closure::<dyn FnMut(JsValue)>::new(move |v| target.borrow_mut().push(v));
    call(root, "traverse", &[visitor.as_ref().clone()]);
    objects.take()
}
fn meshes(root: &JsValue) -> Vec<JsValue> {
    traverse(root)
        .into_iter()
        .filter(|o| boolean(&get(o, "isMesh")))
        .collect()
}
fn stage_source(frame: &JsValue) -> Option<(String, JsValue)> {
    let descriptor = get(frame, "projectedStage");
    let manifest = string(&get(&descriptor, "manifest"));
    if !manifest.is_empty() {
        Some((manifest, descriptor))
    } else if string(&get(frame, "id")) == "hall1" {
        Some(("img/battle-stage/manifest.json".into(), JsValue::NULL))
    } else {
        None
    }
}
fn soft_texture(host: &JsValue, stops: &[(f64, &str)]) -> JsValue {
    let canvas = call(&document(), "createElement", &["canvas".into()]);
    set(&canvas, "width", &64.into());
    set(&canvas, "height", &64.into());
    let g = call(&canvas, "getContext", &["2d".into()]);
    let gradient = call(
        &g,
        "createRadialGradient",
        &[
            32.into(),
            32.into(),
            0.into(),
            32.into(),
            32.into(),
            32.into(),
        ],
    );
    for (at, color) in stops {
        call(&gradient, "addColorStop", &[n(*at), (*color).into()]);
    }
    set(&g, "fillStyle", &gradient);
    call(&g, "fillRect", &[0.into(), 0.into(), 64.into(), 64.into()]);
    let t = make(host, "CanvasTexture", &[canvas]);
    set(
        &t,
        "colorSpace",
        &get(&get(host, "THREE"), "SRGBColorSpace"),
    );
    t
}
fn shader_patch(shader: &JsValue, kind: &str) {
    let result = policy(
        "client.arena.shader",
        serde_json::json!([
            kind,
            string(&get(shader, "vertexShader")),
            string(&get(shader, "fragmentShader"))
        ]),
    );
    set(shader, "vertexShader", &json(&result["vertex"]));
    set(shader, "fragmentShader", &json(&result["fragment"]));
}

struct Figure {
    group: JsValue,
    tilt: JsValue,
    model: JsValue,
    blob: JsValue,
    mixer: JsValue,
    pose: JsValue,
    motion: JsValue,
    hand: JsValue,
    feet: JsValue,
    gait: JsValue,
    collapse: JsValue,
    hold_out: JsValue,
    flash: JsValue,
    port: JsValue,
    bones: JsValue,
    kick: [f64; 4],
    /// How far the face turns to the opponent, 0..1 (see `rigging::look_at`).
    look: f64,
    /// The gun `arm` last gave this fighter.
    gun: String,
    fall_t: f64,
    fall_side: f64,
    /// The kit last put on, as `head|body|suit|boots`.
    worn: String,
}
struct Mirror {
    group: JsValue,
    tilt: JsValue,
    model: JsValue,
    // (source, reflection) bone positions and quaternions
    pairs: Vec<[JsValue; 4]>,
    mats: Rc<RefCell<Vec<JsValue>>>,
}
/// Holds drawing to about 120 frames a second on 240 Hz and faster screens, and to about 60 on
/// a phone or a fitting screen over 100 Hz: drawing every refresh there only heats the device, and
/// a whole number of refreshes per frame paces evenly. `?nocap` turns it off for measurements.
#[derive(Default)]
pub(super) struct FramePace {
    target: f64,
    last: f64,
    gaps: Vec<f64>,
    refresh: f64,
    every: f64,
    drawn: f64,
}
impl FramePace {
    /// The battle's pace: 120 frames on a computer, 60 on a touch phone.
    fn battle() -> Self {
        let phone = get(&global(), "matchMedia").is_function()
            && boolean(&get(
                &call(&global(), "matchMedia", &["(pointer: coarse)".into()]),
                "matches",
            ));
        Self::new(if phone { 13. } else { 8. })
    }
    /// `target` is the shortest gap between drawn frames, in milliseconds, worth having.
    pub(super) fn new(target: f64) -> Self {
        Self { target, ..Self::default() }
    }
    /// Minimal quality holds the battle to about 30 frames: a weak phone then stays cool and even.
    fn follow(&mut self, quality: &JsValue) {
        let target = if string(&get(quality, "id")) == "minimal" {
            30.
        } else {
            Self::battle().target
        };
        if target != self.target {
            *self = Self::new(target);
        }
    }
    pub(super) fn due(&mut self, time: f64) -> bool {
        if self.every == 0. {
            let gap = time - self.last;
            self.last = time;
            if gap > 0. && gap < 50. {
                self.gaps.push(gap);
            }
            if self.gaps.len() >= 90 {
                self.gaps.sort_by(f64::total_cmp);
                self.refresh = self.gaps[self.gaps.len() / 2];
                let search = string(&get(&get(&global(), "location"), "search"));
                let off = js_sys::RegExp::new(r"[?&]nocap", "").test(&search);
                self.every = if self.refresh < self.target * 0.575 && !off {
                    (self.target / self.refresh).ceil()
                } else {
                    1.
                };
                self.gaps = vec![];
            }
            return true;
        }
        if self.every <= 1. || time - self.drawn >= self.every * self.refresh - self.refresh * 0.5 {
            self.drawn = time;
            return true;
        }
        false
    }
}

#[derive(Clone)]
struct Walk {
    from: f64,
    to: f64,
    t: f64,
    dur: f64,
    mine: f64,
}
#[derive(Clone, Default)]
struct Calibration {
    fn_: f64,
    d7: f64,
    cam_h: f64,
    hz: f64,
    kx: f64,
    me: [f64; 3],
    back: f64,
    aspect: f64,
}
impl Calibration {
    fn new(frame: &JsValue, fp: bool) -> Self {
        let v = policy(
            "client.arena.calibrate",
            serde_json::json!([data(frame), fp]),
        );
        let number = |k: &str| v[k].as_f64().unwrap_or_default();
        Self {
            fn_: number("fn"),
            d7: number("d7"),
            cam_h: number("camH"),
            hz: number("hz"),
            kx: number("kx"),
            me: [
                v["me"]["x"].as_f64().unwrap_or_default(),
                v["me"]["y"].as_f64().unwrap_or_default(),
                v["me"]["z"].as_f64().unwrap_or_default(),
            ],
            back: number("back"),
            aspect: number("aspect"),
        }
    }
}
struct ArenaState {
    disposed: bool,
    host: JsValue,
    renderer: JsValue,
    scene: JsValue,
    camera: JsValue,
    timer: JsValue,
    hemi: JsValue,
    rim: JsValue,
    fill: JsValue,
    me_key: JsValue,
    back_light: JsValue,
    bounce: JsValue,
    spark: JsValue,
    /// The «Зонт» domes that show a blocked hit: the enemy's in the far pass, mine in the near one.
    enemy_shield: ShieldFx,
    my_shield: ShieldFx,
    /// Lightning, shells, the drone and a falling roof, and the lines of the warned stretch.
    hazard: HazardFx,
    /// The warned stretch's edges in steps ahead of me, while it is marked on the floor.
    warned: Option<[f64; 2]>,
    muzzle_light: JsValue,
    catcher: JsValue,
    dust: JsValue,
    lens: JsValue,
    projected_stage: JsValue,
    stage_loading: Promise,
    stage_load_generation: u64,
    stage_enabled: bool,
    stage_travel: f64,
    walk_origin: f64,
    motion_debug: bool,
    quality: JsValue,
    env: JsValue,
    env_src: String,
    frame: JsValue,
    k: Option<Calibration>,
    me: Option<Figure>,
    enemy: Option<Figure>,
    mirror: Option<Mirror>,
    arms_only: JsValue,
    viewmodel: JsValue,
    in_water: JsValue,
    ready: bool,
    running: bool,
    fp: bool,
    my_gun: String,
    dist: f64,
    depth: f64,
    walk: Option<Walk>,
    spark_life: f64,
    spark_max: f64,
    flash_of: Option<bool>,
    suppressed: [bool; 2],
    view_at: [f64; 3],
    view_tilt: f64,
    view_set: bool,
    bob_phase: f64,
    bob_k: f64,
    bob_depth: f64,
    views: serde_json::Value,
    muzzle: serde_json::Value,
    wide: f64,
    fp_key: f64,
    wide_k: JsValue,
    wide_y: JsValue,
    width: f64,
    height: f64,
    view: JsValue,
    sun_dir: JsValue,
    tmp_a: JsValue,
    tmp_b: JsValue,
    tmp_size: JsValue,
    aim: JsValue,
    tiers: Vec<JsValue>,
    textures_done: Promise,
    on_frame: JsValue,
    /// Resolvers waiting for the next frame that reaches the screen.
    drawn: Vec<JsValue>,
    animation: Option<Closure<dyn FnMut(f64)>>,
    pace: FramePace,
    widen: Closure<dyn FnMut(JsValue)>,
    mirror_shader: Closure<dyn FnMut(JsValue)>,
    callbacks: Vec<Closure<dyn FnMut(JsValue)>>,
    material_callbacks: Vec<Closure<dyn FnMut(JsValue) -> JsValue>>,
    marks: Rc<RefCell<Vec<(bool, String)>>>,
}

#[wasm_bindgen]
pub struct RustArena {
    state: Rc<RefCell<ArenaState>>,
}

#[wasm_bindgen]
impl RustArena {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, canvas: JsValue, options: JsValue) -> Self {
        console_error_panic_hook::set_once();
        let renderer = make(
            &host,
            "WebGLRenderer",
            &[object(&[
                ("canvas", canvas),
                ("alpha", true.into()),
                ("antialias", false.into()),
                ("powerPreference", "high-performance".into()),
            ])],
        );
        call(&renderer, "setClearColor", &[0.into(), 0.into()]);
        let three = get(&host, "THREE");
        set(
            &renderer,
            "outputColorSpace",
            &get(&three, "SRGBColorSpace"),
        );
        set(
            &renderer,
            "toneMapping",
            &get(&three, "ACESFilmicToneMapping"),
        );
        set(&get(&renderer, "shadowMap"), "enabled", &true.into());
        set(
            &get(&renderer, "shadowMap"),
            "type",
            &get(&three, "PCFShadowMap"),
        );
        let lens = create(&host, "Lens", &[renderer.clone()]);
        // Optional only for the frozen painted-renderer test host and halls that
        // do not have a reconstructed stage. The production host supplies it.
        let projected_stage = if get(&host, "ProjectedStage").is_function() {
            create(&host, "ProjectedStage", &[renderer.clone()])
        } else {
            JsValue::NULL
        };
        let scene = make(&host, "Scene", &[]);
        set(
            &scene,
            "fog",
            &make(&host, "Fog", &[n(0x3d342c as f64), n(2.5), n(30.)]),
        );
        let camera = make(
            &host,
            "PerspectiveCamera",
            &[n(45.), n(1024. / 1536.), n(0.1), n(80.)],
        );
        let hemi = make(
            &host,
            "HemisphereLight",
            &[n(0xc9b49a as f64), n(0x231e1a as f64), n(0.6)],
        );
        let rim = make(&host, "DirectionalLight", &[n(0xffb06a as f64), n(3.4)]);
        set(&rim, "castShadow", &true.into());
        let shadow = get(&rim, "shadow");
        vec_set(&get(&shadow, "mapSize"), &[1024., 1024.]);
        set(&shadow, "radius", &n(5.));
        set(&shadow, "bias", &n(-0.0004));
        set(&shadow, "normalBias", &n(0.02));
        for (key, value) in [
            ("left", -5.),
            ("right", 5.),
            ("top", 5.),
            ("bottom", -5.),
            ("near", 1.),
            ("far", 40.),
        ] {
            set(&get(&shadow, "camera"), key, &n(value));
        }
        let fill = make(
            &host,
            "SpotLight",
            &[
                n(0xd9c3a3 as f64),
                n(2.4),
                n(0.),
                n(17_f64.to_radians()),
                n(0.7),
                n(0.),
            ],
        );
        let me_key = make(
            &host,
            "SpotLight",
            &[
                n(0xb4b4b0 as f64),
                n(5.),
                n(0.),
                n(15_f64.to_radians()),
                n(0.8),
                n(0.),
            ],
        );
        let bounce = make(
            &host,
            "PointLight",
            &[n(0x46d8d8 as f64), n(0.22), n(3.), n(2.)],
        );
        let back = make(&host, "DirectionalLight", &[n(0x8ea3b5 as f64), n(0.45)]);
        vec_set(&get(&back, "position"), &[1., 3., 8.]);
        call(
            &scene,
            "add",
            &[
                hemi.clone(),
                rim.clone(),
                get(&rim, "target"),
                fill.clone(),
                get(&fill, "target"),
                me_key.clone(),
                get(&me_key, "target"),
                bounce.clone(),
                back.clone(),
            ],
        );
        for light in [&hemi, &rim, &fill, &me_key, &bounce, &back] {
            call(&get(light, "layers"), "enable", &[n(NEAR)]);
        }
        let catcher = make(
            &host,
            "Mesh",
            &[
                make(&host, "PlaneGeometry", &[n(40.), n(40.)]),
                make(
                    &host,
                    "ShadowMaterial",
                    &[object(&[
                        ("color", n(0x0a0705 as f64)),
                        ("opacity", n(0.58)),
                        ("depthWrite", false.into()),
                    ])],
                ),
            ],
        );
        set(
            &get(&catcher, "rotation"),
            "x",
            &n(-std::f64::consts::FRAC_PI_2),
        );
        set(&catcher, "receiveShadow", &true.into());
        let dust = create(&host, "Dust", &[]);
        call(&scene, "add", &[catcher.clone(), get(&dust, "points")]);
        let spark = make(
            &host,
            "Sprite",
            &[make(
                &host,
                "SpriteMaterial",
                &[object(&[
                    (
                        "map",
                        soft_texture(
                            &host,
                            &[
                                (0., "rgba(255,248,230,1)"),
                                (0.25, "rgba(255,190,110,.85)"),
                                (1., "rgba(255,120,40,0)"),
                            ],
                        ),
                    ),
                    ("blending", get(&three, "AdditiveBlending")),
                    ("depthWrite", false.into()),
                    ("depthTest", false.into()),
                    ("transparent", true.into()),
                    ("fog", false.into()),
                ])],
            )],
        );
        set(&spark, "visible", &false.into());
        call(&scene, "add", &[spark.clone()]);
        let muzzle_light = make(
            &host,
            "PointLight",
            &[n(0xffb36b as f64), n(0.), n(4.), n(2.)],
        );
        call(&get(&muzzle_light, "layers"), "enable", &[n(NEAR)]);
        call(&scene, "add", &[muzzle_light.clone()]);
        let timer = make(&host, "Timer", &[]);
        call(&timer, "connect", &[document()]);
        let wide_k = object(&[("value", n(1.))]);
        let wide_y = object(&[("value", n(0.))]);
        let wk = wide_k.clone();
        let wy = wide_y.clone();
        let enemy_shield = ShieldFx::new(
            &three,
            &scene,
            0.,
            &object(&[("value", n(1.))]),
            &object(&[("value", n(0.))]),
        );
        let my_shield = ShieldFx::new(&three, &scene, NEAR, &wide_k, &wide_y);
        let hazard = HazardFx::new(&three, &scene);
        let widen = Closure::<dyn FnMut(JsValue)>::new(move |shader| {
            set(&get(&shader, "uniforms"), "wideK", &wk);
            set(&get(&shader, "uniforms"), "wideY", &wy);
            shader_patch(&shader, "wide");
        });
        let mirror_shader =
            Closure::<dyn FnMut(JsValue)>::new(move |shader| shader_patch(&shader, "mirror"));
        let config = policy(
            "client.arena.data",
            serde_json::json!([data(&get(&options, "fpview"))]),
        );
        let vector = || make(&host, "Vector3", &[]);
        let state = Rc::new(RefCell::new(ArenaState {
            disposed: false,
            renderer,
            scene,
            camera,
            timer,
            hemi,
            rim,
            fill,
            me_key,
            back_light: back,
            bounce,
            spark,
            enemy_shield,
            my_shield,
            hazard,
            warned: None,
            muzzle_light,
            catcher,
            dust,
            lens,
            projected_stage,
            stage_loading: Promise::resolve(&JsValue::UNDEFINED),
            stage_load_generation: 0,
            stage_enabled: false,
            stage_travel: 0.,
            walk_origin: 0.,
            motion_debug: boolean(&get(&options, "motionDebug")),
            quality: get(&host, "quality"),
            env: JsValue::NULL,
            env_src: String::new(),
            frame: JsValue::NULL,
            k: None,
            me: None,
            enemy: None,
            mirror: None,
            arms_only: JsValue::NULL,
            viewmodel: JsValue::NULL,
            in_water: JsValue::NULL,
            ready: false,
            running: false,
            fp: false,
            my_gun: "pistol".into(),
            dist: 7.,
            depth: 0.,
            walk: None,
            spark_life: 0.,
            spark_max: 0.18,
            flash_of: None,
            suppressed: [false; 2],
            view_at: [0.; 3],
            view_tilt: 0.,
            view_set: false,
            bob_phase: 0.,
            bob_k: 0.,
            bob_depth: 0.02,
            views: config["views"].clone(),
            muzzle: config["muzzle"].clone(),
            wide: p(&options, "wide"),
            fp_key: p(&options, "fpkey"),
            wide_k,
            wide_y,
            width: 1.,
            height: 1.,
            view: object(&[("x", n(0.)), ("y", n(0.)), ("w", n(1.)), ("h", n(1.))]),
            sun_dir: make(&host, "Vector3", &[n(0.), n(0.5), n(-1.)]),
            tmp_a: vector(),
            tmp_b: vector(),
            tmp_size: make(&host, "Vector2", &[]),
            aim: vector(),
            tiers: vec![],
            textures_done: Promise::resolve(&JsValue::UNDEFINED),
            on_frame: JsValue::NULL,
            drawn: vec![],
            animation: None,
            pace: FramePace::battle(),
            widen,
            mirror_shader,
            callbacks: vec![],
            material_callbacks: vec![],
            marks: Rc::new(RefCell::new(vec![])),
            host,
        }));
        let weak = Rc::downgrade(&state);
        state.borrow_mut().animation = Some(Closure::<dyn FnMut(f64)>::new(move |time| {
            let Some(state) = weak.upgrade() else {
                return;
            };
            let (callback, drawn) = {
                let mut s = state.borrow_mut();
                if !s.running || !s.ready || !s.pace.due(time) {
                    return;
                }
                super::fps_meter::frame(time);
                s.tick(time);
                (s.on_frame.clone(), std::mem::take(&mut s.drawn))
            };
            for resolve in drawn {
                invoke(&resolve, &[]);
            }
            // DOM ring placement may synchronously call zoneOnScreen, so release the frame borrow first.
            if present(&callback) {
                invoke(&callback, &[]);
            }
            super::fps_meter::done();
        }));
        Self { state }
    }

    #[wasm_bindgen(getter)]
    pub fn ready(&self) -> bool {
        self.state.borrow().ready
    }
    #[wasm_bindgen(getter)]
    pub fn gl(&self) -> JsValue {
        call(&self.state.borrow().renderer, "getContext", &[])
    }
    /// Lightning strikes the lens has drawn in this battle picture.
    #[wasm_bindgen(getter)]
    pub fn strikes(&self) -> f64 {
        num(&get(&self.state.borrow().lens, "strikes"))
    }
    #[wasm_bindgen(getter,js_name=onFrame)]
    pub fn on_frame(&self) -> JsValue {
        self.state.borrow().on_frame.clone()
    }
    #[wasm_bindgen(setter,js_name=onFrame)]
    pub fn set_on_frame(&self, value: JsValue) {
        self.state.borrow_mut().on_frame = value;
    }
    #[wasm_bindgen(getter,js_name=texturesDone)]
    pub fn textures_done(&self) -> Promise {
        self.state.borrow().textures_done.clone()
    }

    pub fn load(&self, me_url: String, enemy_url: String, distance: f64) -> Promise {
        let state = self.state.clone();
        if state.borrow().disposed {
            return Promise::resolve(&JsValue::UNDEFINED);
        }
        let host = state.borrow().host.clone();
        let same = me_url == enemy_url;
        let requests = array(&[
            sdk(&host, "loadModel", &[me_url.clone().into()]),
            if same {
                JsValue::NULL
            } else {
                sdk(&host, "loadModel", &[enemy_url.clone().into()])
            },
            sdk(&host, "loadGuns", &[]),
            state.borrow().stage_loading.clone().into(),
        ]);
        future_to_promise(async move {
            let models = JsFuture::from(Promise::all(&requests)).await?;
            let mine = at(&models, 0);
            let theirs = at(&models, 1);
            let guns = at(&models, 2);
            let foe = if present(&theirs) {
                theirs.clone()
            } else {
                mine.clone()
            };
            let mut s = state.borrow_mut();
            if s.disposed {
                return Ok(JsValue::UNDEFINED);
            }
            s.enemy = Some(s.make_figure(&foe, 0x77756f, 0.42, 0.4, 0, -1., false));
            s.me = Some(s.make_figure(&mine, 0x8a8780, 0.85, 0., 0x0a0908, 1., true));
            let me = s.me.as_ref().unwrap();
            super::look::first_person(me.model.clone(), s.fp, super::platform::current_look());
            super::look::apply(me.model.clone(), super::platform::current_look());
            set(&me.blob, "visible", &(!s.fp).into());
            s.arms_only = sdk(&s.host, "armsOnly", &[s.me.as_ref().unwrap().model.clone()]);
            invoke(&s.arms_only, &[s.fp.into()]);
            s.apply_lighting();
            s.mirror = Some(s.make_mirror(&foe));
            s.give_weapons(&mine, &foe, &guns);
            let own = s.me.as_ref().unwrap().model.clone();
            let enemy = s.enemy.as_ref().unwrap().model.clone();
            let reflected = s.mirror.as_ref().unwrap().model.clone();
            s.tiers = if present(&theirs) {
                vec![
                    create(
                        &host,
                        "TextureTiers",
                        &[array(&[own]).into(), me_url.into()],
                    ),
                    create(
                        &host,
                        "TextureTiers",
                        &[array(&[enemy, reflected]).into(), enemy_url.into()],
                    ),
                ]
            } else {
                vec![create(
                    &host,
                    "TextureTiers",
                    &[array(&[own, enemy, reflected]).into(), me_url.into()],
                )]
            };
            s.ready = true;
            let quality = s.quality.clone();
            s.set_quality(quality);
            s.reset(distance);
            if s.running {
                s.resume();
            }
            Ok(JsValue::UNDEFINED)
        })
    }
    pub fn resume(&self) {
        self.state.borrow_mut().resume();
    }
    /// Repaints the player's figure after the look was edited between battles.
    #[wasm_bindgen(js_name=applyLook)]
    pub fn apply_look(&self) {
        let mut state = self.state.borrow_mut();
        let s = &mut *state;
        let Some(model) = s.me.as_ref().map(|me| me.model.clone()) else {
            return;
        };
        // The first-person cut hides the head by visibility, and the repaint shows the face again:
        // lift the cut, repaint, then cut the new look (else the camera sits inside the head).
        if get(&s.arms_only, "dispose").is_function() {
            let _ = try_call(&s.arms_only, "dispose", &[]);
        }
        super::look::first_person(model.clone(), s.fp, super::platform::current_look());
        super::look::apply(model.clone(), super::platform::current_look());
        s.arms_only = sdk(&s.host, "armsOnly", &[model]);
        invoke(&s.arms_only, &[s.fp.into()]);
    }
    /// Compiles every shader of the scene ahead of the first frame.
    pub fn warm(&self) -> Promise {
        let s = self.state.borrow();
        if !get(&s.renderer, "compileAsync").is_function() {
            return Promise::resolve(&JsValue::UNDEFINED);
        }
        // the lens renders into its own targets, which need other programs than the screen
        match try_call(
            &s.lens,
            "warm",
            &[s.scene.clone(), s.camera.clone(), n(NEAR)],
        ) {
            Ok(p) => future_to_promise(async move {
                let _ = JsFuture::from(Promise::resolve(&p)).await;
                Ok(JsValue::UNDEFINED)
            }),
            Err(_) => Promise::resolve(&JsValue::UNDEFINED),
        }
    }
    /// Resolves once the running loop has put the next frame on screen.
    pub fn drawn(&self) -> Promise {
        let state = self.state.clone();
        Promise::new(&mut |resolve, _| {
            state.borrow_mut().drawn.push(resolve.into());
        })
    }
    pub fn pause(&self) {
        let mut s = self.state.borrow_mut();
        s.running = false;
        call(&s.renderer, "setAnimationLoop", &[JsValue::NULL]);
    }
    pub fn dispose(&self) {
        self.state.borrow_mut().dispose();
    }
    #[wasm_bindgen(js_name=setQuality)]
    pub fn set_quality(&self, q: JsValue) {
        self.state.borrow_mut().set_quality(q);
    }
    #[wasm_bindgen(js_name=setFrame)]
    pub fn set_frame(&self, f: JsValue) {
        let src = {
            let mut s = self.state.borrow_mut();
            s.set_frame(f.clone());
            string(&get(&f, "src"))
        };
        load_stage(&self.state);
        make_env(&self.state, src);
    }
    /// Presentation follows the exact match used for accuracy and shield rules.
    /// Local play is side 0; a future network view can pass its own side explicitly.
    #[wasm_bindgen(js_name=setConditions)]
    pub fn set_conditions(&self, match_state: JsValue, side: Option<u32>) {
        let mut s = self.state.borrow_mut();
        if s.disposed {
            return;
        }
        let m = data(&match_state);
        let own = side.unwrap_or(0).min(1) as usize;
        let night_vision = m["config"]["time"].as_str() == Some("night")
            && m["fighters"][own]["gear"]["nvg"].as_bool() == Some(true);
        if get(&s.lens, "setNightVision").is_function() {
            call(&s.lens, "setNightVision", &[night_vision.into()]);
        }
        if get(&s.lens, "setEvents").is_function() {
            let events = m["config"]["events"].as_bool() == Some(true);
            call(&s.lens, "setEvents", &[events.into()]);
        }
        let fx = policy(
            "client.world.fx",
            serde_json::json!([
                m["config"]["weather"],
                m["config"]["time"],
                m["config"]["mapId"]
            ]),
        );
        if get(&s.lens, "setBattleAtmosphere").is_function() && s.k.is_some() {
            let heat = fx["heat"].as_f64().unwrap_or(0.);
            let glare = m["config"]["time"].as_str() == Some("evening")
                && m["fighters"][own]["facesSun"].as_bool() == Some(true);
            call(
                &s.lens,
                "setBattleAtmosphere",
                &[
                    (heat > 0.).into(),
                    glare.into(),
                    n(1. / s.depth_of(8.)),
                    n(1. / s.depth_of(12.)),
                    n(heat),
                ],
            );
        }
        // the enemy's field shows faintly in heat and in rain (my own would cover the screen)
        s.enemy_shield.set_weather(
            fx["heat"].as_f64().unwrap_or(0.),
            fx["wet"].as_f64().unwrap_or(0.),
        );
        if get(&s.lens, "setWeather").is_function() {
            call(
                &s.lens,
                "setWeather",
                &[js_sys::JSON::parse(&fx.to_string()).unwrap_or(JsValue::NULL)],
            );
        }
        for (slot, fighter) in [own, 1 - own].into_iter().enumerate() {
            let spec = game_core::dispatch(
                "weaponSpec",
                serde_json::json!([m, fighter, m["fighters"][fighter]["weapon"]]),
            )
            .unwrap_or_default();
            s.suppressed[slot] = spec["suppressor"].as_bool() == Some(true);
        }
        let dataset = get(&get(&s.renderer, "domElement"), "dataset");
        if present(&dataset) {
            set(
                &dataset,
                "nightVision",
                &(if night_vision { "on" } else { "off" }).into(),
            );
        }
    }
    pub fn resize(&self, width: f64, height: f64, view: JsValue) {
        self.state.borrow_mut().resize(width, height, view);
    }
    pub fn arm(&self, who: String, gun: String, mods: JsValue) {
        let mut s = self.state.borrow_mut();
        if !s.ready {
            return;
        }
        let enemy = who == "enemy";
        let f = if enemy {
            s.enemy.as_mut().unwrap()
        } else {
            s.me.as_mut().unwrap()
        };
        call(&f.motion, "arm", &[gun.clone().into()]);
        f.gun.clone_from(&gun);
        if present(&f.hand) {
            call(&f.hand, "fit", &[mods.clone()]);
        }
        if enemy && present(&s.in_water) {
            call(&s.in_water, "fit", &[mods]);
        }
        if !enemy {
            s.my_gun = gun;
        }
    }
    /// Puts on a fighter's kit (`head`, `body`, `suit`, `boots` item ids); the same kit again does nothing.
    pub fn wear(&self, who: String, kit: JsValue) {
        let mut s = self.state.borrow_mut();
        if !s.ready || !kit.is_object() {
            return;
        }
        let wear = object(&["head", "body", "suit", "boots", "colors"].map(|k| (k, get(&kit, k))));
        let key = ["head", "body", "suit", "boots"]
            .map(|k| get(&kit, k).as_string().unwrap_or_default())
            .join("|")
            + &js_sys::JSON::stringify(&get(&kit, "colors"))
                .ok()
                .and_then(|t| t.as_string())
                .unwrap_or_default();
        let enemy = who == "enemy";
        let s = &mut *s;
        let f = if enemy {
            s.enemy.as_mut()
        } else {
            s.me.as_mut()
        }
        .unwrap();
        if f.worn == key {
            return;
        }
        f.worn = key;
        let model = f.model.clone();
        if enemy {
            sdk(&s.host, "dress", &[model, wear.clone()]);
            if let Some(m) = &s.mirror {
                sdk(&s.host, "dress", &[m.model.clone(), wear]);
            }
            return;
        }
        // The first-person cut keeps the triangles it saw: rebuild it for the new kit.
        if get(&s.arms_only, "dispose").is_function() {
            let _ = try_call(&s.arms_only, "dispose", &[]);
        }
        sdk(&s.host, "dress", &[model.clone(), wear]);
        s.arms_only = sdk(&s.host, "armsOnly", &[model.clone()]);
        invoke(&s.arms_only, &[s.fp.into()]);
        super::look::first_person(model, s.fp, super::platform::current_look());
    }
    #[wasm_bindgen(js_name=move)]
    pub fn move_to(&self, distance: f64, me_move: f64, enemy_move: f64) {
        let mut s = self.state.borrow_mut();
        if !s.ready {
            return;
        }
        s.finish_walk();
        let k = s.k.as_ref().unwrap();
        let plan = policy(
            "client.arena.move",
            serde_json::json!([
                s.depth,
                distance,
                me_move,
                s.dist,
                k.d7,
                k.back,
                s.stage_enabled,
                enemy_move
            ]),
        );
        s.dist = distance;
        s.walk_origin = s.stage_travel;
        if plan["walk"].is_null() {
            s.walk = None;
            s.depth = plan["to"].as_f64().unwrap();
            return;
        }
        let w = &plan["walk"];
        let walk = Walk {
            from: w["from"].as_f64().unwrap(),
            to: w["to"].as_f64().unwrap(),
            t: w["t"].as_f64().unwrap(),
            dur: w["dur"].as_f64().unwrap(),
            mine: w["mine"].as_f64().unwrap(),
        };
        if let Some(ground) = plan["stride"].as_f64() {
            s.stride(s.enemy.as_ref().unwrap(), ground, walk.dur);
        } else if s.stage_enabled && enemy_move == 0. {
            call(&s.enemy.as_ref().unwrap().motion, "stand", &[n(0.15)]);
        }
        if let Some(bob) = plan["bob"].as_f64() {
            s.bob_depth = bob;
        }
        s.walk = Some(walk);
    }
    pub fn attack(&self, who: String, kind: String, in_seconds: f64) {
        let s = self.state.borrow();
        if !s.ready {
            return;
        }
        let f = if who == "enemy" {
            s.enemy.as_ref().unwrap()
        } else {
            s.me.as_ref().unwrap()
        };
        call(&f.motion, "attack", &[kind.into(), n(in_seconds)]);
    }
    /// `weapon` and `broken` (the shield gave way) shape the dome when the hit is blocked.
    pub fn hit(
        &self,
        who: String,
        zone: String,
        kind: String,
        weapon: Option<String>,
        broken: Option<bool>,
    ) {
        self.state.borrow_mut().hit(
            &who,
            &zone,
            &kind,
            weapon.as_deref().unwrap_or("pistol"),
            broken.unwrap_or(false),
        );
    }
    pub fn fall(&self, who: String) {
        let mut s = self.state.borrow_mut();
        if !s.ready {
            return;
        }
        let f = if who == "enemy" {
            s.enemy.as_mut().unwrap()
        } else {
            s.me.as_mut().unwrap()
        };
        if f.fall_t >= 0. {
            return;
        }
        f.fall_t = 0.;
        call(&f.motion, "fall", &[]);
    }
    /// Marks the stretch the coming hazard will strike with two amber lines on the floor, and
    /// dims the lights for a blackout; `null` takes the lines away while the turn plays.
    #[wasm_bindgen(js_name=hazardMark)]
    pub fn hazard_mark(&self, match_state: JsValue) {
        let mut s = self.state.borrow_mut();
        if !s.ready {
            return;
        }
        let m = data(&match_state);
        let h = &m["hazard"];
        let now = m["config"]["events"].as_bool() == Some(true)
            && !h["kind"].is_null()
            && h["turn"] == m["turn"];
        s.warned = match (now, h["at"].as_f64(), m["lane"][0].as_f64()) {
            (true, Some(at), Some(me)) => {
                let reach = h["radius"].as_f64().unwrap_or(0.) + 0.5;
                Some([at - me - reach, at - me + reach])
            }
            _ => None,
        };
        let marked = s.warned.is_some();
        s.hazard.show_mark(marked);
        if !m.is_null() && get(&s.lens, "setDim").is_function() {
            let dark = now && h["kind"].as_str() == Some("blackout");
            call(&s.lens, "setDim", &[n(if dark { 0.3 } else { 1. })]);
        }
    }
    /// Plays this turn's hazard from the report and returns the seconds until it hits.
    #[wasm_bindgen(js_name=hazardStrike)]
    pub fn hazard_strike(&self, report: JsValue) -> f64 {
        let mut s = self.state.borrow_mut();
        if !s.ready {
            return 0.;
        }
        s.strike(&data(&report))
    }
    #[wasm_bindgen(js_name=setFirstPerson)]
    pub fn set_first_person(&self, on: bool) {
        self.state.borrow_mut().first_person(on);
    }
    pub fn reset(&self, distance: f64) {
        self.state.borrow_mut().reset(distance);
    }
    /// Where each fighter sounds from: [our pan, enemy pan], from the camera's left-right angle to them.
    pub fn ears(&self) -> JsValue {
        let s = self.state.borrow();
        if !s.ready {
            return JsValue::UNDEFINED;
        }
        let inverse = get(&s.camera, "matrixWorldInverse");
        let mut out = [0.0; 2];
        for (slot, figure) in [s.me.as_ref(), s.enemy.as_ref()].into_iter().enumerate() {
            let Some(figure) = figure else { continue };
            call(&figure.group, "getWorldPosition", &[s.tmp_a.clone()]);
            call(&s.tmp_a, "applyMatrix4", &[inverse.clone()]);
            let (x, z) = (p(&s.tmp_a, "x"), p(&s.tmp_a, "z"));
            let reach = (x * x + z * z).sqrt().max(0.001);
            out[slot] = (x / reach * 0.9).clamp(-0.85, 0.85);
        }
        // A plain array: it goes through JSON to the cue policy, where a typed array would become an object.
        js_sys::Array::of2(&n(out[0]), &n(out[1])).into()
    }
    #[wasm_bindgen(js_name=zoneOnScreen)]
    pub fn zone_on_screen(&self, zone: String) -> JsValue {
        let s = self.state.borrow();
        if !s.ready {
            return json(&policy(
                "client.arena.zoneFallback",
                serde_json::json!([zone]),
            ));
        }
        zone_world(s.enemy.as_ref().unwrap(), &zone, &s.tmp_a, &s.tmp_b);
        call(&s.tmp_a, "project", &[s.camera.clone()]);
        let q = math("screen", &[p(&s.tmp_a, "x"), p(&s.tmp_a, "y")]);
        call(&s.lens, "toScreen", &[n(q[0]), n(q[1])])
    }
}

impl Drop for RustArena {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.dispose();
        }
    }
}

impl ArenaState {
    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        self.running = false;
        self.ready = false;
        self.on_frame = JsValue::NULL;
        // A renderer retains its JS animation callback; cancel before Rust drops that closure.
        let _ = try_call(&self.renderer, "setAnimationLoop", &[JsValue::NULL]);
        // Stop mixer callbacks before releasing the figure graph; late events must
        // not re-enter a disposed arena or keep its state alive.
        for figure in [self.me.as_ref(), self.enemy.as_ref()]
            .into_iter()
            .flatten()
        {
            for resource in [&figure.motion, &figure.flash, &figure.hand] {
                if get(resource, "dispose").is_function() {
                    let _ = try_call(resource, "dispose", &[]);
                }
            }
        }
        for resource in [&self.dust, &self.arms_only] {
            if get(resource, "dispose").is_function() {
                let _ = try_call(resource, "dispose", &[]);
            }
        }
        for resource in [
            &self.timer,
            &self.lens,
            &self.projected_stage,
            &self.env,
            &get(&get(&self.rim, "shadow"), "map"),
        ] {
            if present(resource) {
                let _ = try_call(resource, "dispose", &[]);
            }
        }
        for tier in &self.tiers {
            let _ = try_call(tier, "dispose", &[]);
        }
        self.enemy_shield.dispose();
        self.my_shield.dispose();
        self.hazard.dispose();
        let _ = try_call(&self.renderer, "dispose", &[]);
    }
    fn make_figure(
        &self,
        gltf: &JsValue,
        body_color: u32,
        joints_rough: f64,
        joints_metal: f64,
        lift: u32,
        fall_side: f64,
        near: bool,
    ) -> Figure {
        let h = &self.host;
        let model = sdk(h, "clone", &[get(gltf, "scene")]);
        sdk(h, "dress", &[model.clone(), get(h, "KIT_A")]);
        if near {
            // The first-person cut swaps index buffers: keep them apart from the foe's clone.
            for mesh in meshes(&model) {
                if !boolean(&get(&get(&mesh, "userData"), "ownGeometry")) {
                    set(&mesh, "geometry", &call(&get(&mesh, "geometry"), "clone", &[]));
                }
            }
        }
        let body = make(
            h,
            "MeshStandardMaterial",
            &[object(&[
                ("color", body_color.into()),
                ("roughness", n(0.82)),
                ("metalness", n(0.04)),
                ("emissive", lift.into()),
            ])],
        );
        let joints = make(
            h,
            "MeshStandardMaterial",
            &[object(&[
                ("color", 0x1c1d1d.into()),
                ("roughness", n(joints_rough)),
                ("metalness", n(joints_metal)),
            ])],
        );
        for mesh in meshes(&model) {
            set(&mesh, "castShadow", &true.into());
            set(&mesh, "frustumCulled", &false.into());
            let own = get(&mesh, "material");
            if p(&own, "alphaTest") > 0. {
                set(&own, "alphaToCoverage", &true.into());
            }
            if present(&get(&own, "map")) {
                continue;
            }
            set(
                &mesh,
                "material",
                if string(&get(&own, "name"))
                    .to_ascii_lowercase()
                    .contains("joint")
                {
                    &joints
                } else {
                    &body
                },
            );
        }
        call(&model, "updateMatrixWorld", &[true.into()]);
        let bounds = make(h, "Box3", &[]);
        for mesh in meshes(&model) {
            if super::scene_assets::plain_kit(&string(&get(&mesh, "name"))) {
                call(&bounds, "expandByObject", &[mesh, true.into()]);
            }
        }
        let bones = sdk(h, "findBones", &[model.clone(), bounds.clone()]);
        let size = math(
            "scale",
            &[p(&get(&bounds, "min"), "y"), p(&get(&bounds, "max"), "y")],
        );
        call(&get(&model, "scale"), "multiplyScalar", &[n(size[0])]);
        set(&get(&model, "position"), "y", &n(size[1]));
        let tilt = make(h, "Group", &[]);
        call(&tilt, "add", &[model.clone()]);
        let blob = make(
            h,
            "Mesh",
            &[
                make(h, "PlaneGeometry", &[n(0.95), n(0.7)]),
                make(
                    h,
                    "MeshBasicMaterial",
                    &[object(&[
                        (
                            "map",
                            soft_texture(
                                h,
                                &[
                                    (0., "rgba(0,0,0,.8)"),
                                    (0.55, "rgba(0,0,0,.35)"),
                                    (1., "rgba(0,0,0,0)"),
                                ],
                            ),
                        ),
                        ("transparent", true.into()),
                        ("depthWrite", false.into()),
                        ("fog", false.into()),
                    ])],
                ),
            ],
        );
        set(
            &get(&blob, "rotation"),
            "x",
            &n(-std::f64::consts::FRAC_PI_2),
        );
        set(&get(&blob, "position"), "y", &n(0.004));
        let group = make(h, "Group", &[]);
        call(&group, "add", &[tilt.clone(), blob.clone()]);
        call(&self.scene, "add", &[group.clone()]);
        let mixer = make(h, "AnimationMixer", &[model.clone()]);
        let motion = create(h, "Motion", &[mixer.clone(), get(gltf, "animations")]);
        let feet = create(
            h,
            "Feet",
            &[
                model.clone(),
                get(gltf, "animations"),
                get(&motion, "stances"),
            ],
        );
        let gait = object(&[
            ("walk", gait_in(h, &model, gltf, "walk")),
            ("run", gait_in(h, &model, gltf, "run")),
        ]);
        let port = make(
            h,
            "Mesh",
            &[
                make(h, "SphereGeometry", &[n(0.016), n(12.), n(8.)]),
                make(
                    h,
                    "MeshBasicMaterial",
                    &[object(&[("color", 0x7ff3ff.into()), ("fog", false.into())])],
                ),
            ],
        );
        call(&self.scene, "add", &[port.clone()]);
        if near {
            for node in traverse(&group) {
                call(&get(&node, "layers"), "set", &[n(NEAR)]);
            }
            call(&get(&port, "layers"), "set", &[n(NEAR)]);
            call(&get(&port, "scale"), "setScalar", &[n(0.4)]);
        }
        let flash = create(
            h,
            "Flash",
            &[
                soft_texture(
                    h,
                    &[
                        (0., "rgba(255,250,235,1)"),
                        (0.18, "rgba(255,214,150,.9)"),
                        (0.5, "rgba(255,150,60,.35)"),
                        (1., "rgba(255,110,30,0)"),
                    ],
                ),
                n(if near { NEAR } else { 0. }),
            ],
        );
        for part in js_sys::Array::from(&get(&flash, "parts")).iter() {
            call(&self.scene, "add", &[part]);
        }
        Figure {
            group,
            tilt,
            blob,
            mixer,
            pose: create(h, "Pose", &[model.clone()]),
            motion,
            hand: JsValue::NULL,
            feet,
            gait,
            collapse: create(h, "Collapse", &[model.clone()]),
            hold_out: create(h, "HoldOut", &[model.clone()]),
            flash,
            port,
            bones,
            model,
            kick: [0.; 4],
            look: 1.,
            gun: String::new(),
            fall_t: -1.,
            fall_side,
            worn: String::new(),
        }
    }

    fn make_mirror(&self, gltf: &JsValue) -> Mirror {
        let source = self.enemy.as_ref().unwrap();
        let h = &self.host;
        let model = sdk(h, "clone", &[get(gltf, "scene")]);
        sdk(h, "dress", &[model.clone(), get(h, "KIT_A")]);
        let originals = meshes(&source.model);
        let mut copies: Vec<(JsValue, JsValue)> = vec![];
        for (index, mesh) in meshes(&model).iter().enumerate() {
            let from = originals
                .get(index)
                .map(|v| get(v, "material"))
                .unwrap_or_else(|| get(mesh, "material"));
            let material = if let Some((_, material)) = copies.iter().find(|(v, _)| *v == from) {
                material.clone()
            } else {
                let material = wet_material(&from);
                copies.push((from, material.clone()));
                material
            };
            set(mesh, "material", &material);
            set(mesh, "frustumCulled", &false.into());
            set(mesh, "castShadow", &false.into());
        }
        copy(&get(&model, "scale"), &get(&source.model, "scale"));
        copy(&get(&model, "position"), &get(&source.model, "position"));
        let flip = make(h, "Group", &[]);
        call(&flip, "add", &[model.clone()]);
        set(&get(&flip, "scale"), "y", &n(-1.));
        let tilt = make(h, "Group", &[]);
        call(&tilt, "add", &[flip]);
        let group = make(h, "Group", &[]);
        call(&group, "add", &[tilt.clone()]);
        call(&self.scene, "add", &[group.clone()]);
        let a = traverse(&source.model)
            .into_iter()
            .filter(|v| boolean(&get(v, "isBone")));
        let b = traverse(&model)
            .into_iter()
            .filter(|v| boolean(&get(v, "isBone")));
        Mirror {
            group,
            tilt,
            model,
            pairs: a
                .zip(b)
                .map(|(a, b)| {
                    [
                        get(&a, "position"),
                        get(&a, "quaternion"),
                        get(&b, "position"),
                        get(&b, "quaternion"),
                    ]
                })
                .collect(),
            mats: Rc::new(RefCell::new(copies.into_iter().map(|(_, m)| m).collect())),
        }
    }

    fn give_weapons(&mut self, mine: &JsValue, foe: &JsValue, guns: &JsValue) {
        let h = &self.host;
        let steel = make(
            h,
            "MeshStandardMaterial",
            &[object(&[
                ("color", 0x1b1c1d.into()),
                ("roughness", n(0.45)),
                ("metalness", n(0.55)),
            ])],
        );
        let my_grip = sdk(h, "gripOf", &[get(mine, "scene")]);
        let foe_grip = sdk(h, "gripOf", &[get(foe, "scene")]);
        let me = self.me.as_mut().unwrap();
        let enemy = self.enemy.as_mut().unwrap();
        let mirror = self.mirror.as_ref().unwrap();
        if present(&my_grip) {
            let look =
                Closure::<dyn FnMut(JsValue) -> JsValue>::new(move |m| call(&m, "clone", &[]));
            me.hand = create(
                h,
                "Hand",
                &[
                    me.model.clone(),
                    my_grip,
                    call(&steel, "clone", &[]),
                    object(&[
                        ("layer", n(NEAR)),
                        ("guns", guns.clone()),
                        ("look", look.as_ref().clone()),
                    ]),
                ],
            );
            self.material_callbacks.push(look);
            set(&me.collapse, "gun", &get(&me.hand, "mount"));
        }
        if present(&foe_grip) {
            enemy.hand = create(
                h,
                "Hand",
                &[
                    enemy.model.clone(),
                    foe_grip.clone(),
                    steel.clone(),
                    object(&[("guns", guns.clone())]),
                ],
            );
            set(&enemy.collapse, "gun", &get(&enemy.hand, "mount"));
            let mats = mirror.mats.clone();
            let wetten = Closure::<dyn FnMut(JsValue) -> JsValue>::new(move |m| {
                let w = wet_material(&m);
                mats.borrow_mut().push(w.clone());
                w
            });
            let wet = invoke(wetten.as_ref(), &[steel]);
            self.in_water = create(
                h,
                "Hand",
                &[
                    mirror.model.clone(),
                    foe_grip,
                    wet,
                    object(&[("guns", guns.clone()), ("look", wetten.as_ref().clone())]),
                ],
            );
            self.material_callbacks.push(wetten);
            let hand = enemy.hand.clone();
            let water = self.in_water.clone();
            let hold = Closure::<dyn FnMut(JsValue)>::new(move |weapon: JsValue| {
                call(&hand, "hold", &[weapon.clone()]);
                call(&water, "hold", &[weapon]);
            });
            set(&enemy.motion, "onHold", hold.as_ref());
            self.callbacks.push(hold);
        }
        let hand = me.hand.clone();
        let hold = Closure::<dyn FnMut(JsValue)>::new(move |weapon| {
            if present(&hand) {
                call(&hand, "hold", &[weapon]);
            }
        });
        set(&me.motion, "onHold", hold.as_ref());
        self.callbacks.push(hold);
        self.viewmodel = if present(&me.hand) {
            create(h, "Viewmodel", &[me.model.clone(), me.hand.clone()])
        } else {
            JsValue::NULL
        };
        for (is_me, f) in [(true, me), (false, enemy)] {
            let queue = self.marks.clone();
            let mark = Closure::<dyn FnMut(JsValue)>::new(move |kind| {
                queue.borrow_mut().push((is_me, string(&kind)))
            });
            set(&f.motion, "onMark", mark.as_ref());
            self.callbacks.push(mark);
        }
    }

    fn place(&self) {
        let k = self.k.as_ref().unwrap();
        let c = xyz(&get(&self.camera, "position"));
        let enemy = self.enemy.as_ref().unwrap();
        let me = self.me.as_ref().unwrap();
        let q = math(
            "place",
            &[
                self.depth,
                k.kx,
                k.me[0],
                k.me[1],
                k.me[2],
                k.back,
                if self.fp { 1. } else { 0. },
                c[0],
                c[1],
                c[2],
            ],
        );
        let positions = [
            get(&enemy.group, "position"),
            get(&me.group, "position"),
            get(&self.rim, "position"),
            get(&get(&self.rim, "target"), "position"),
            get(&self.fill, "position"),
            get(&get(&self.fill, "target"), "position"),
            get(&self.me_key, "position"),
            get(&get(&self.me_key, "target"), "position"),
            get(&self.bounce, "position"),
        ];
        for (position, index) in positions.iter().zip([0, 4, 8, 11, 14, 17, 20, 23, 26]) {
            vec_set(position, &q[index..index + 3]);
        }
        let authored_sun = get(&get(&self.frame, "look"), "sun");
        if js_sys::Array::is_array(&authored_sun) && values(&authored_sun).len() == 3 {
            let sun = xyz(&self.sun_dir);
            vec_set(
                &get(&self.rim, "position"),
                &[
                    q[11] + sun[0] * 14.,
                    q[12] + sun[1] * 14.,
                    q[13] + sun[2] * 14.,
                ],
            );
        }
        set(&get(&enemy.group, "rotation"), "y", &n(q[3]));
        set(&get(&me.group, "rotation"), "y", &n(q[7]));
    }
    fn hold_view(&mut self, dt: f64) {
        let me = self.me.as_ref().unwrap();
        if me.fall_t >= 0. {
            set(&me.group, "visible", &false.into());
            return;
        }
        set(&me.group, "visible", &true.into());
        let act = string(&get(&me.motion, "attackKind"));
        let desired = if self.views.get(&act).is_some() {
            &self.views[&act]
        } else {
            &self.views[&self.my_gun]
        };
        let at = desired["at"].as_array().unwrap();
        let c = xyz(&get(&self.camera, "position"));
        let rotation = get(&me.group, "rotation");
        let q = math(
            "view",
            &[
                self.view_at[0],
                self.view_at[1],
                self.view_at[2],
                self.view_tilt,
                at[0].as_f64().unwrap(),
                at[1].as_f64().unwrap(),
                at[2].as_f64().unwrap(),
                desired["tilt"].as_f64().unwrap(),
                if self.view_set { 1. } else { 0. },
                dt,
                p(&rotation, "y"),
                c[0],
                c[1],
                c[2],
            ],
        );
        self.view_at.copy_from_slice(&q[..3]);
        self.view_tilt = q[3];
        self.view_set = true;
        call(
            &rotation,
            "set",
            &[n(-self.view_tilt), get(&rotation, "y"), n(0.), "YXZ".into()],
        );
        vec_set(&get(&me.group, "position"), &q[4..7]);
    }
    fn step(&mut self, dt: f64, speed: f64) {
        let me = self.me.as_ref().unwrap();
        let q = math(
            "bob",
            &[
                self.bob_phase,
                self.bob_k,
                self.bob_depth,
                dt,
                speed,
                me.fall_t,
            ],
        );
        self.bob_phase = q[0];
        self.bob_k = q[1];
        vec_set(&get(&me.tilt, "position"), &[q[2], q[3], 0.]);
    }
    fn tick(&mut self, time: f64) {
        call(&self.timer, "update", &[n(time)]);
        let frame = math(
            "frame",
            &[
                num(&call(&self.timer, "getDelta", &[])),
                self.k.as_ref().unwrap().hz,
            ],
        );
        let dt = frame[0];
        let mut speed = 0.;
        if let Some(walk) = &mut self.walk {
            let q = math(
                if self.stage_enabled {
                    "walkStage"
                } else {
                    "walk"
                },
                &[walk.from, walk.to, walk.t, walk.dur, walk.mine, dt],
            );
            walk.t = q[0];
            self.depth = q[1];
            speed = q[3];
            if self.stage_enabled {
                self.stage_travel = self.walk_origin + q[5];
                call(
                    &self.projected_stage,
                    "setTravel",
                    &[
                        n(self.stage_travel),
                        n(self.k.as_ref().map_or(0., |k| k.kx)),
                    ],
                );
            }
            pace(self.enemy.as_ref().unwrap(), q[2]);
            if q[4] != 0. {
                self.walk = None;
                for f in [self.enemy.as_ref().unwrap(), self.me.as_ref().unwrap()] {
                    if f.fall_t < 0. {
                        call(&f.motion, "stand", &[n(0.35)]);
                    }
                }
            }
        }
        self.place();
        self.step(dt, speed);
        // Each face turns to the other fighter's head; in first person ours is the camera. The positions come from
        // the last drawn frame: refreshing world matrices here would move the camera before the frame places it.
        let three = get(&self.host, "THREE");
        let at = |o: &JsValue| {
            call(&self.tmp_a, "setFromMatrixPosition", &[get(o, "matrixWorld")]);
            xyz(&self.tmp_a)
        };
        let head = |f: &Option<Figure>| at(&get(&f.as_ref().unwrap().bones, "head"));
        let targets = [
            if self.fp {
                at(&self.camera)
            } else {
                head(&self.me)
            },
            head(&self.enemy),
        ];
        for (is_me, f) in [
            (false, self.enemy.as_mut().unwrap()),
            (true, self.me.as_mut().unwrap()),
        ] {
            call(&f.pose, "restore", &[]);
            call(&f.motion, "update", &[n(dt)]);
            call(&f.mixer, "update", &[n(dt)]);
            call(&f.pose, "save", &[]);
            call(&f.motion, "settle", &[]);
            if !(self.fp && is_me) {
                call(
                    &f.hold_out,
                    "apply",
                    &[f.group.clone(), get(&f.motion, "pistolReady")],
                );
                if present(&f.hand) {
                    call(
                        &f.hold_out,
                        "support",
                        &[f.hand.clone(), get(&f.motion, "gunReady")],
                    );
                }
                call(&f.feet, "update", &[n(dt), get(&f.motion, "planted")]);
                // A falling fighter lets the head go with the collapse; a cheek on the rifle or sniper stock keeps
                // the clip's head behind the sight.
                let aim = if matches!(f.gun.as_str(), "rifle" | "sniper") {
                    num(&get(&f.motion, "gunReady"))
                } else {
                    0.
                };
                let goal = if f.fall_t < 0. { 1. - aim } else { 0. };
                f.look += (goal - f.look) * (1. - (-dt * 6.).exp());
                let bone = |name| get(&f.bones, name);
                super::rigging::look_at(
                    &three,
                    &bone("neck"),
                    &bone("head"),
                    &targets[usize::from(is_me)],
                    f.look,
                );
            }
            reactions(&self.host, f, dt);
        }
        // Animation events are queued because Motion.update may invoke them inside the frame borrow.
        let marks = std::mem::take(&mut *self.marks.borrow_mut());
        for (is_me, kind) in marks {
            let f = if is_me {
                self.me.as_ref().unwrap()
            } else {
                self.enemy.as_ref().unwrap()
            };
            if kind == "knife" || kind == "grenade" || !present(&f.hand) {
                continue;
            }
            let target = if is_me {
                self.enemy.as_ref().unwrap()
            } else {
                self.me.as_ref().unwrap()
            };
            zone_world(target, "body", &self.aim, &self.tmp_b);
            let rounds = self.muzzle["rounds"][&kind].as_u64().unwrap_or(1) as u32;
            call(
                &f.flash,
                "fire",
                &[
                    f.hand.clone(),
                    json(&self.muzzle["size"][&kind]),
                    n(rounds as f64),
                    self.aim.clone(),
                    self.suppressed[usize::from(!is_me)].into(),
                ],
            );
            self.flash_of = Some(is_me);
        }
        set(
            &self.wide_k,
            "value",
            &n(if self.fp { self.wide } else { 1. }),
        );
        set(&self.wide_y, "value", &n(frame[1]));
        if self.fp {
            self.hold_view(dt);
            let me = self.me.as_ref().unwrap();
            let act = string(&get(&me.motion, "attackKind"));
            let gun = me.fall_t < 0. && act != "knife" && act != "grenade";
            if present(&self.viewmodel) {
                zone_world(self.enemy.as_ref().unwrap(), "body", &self.aim, &self.tmp_b);
                call(
                    &self.viewmodel,
                    "update",
                    &[
                        n(dt),
                        gun.into(),
                        (gun && !act.is_empty()).into(),
                        self.my_gun.clone().into(),
                        self.camera.clone(),
                        self.aim.clone(),
                    ],
                );
            }
        } else if present(&self.viewmodel) {
            call(&self.viewmodel, "off", &[]);
        }
        let enemy = self.enemy.as_ref().unwrap();
        let me = self.me.as_ref().unwrap();
        if self.reflection_enabled() {
            sync_mirror(self.mirror.as_ref().unwrap(), enemy);
        }
        call(&self.scene, "updateMatrixWorld", &[]);
        place_port(enemy, &self.tmp_a, &self.tmp_b);
        place_port(me, &self.tmp_a, &self.tmp_b);
        if self.fp {
            set(&me.port, "visible", &false.into());
        }
        let lit = num(&call(&enemy.flash, "update", &[n(dt)])).max(num(&call(
            &me.flash,
            "update",
            &[n(dt)],
        )));
        if boolean(&get(&me.flash, "fired")) && present(&self.viewmodel) {
            call(&self.viewmodel, "shot", &[]);
        }
        if let Some(is_me) = self.flash_of {
            if lit > 0. {
                let f = if is_me { me } else { enemy };
                copy(
                    &get(&self.muzzle_light, "position"),
                    &get(&get(&f.flash, "sprite"), "position"),
                );
            }
        }
        set(&self.muzzle_light, "intensity", &n(9. * lit));
        if self.spark_life > 0. {
            let q = math(
                "spark",
                &[
                    self.spark_life,
                    self.spark_max,
                    p(&get(&self.spark, "userData"), "size"),
                    dt,
                ],
            );
            self.spark_life = q[0];
            set(&get(&self.spark, "material"), "opacity", &n(q[1]));
            call(&get(&self.spark, "scale"), "setScalar", &[n(q[2])]);
            set(&self.spark, "visible", &(q[3] != 0.).into());
        }
        let aspect = p(&self.camera, "aspect");
        if let Some(enemy) = &self.enemy {
            let (feet, height) = self.standing(enemy);
            self.enemy_shield.follow(feet, height);
        }
        self.enemy_shield.update(dt, aspect);
        self.my_shield.update(dt, aspect);
        if let Some([near, far]) = self.warned {
            if self.hazard.marking() {
                let camera = xyz(&get(&self.camera, "position"));
                let me = self.lane_point(0.);
                let enemy = self.lane_point(self.dist);
                let (dx, dz) = (enemy[0] - me[0], enemy[2] - me[2]);
                let l = dx.hypot(dz).max(1e-6);
                let edges = [self.lane_point(near), self.lane_point(far)];
                self.hazard
                    .place_mark(edges, [-dz / l, 0., dx / l], camera);
            }
        }
        self.hazard.update(dt);
        for (c, bolt) in self.hazard.take_flashes() {
            if get(&self.lens, "flash").is_function() {
                call(
                    &self.lens,
                    "flash",
                    &[n(c[0]), n(c[1]), n(c[2]), bolt.into()],
                );
            }
        }
        if p(&self.quality, "dust") != 0. {
            call(
                &self.lens,
                "sunDirection",
                &[self.camera.clone(), self.sun_dir.clone()],
            );
            let height = p(
                &call(
                    &self.renderer,
                    "getDrawingBufferSize",
                    &[self.tmp_size.clone()],
                ),
                "y",
            );
            call(
                &self.dust,
                "update",
                &[
                    call(&self.timer, "getElapsed", &[]),
                    n(height * self.k.as_ref().unwrap().fn_),
                    self.sun_dir.clone(),
                    get(&self.lens, "raysTexture"),
                    n(1. / self.depth.max(0.5)),
                    n(height * p(&self.lens, "dof")),
                ],
            );
        }
        call(
            &self.lens,
            "render",
            &[
                self.scene.clone(),
                self.camera.clone(),
                n(NEAR),
                object(&[("enemyDepth", n(self.depth)), ("light", self.rim.clone())]),
            ],
        );
        if self.motion_debug {
            let data = get(&get(&self.renderer, "domElement"), "dataset");
            if present(&data) {
                set(
                    &data,
                    "stageTravel",
                    &format!("{:.6}", self.stage_travel).into(),
                );
                set(&data, "enemyDepth", &format!("{:.6}", self.depth).into());
                set(
                    &data,
                    "projectedStage",
                    &(self.stage_enabled && boolean(&get(&self.projected_stage, "ready")))
                        .to_string()
                        .into(),
                );
                set(&data, "walking", &self.walk.is_some().to_string().into());
            }
        }
    }
    fn resume(&mut self) {
        if self.disposed {
            return;
        }
        self.running = true;
        if !self.ready {
            return;
        }
        call(&self.timer, "reset", &[]);
        call(
            &self.renderer,
            "setAnimationLoop",
            &[self.animation.as_ref().unwrap().as_ref().clone()],
        );
    }
    fn set_quality(&mut self, q: JsValue) {
        self.quality = q.clone();
        self.pace.follow(&q);
        call(&self.lens, "setQuality", &[q.clone()]);
        let shadows = p(&q, "shadow") > 0.;
        set(
            &get(&self.renderer, "shadowMap"),
            "enabled",
            &shadows.into(),
        );
        set(&self.rim, "castShadow", &shadows.into());
        set(&self.catcher, "visible", &shadows.into());
        let shadow = get(&self.rim, "shadow");
        if p(&get(&shadow, "mapSize"), "x") != p(&q, "shadow") && shadows {
            vec_set(
                &get(&shadow, "mapSize"),
                &[p(&q, "shadow"), p(&q, "shadow")],
            );
            dispose(&get(&shadow, "map"));
            set(&shadow, "map", &JsValue::NULL);
        }
        set(&shadow, "radius", &get(&q, "shadowRadius"));
        let env = boolean(&get(&q, "env"));
        let environment = if env { self.env.clone() } else { JsValue::NULL };
        set(&self.scene, "environment", &environment);
        self.apply_lighting();
        call(&self.dust, "setCount", &[get(&q, "dust")]);
        let id = string(&get(&q, "id"));
        self.enemy_shield.set_quality(&id);
        self.my_shield.set_quality(&id);
        let mut mats = vec![get(&self.catcher, "material")];
        if let Some(enemy) = &self.enemy {
            for mesh in meshes(&enemy.model) {
                let m = get(&mesh, "material");
                if !mats.contains(&m) {
                    mats.push(m);
                }
            }
        }
        for m in mats {
            sdk(&self.host, "tune", &[m, q.clone()]);
        }
        if let Some(me) = &self.me {
            let mut seen = vec![];
            for mesh in meshes(&me.model) {
                let m = get(&mesh, "material");
                if seen.contains(&m) {
                    continue;
                }
                seen.push(m.clone());
                sdk(
                    &self.host,
                    "tune",
                    &[m, q.clone(), self.widen.as_ref().clone(), "wide".into()],
                );
            }
        }
        if let Some(mirror) = &self.mirror {
            set(&mirror.group, "visible", &self.reflection_enabled().into());
            for m in mirror.mats.borrow().iter() {
                sdk(
                    &self.host,
                    "tune",
                    &[m.clone(), q.clone(), self.mirror_shader.as_ref().clone()],
                );
            }
        }
        let aniso = p(&q, "anisotropy").min(num(&call(
            &get(&self.renderer, "capabilities"),
            "getMaxAnisotropy",
            &[],
        )));
        let pending: Vec<JsValue> = self
            .tiers
            .iter()
            .map(|t| call(t, "set", &[get(&q, "textures"), n(aniso)]))
            .collect();
        // A missing optional high-resolution map must not reject the game's quality change.
        self.textures_done = Promise::all_settled(&array(&pending));
        self.resize(self.width, self.height, self.view.clone());
    }
    fn set_frame(&mut self, f: JsValue) {
        let changed = string(&get(&self.frame, "id")) != string(&get(&f, "id"));
        self.frame = f.clone();
        self.stage_load_generation = self.stage_load_generation.wrapping_add(1);
        self.stage_enabled = stage_source(&f).is_some() && present(&self.projected_stage);
        if !self.stage_enabled {
            self.stage_loading = Promise::resolve(&JsValue::UNDEFINED);
        }
        if present(&self.projected_stage) {
            call(
                &self.lens,
                "setStage",
                &[if self.stage_enabled {
                    self.projected_stage.clone()
                } else {
                    JsValue::NULL
                }],
            );
        }
        if changed {
            self.stage_travel = 0.;
            self.walk_origin = 0.;
            self.walk = None;
            if present(&self.projected_stage) {
                call(&self.projected_stage, "setTravel", &[n(0.), n(0.)]);
            }
        }
        let k = Calibration::new(&f, self.fp);
        vec_set(&get(&self.camera, "position"), &[0., k.cam_h, 0.]);
        let projection = math(
            "project",
            &[
                p(&self.camera, "near"),
                k.hz,
                k.fn_,
                k.aspect,
                p(&self.camera, "far"),
            ],
        );
        call(
            &get(&self.camera, "projectionMatrix"),
            "makePerspective",
            &projection.iter().copied().map(n).collect::<Vec<_>>(),
        );
        let inverse = get(&self.camera, "projectionMatrixInverse");
        copy(&inverse, &get(&self.camera, "projectionMatrix"));
        call(&inverse, "invert", &[]);
        let look = get(&f, "look");
        if get(&self.lens, "setNightVision").is_function() {
            call(&self.lens, "setNightVision", &[false.into()]);
        }
        self.suppressed = [false; 2];
        set(
            &self.renderer,
            "toneMappingExposure",
            &get(&look, "exposure"),
        );
        call(&self.lens, "setHorizon", &[n(k.hz)]);
        call(
            &self.lens,
            "setBackdrop",
            &[get(&f, "src"), get(&f, "src2x"), get(&f, "depth")],
        );
        call(
            &self.lens,
            "setLook",
            &[get(&look, "lens"), get(&look, "exposure")],
        );
        for (light, key) in [
            (&self.rim, "rim"),
            (&self.fill, "fill"),
            (&self.hemi, "sky"),
        ] {
            call(&get(light, "color"), "set", &[get(&look, key)]);
        }
        call(
            &get(&self.hemi, "groundColor"),
            "set",
            &[get(&look, "ground")],
        );
        set(&self.bounce, "intensity", &get(&look, "puddle"));
        call(
            &get(&get(&self.scene, "fog"), "color"),
            "set",
            &[get(&look, "fog")],
        );
        let authored_sun = get(&look, "sun");
        let sun: Vec<f64> = if js_sys::Array::is_array(&authored_sun) {
            values(&authored_sun).iter().map(num).collect()
        } else {
            vec![]
        };
        if sun.len() == 3
            && sun.iter().all(|v| v.is_finite())
            && sun.iter().map(|v| v * v).sum::<f64>() > 0.001
        {
            vec_set(&self.sun_dir, &sun);
            call(&self.sun_dir, "normalize", &[]);
        } else {
            vec_set(&self.sun_dir, &[0., 0.5, -1.]);
        }
        if get(&self.lens, "setWorldSun").is_function() {
            call(
                &self.lens,
                "setWorldSun",
                &[if sun.len() == 3 {
                    self.sun_dir.clone()
                } else {
                    JsValue::NULL
                }],
            );
        }
        self.apply_lighting();
        if let Some(mirror) = &self.mirror {
            set(&mirror.group, "visible", &self.reflection_enabled().into());
        }
        self.k = Some(k);
    }
    fn reflection_enabled(&self) -> bool {
        boolean(&get(&self.quality, "mirror"))
            && get(&get(&self.frame, "look"), "reflection").as_bool() != Some(false)
    }
    fn light_intensity(&self, key: &str, fallback: f64) -> f64 {
        let value = get(&get(&get(&self.frame, "look"), "lighting"), key);
        if value.as_f64().is_some_and(f64::is_finite) {
            num(&value).clamp(0., 20.)
        } else {
            fallback
        }
    }
    fn apply_lighting(&self) {
        let key = self.light_intensity("key", 5.);
        set(&self.rim, "intensity", &n(self.light_intensity("rim", 3.4)));
        set(
            &self.fill,
            "intensity",
            &n(self.light_intensity("fill", 2.4)),
        );
        set(
            &self.me_key,
            "intensity",
            &n(if self.fp { self.fp_key * key / 5. } else { key }),
        );
        set(&self.back_light, "intensity", &n(key * 0.09));
        let sky = self.light_intensity("sky", 0.6);
        set(
            &self.hemi,
            "intensity",
            &n(sky
                * if boolean(&get(&self.quality, "env")) {
                    0.22 / 0.6
                } else {
                    1.
                }),
        );
        if present(&get(&get(&self.frame, "look"), "lighting")) {
            let look = get(&self.frame, "look");
            call(&get(&self.me_key, "color"), "set", &[get(&look, "fill")]);
            call(&get(&self.back_light, "color"), "set", &[get(&look, "sky")]);
        } else {
            call(&get(&self.me_key, "color"), "set", &[n(0xb4b4b0 as f64)]);
            call(
                &get(&self.back_light, "color"),
                "set",
                &[n(0x8ea3b5 as f64)],
            );
        }
    }
    fn resize(&mut self, width: f64, height: f64, view: JsValue) {
        self.width = width;
        self.height = height;
        self.view = if present(&view) {
            view
        } else {
            object(&[("x", n(0.)), ("y", n(0.)), ("w", n(1.)), ("h", n(1.))])
        };
        let ratio = p(&global(), "devicePixelRatio");
        let size = math(
            "resize",
            &[
                width,
                height,
                if ratio == 0. { 1. } else { ratio },
                p(&self.quality, "maxDpr"),
                p(&self.quality, "maxPixels"),
            ],
        );
        call(&self.renderer, "setPixelRatio", &[n(size[0])]);
        call(
            &self.renderer,
            "setSize",
            &[n(size[1]), n(size[2]), false.into()],
        );
        let px = call(
            &self.renderer,
            "getDrawingBufferSize",
            &[self.tmp_size.clone()],
        );
        call(
            &self.lens,
            "setSize",
            &[
                get(&px, "x"),
                get(&px, "y"),
                n(width),
                n(height),
                self.view.clone(),
            ],
        );
    }
    fn depth_of(&self, d: f64) -> f64 {
        let k = self.k.as_ref().unwrap();
        math("depth", &[d, k.d7, k.back])[0]
    }
    fn stride(&self, f: &Figure, ground: f64, dur: f64) {
        let g = policy(
            "client.arena.stride",
            serde_json::json!([
                ground,
                dur,
                data(&get(&f.gait, "walk")),
                data(&get(&f.gait, "run")),
                p(&get(&f.model, "scale"), "x")
            ]),
        );
        if !g.is_null() {
            call(&f.motion, "walk", &[json(&g["kind"]), json(&g["phase"])]);
        }
    }
    fn first_person(&mut self, on: bool) {
        if on == self.fp {
            return;
        }
        self.finish_walk();
        self.fp = on;
        if present(&self.frame) {
            let k = Calibration::new(&self.frame, on);
            vec_set(&get(&self.camera, "position"), &[0., k.cam_h, 0.]);
            vec_set(&get(&self.camera, "rotation"), &[0., 0., 0.]);
            self.k = Some(k);
        }
        if !self.ready {
            return;
        }
        self.walk = None;
        self.depth = self.depth_of(self.dist);
        let me = self.me.as_ref().unwrap();
        set(&me.blob, "visible", &(!on).into());
        if present(&self.arms_only) {
            invoke(&self.arms_only, &[on.into()]);
        }
        // Restore the helmet's actual visibility before deciding whether it covers the beard.
        super::look::first_person(me.model.clone(), on, super::platform::current_look());
        self.apply_lighting();
        self.view_set = false;
        if !on {
            set(&me.group, "visible", &true.into());
            set(&get(&me.group, "rotation"), "x", &n(0.));
        }
    }
    fn reset(&mut self, distance: f64) {
        self.stage_travel = 0.;
        self.walk_origin = 0.;
        if present(&self.projected_stage) {
            call(&self.projected_stage, "setTravel", &[n(0.), n(0.)]);
        }
        if let Some(k) = &self.k {
            vec_set(&get(&self.camera, "position"), &[0., k.cam_h, 0.]);
            vec_set(&get(&self.camera, "rotation"), &[0., 0., 0.]);
        }
        if !self.ready {
            return;
        }
        self.walk = None;
        self.dist = distance;
        self.spark_life = 0.;
        set(&self.spark, "visible", &false.into());
        self.enemy_shield.stop();
        self.my_shield.stop();
        self.hazard.stop();
        self.warned = None;
        if get(&self.lens, "setDim").is_function() {
            call(&self.lens, "setDim", &[n(1.)]);
        }
        for f in [self.enemy.as_ref().unwrap(), self.me.as_ref().unwrap()] {
            call(&f.flash, "stop", &[]);
        }
        set(&self.muzzle_light, "intensity", &n(0.));
        self.depth = self.depth_of(distance);
        reset_figure(self.enemy.as_mut().unwrap(), 0.);
        reset_figure(self.me.as_mut().unwrap(), 0.43);
    }
    // Finish an interrupted step exactly once before changing view or starting a
    // new turn. Accumulated movement belongs to the player, not opponent range.
    fn finish_walk(&mut self) {
        if !self.stage_enabled {
            return;
        }
        let Some(walk) = self.walk.take() else {
            return;
        };
        if self.stage_enabled {
            self.stage_travel = self.walk_origin + walk.mine;
            call(
                &self.projected_stage,
                "setTravel",
                &[
                    n(self.stage_travel),
                    n(self.k.as_ref().map_or(0., |k| k.kx)),
                ],
            );
        }
        self.depth = walk.to;
        if let Some(enemy) = &self.enemy {
            if enemy.fall_t < 0. {
                call(&enemy.motion, "stand", &[n(0.15)]);
            }
        }
    }
    fn hit(&mut self, who: &str, zone: &str, kind: &str, weapon: &str, broken: bool) {
        if !self.ready {
            return;
        }
        if kind == "blocked" {
            self.shield(who, zone, weapon, broken);
        }
        let f = if who == "enemy" {
            self.enemy.as_mut().unwrap()
        } else {
            self.me.as_mut().unwrap()
        };
        let kick = serde_json::json!({"head":f.kick[0],"body":f.kick[1],"groin":f.kick[2],"legs":f.kick[3]});
        let hit = policy("client.arena.hit", serde_json::json!([kick, zone, kind]));
        for (i, z) in ZONES.iter().enumerate() {
            f.kick[i] = hit["kick"][z].as_f64().unwrap();
        }
        if who != "enemy" || kind == "blocked" {
            return;
        }
        zone_world(
            f,
            hit["zone"].as_str().unwrap(),
            &get(&self.spark, "position"),
            &self.tmp_b,
        );
        if let Some(h) = hit["height"].as_f64() {
            set(&get(&self.spark, "position"), "y", &n(h));
        }
        set(&get(&self.spark, "userData"), "size", &json(&hit["size"]));
        call(
            &get(&get(&self.spark, "material"), "color"),
            "set",
            &[json(&hit["color"])],
        );
        self.spark_max = hit["life"].as_f64().unwrap();
        self.spark_life = self.spark_max;
        set(&self.spark, "visible", &true.into());
    }
}

impl ArenaState {
    /// The dome lights up round whoever was hit; the shot comes from the camera when
    /// I fire (that is where the player sees it from) and from the enemy's chest when they do.
    fn shield(&mut self, who: &str, zone: &str, weapon: &str, broken: bool) {
        let mine = who != "enemy";
        let (target, shooter) = if mine {
            (self.me.as_ref().unwrap(), self.enemy.as_ref().unwrap())
        } else {
            (self.enemy.as_ref().unwrap(), self.me.as_ref().unwrap())
        };
        let (feet, height) = self.standing(target);
        let from = if mine {
            zone_world(shooter, "body", &self.tmp_a, &self.tmp_b);
            xyz(&self.tmp_a)
        } else {
            self.placed(&self.camera)
        };
        zone_world(target, zone, &self.tmp_a, &self.tmp_b);
        let to = xyz(&self.tmp_a);
        let fx = if mine {
            &mut self.my_shield
        } else {
            &mut self.enemy_shield
        };
        fx.fire(feet, height, from, to, weapon, broken);
    }
    // Last frame's matrices, read as they are: updating them here would move the camera
    // and the figures out of step with the frame that is drawn next.
    fn placed(&self, object: &JsValue) -> [f64; 3] {
        call(
            &self.tmp_a,
            "setFromMatrixPosition",
            &[get(object, "matrixWorld")],
        );
        xyz(&self.tmp_a)
    }
    /// Where a fighter's feet are (under the hips, on the floor) and how tall they stand.
    /// A spot on the floor `o` steps ahead of me along the lane; the enemy stands at `self.dist`.
    /// In first person a spot by my feet is kept a little ahead of the lens, so it stays in sight.
    fn lane_point(&self, o: f64) -> [f64; 3] {
        let k = self.k.as_ref().unwrap();
        // Where the walk ends: the hazard strikes after the moves.
        let goal = self.walk.as_ref().map_or(self.depth, |w| w.to);
        let scale = goal / self.depth_of(self.dist).max(1e-6);
        let ahead = |o: f64| {
            let d = self.depth_of(o) * scale;
            [k.kx * d, 0., -d]
        };
        if o >= 1. {
            return ahead(o);
        }
        let one = ahead(1.);
        if self.fp && o > -0.5 {
            let c = xyz(&get(&self.camera, "position"));
            let t = (1.8 / (c[2] - one[2]).max(1.8)).max(o);
            return [c[0] + (one[0] - c[0]) * t, 0., c[2] + (one[2] - c[2]) * t];
        }
        let me = xyz(&get(&self.me.as_ref().unwrap().group, "position"));
        [0, 1, 2].map(|i| me[i] + (one[i] - me[i]) * o)
    }
    /// Starts the strike in the scene; returns the seconds until it lands.
    fn strike(&mut self, r: &serde_json::Value) -> f64 {
        let h = &r["hazard"];
        let Some(kind) = h["kind"].as_str() else {
            return 0.;
        };
        let me_at = r["lane"][0].as_f64().unwrap_or(0.);
        let hits: Vec<u64> = h["hits"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x["side"].as_u64()).collect())
            .unwrap_or_default();
        let side_spot = |side: u64| {
            if side == 0 {
                self.lane_point(0.)
            } else {
                self.lane_point(self.dist)
            }
        };
        // Lightning and a falling roof land on the one they hit; a shell on its mark.
        let spot = match (hits.as_slice(), h["at"].as_f64()) {
            ([one], _) if kind != "shell" => side_spot(*one),
            (_, Some(at)) => self.lane_point(at - me_at),
            ([one, ..], None) => side_spot(*one),
            _ => {
                // The drone found nobody standing still: it goes off beside the lane.
                let mid = self.lane_point(self.dist / 2.);
                let aside = if js_sys::Math::random() < 0.5 { -2.5 } else { 2.5 };
                [mid[0] + aside, 0., mid[2]]
            }
        };
        let camera = xyz(&get(&self.camera, "position"));
        let close = (spot[0] - camera[0]).hypot(spot[2] - camera[2]) < 3.;
        match kind {
            "lightning" => {
                self.hazard.bolt(spot, camera);
                0.03
            }
            "shell" => {
                self.hazard
                    .blast_in(0.5, spot, if close { 0.75 } else { 1. });
                0.5
            }
            "collapse" => {
                self.hazard.collapse(spot, 0.85);
                0.85
            }
            "drone" => {
                let from = [spot[0] + js_sys::Math::random() * 10. - 5., 9., spot[2] - 12.];
                let to = [spot[0], spot[1] + 0.4, spot[2]];
                self.hazard
                    .drone(from, to, 1.05, if close { 0.55 } else { 0.7 });
                1.05
            }
            _ => 0.,
        }
    }
    fn standing(&self, target: &Figure) -> ([f64; 3], f64) {
        let top = self.placed(&get(&target.bones, "top"));
        let hips = self.placed(&get(&target.bones, "hips"));
        let floor = self.placed(&target.group)[1];
        ([hips[0], floor, hips[2]], (top[1] - floor).max(1.2))
    }
}

fn wet_material(material: &JsValue) -> JsValue {
    let wet = call(material, "clone", &[]);
    call(&get(&wet, "color"), "multiplyScalar", &[n(0.62)]);
    set(&wet, "transparent", &true.into());
    set(&wet, "opacity", &n(0.5));
    set(&wet, "depthWrite", &false.into());
    wet
}
fn gait_in(host: &JsValue, model: &JsValue, gltf: &JsValue, name: &str) -> JsValue {
    values(&get(gltf, "animations"))
        .into_iter()
        .find(|c| string(&get(c, "name")) == name)
        .map(|clip| sdk(host, "gaitOf", &[model.clone(), clip]))
        .unwrap_or(JsValue::NULL)
}
fn pace(f: &Figure, speed: f64) {
    let name = string(&get(&f.motion, "gait"));
    let gait = get(&f.gait, &name);
    if present(&gait) {
        call(
            &f.motion,
            "pace",
            &[n(math(
                "pace",
                &[speed, p(&gait, "speed"), p(&get(&f.model, "scale"), "x")],
            )[0])],
        );
    }
}
fn reset_figure(f: &mut Figure, offset: f64) {
    f.fall_t = -1.;
    f.look = 1.;
    vec_set(&get(&f.tilt, "rotation"), &[0., 0., 0.]);
    vec_set(&get(&f.tilt, "position"), &[0., 0., 0.]);
    vec_set(&get(&f.blob, "scale"), &[1., 1., 1.]);
    set(&get(&f.blob, "position"), "x", &n(0.));
    f.kick = [0.; 4];
    call(&f.motion, "reset", &[n(offset)]);
}
fn sync_mirror(mirror: &Mirror, source: &Figure) {
    copy(
        &get(&mirror.group, "position"),
        &get(&source.group, "position"),
    );
    copy(
        &get(&mirror.group, "rotation"),
        &get(&source.group, "rotation"),
    );
    let r = get(&source.tilt, "rotation");
    vec_set(
        &get(&mirror.tilt, "rotation"),
        &[-p(&r, "x"), p(&r, "y"), -p(&r, "z")],
    );
    for [ap, aq, bp, bq] in &mirror.pairs {
        copy(bp, ap);
        copy(bq, aq);
    }
}
fn reactions(host: &JsValue, f: &mut Figure, dt: f64) {
    let k = f.kick;
    let q = math(
        "reactions",
        &[
            k[0],
            k[1],
            k[2],
            k[3],
            dt,
            f.fall_t,
            f.fall_side,
            p(host, "FALL_TIME"),
        ],
    );
    for (i, bones) in [
        ["head", "neck"],
        ["spine2", "spine1"],
        ["spine", "spine1"],
        ["rUpLeg", "rLeg"],
    ]
    .iter()
    .enumerate()
    {
        if k[i] != 0. {
            for (j, bone) in bones.iter().enumerate() {
                sdk(host, "pitch", &[get(&f.bones, bone), n(q[i * 2 + j])]);
            }
        }
    }
    f.kick.copy_from_slice(&q[8..12]);
    if f.fall_t >= 0. {
        if f.fall_t == 0. {
            call(&f.collapse, "begin", &[f.group.clone(), n(f.fall_side)]);
        }
        f.fall_t = q[12];
        call(&f.collapse, "apply", &[n(q[13])]);
        vec_set(&get(&f.blob, "scale"), &[q[14], 1., 1.15]);
        set(&get(&f.blob, "position"), "x", &n(q[15]));
    }
}
fn place_port(f: &Figure, a: &JsValue, b: &JsValue) {
    call(&get(&f.bones, "head"), "getWorldPosition", &[a.clone()]);
    call(&get(&f.bones, "top"), "getWorldPosition", &[b.clone()]);
    let x = xyz(a);
    let y = xyz(b);
    let q = math(
        "port",
        &[
            x[0],
            x[1],
            x[2],
            y[0],
            y[1],
            y[2],
            p(&get(&f.group, "rotation"), "y"),
        ],
    );
    call(&get(&f.port, "position"), "fromArray", &[arr(&q)]);
    set(
        &f.port,
        "visible",
        &(f.fall_t < 0. && boolean(&get(&f.group, "visible"))).into(),
    );
}
fn zone_world(f: &Figure, zone: &str, out: &JsValue, tmp: &JsValue) {
    let index = ZONES.iter().position(|z| *z == zone).unwrap_or(1);
    let pairs = [
        ["head", "top"],
        ["spine1", "spine2"],
        ["hips", "hips"],
        ["rLeg", "rUpLeg"],
    ];
    call(
        &get(&f.bones, pairs[index][0]),
        "getWorldPosition",
        &[out.clone()],
    );
    call(
        &get(&f.bones, pairs[index][1]),
        "getWorldPosition",
        &[tmp.clone()],
    );
    let a = xyz(out);
    let b = xyz(tmp);
    call(
        out,
        "fromArray",
        &[arr(&math(
            "zone",
            &[a[0], a[1], a[2], b[0], b[1], b[2], index as f64],
        ))],
    );
}

fn load_stage(state: &Rc<RefCell<ArenaState>>) {
    let (request, generation) = {
        let s = state.borrow();
        if s.disposed || !s.stage_enabled {
            return;
        }
        (
            try_call(&s.projected_stage, "load", &{
                let (manifest, variant) = stage_source(&s.frame).unwrap();
                if present(&variant) {
                    vec![manifest.into(), variant]
                } else {
                    vec![manifest.into()]
                }
            }),
            s.stage_load_generation,
        )
    };
    let weak = Rc::downgrade(state);
    let loading = future_to_promise(async move {
        let result = match request {
            Ok(request) => JsFuture::from(Promise::resolve(&request)).await,
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            if let Some(state) = weak.upgrade() {
                let mut s = state.borrow_mut();
                if !s.disposed && s.stage_load_generation == generation && s.stage_enabled {
                    s.stage_enabled = false;
                    call(&s.lens, "setStage", &[JsValue::NULL]);
                    let _ = try_call(
                        &get(&global(), "console"),
                        "warn",
                        &["Projected stage unavailable; using backdrop".into(), error],
                    );
                }
            }
        }
        Ok(JsValue::UNDEFINED)
    });
    state.borrow_mut().stage_loading = loading;
}

fn make_env(state: &Rc<RefCell<ArenaState>>, src: String) {
    let request = {
        let mut s = state.borrow_mut();
        if s.disposed || s.env_src == src {
            return;
        }
        s.env_src = src.clone();
        call(
            &make(&s.host, "TextureLoader", &[]),
            "loadAsync",
            &[src.clone().into()],
        )
    };
    let weak = Rc::downgrade(state);
    spawn_local(async move {
        let Ok(tex) = JsFuture::from(Promise::resolve(&request)).await else {
            return;
        };
        let Some(state) = weak.upgrade() else {
            dispose(&tex);
            return;
        };
        let mut s = state.borrow_mut();
        if s.disposed || s.env_src != src {
            dispose(&tex);
            return;
        }
        let three = get(&s.host, "THREE");
        set(
            &tex,
            "mapping",
            &get(&three, "EquirectangularReflectionMapping"),
        );
        set(&tex, "colorSpace", &get(&three, "SRGBColorSpace"));
        let pmrem = make(&s.host, "PMREMGenerator", &[s.renderer.clone()]);
        let env = get(
            &call(&pmrem, "fromEquirectangular", &[tex.clone()]),
            "texture",
        );
        dispose(&pmrem);
        dispose(&tex);
        dispose(&s.env);
        s.env = env.clone();
        if boolean(&get(&s.quality, "env")) {
            set(&s.scene, "environment", &env);
        }
    });
}
