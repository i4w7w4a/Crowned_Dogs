//! Arena ownership and frame orchestration. Three.js objects are opaque renderer handles.
use super::bridge::*;
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
    fall_t: f64,
    fall_side: f64,
}
struct Mirror {
    group: JsValue,
    tilt: JsValue,
    model: JsValue,
    pairs: Vec<(JsValue, JsValue)>,
    mats: Rc<RefCell<Vec<JsValue>>>,
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
    bounce: JsValue,
    spark: JsValue,
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
            bounce,
            spark,
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
                if !s.running || !s.ready {
                    return;
                }
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
            set(&me.blob, "visible", &(!s.fp).into());
            s.arms_only = sdk(&s.host, "armsOnly", &[s.me.as_ref().unwrap().model.clone()]);
            invoke(&s.arms_only, &[s.fp.into()]);
            set(&s.me_key, "intensity", &n(if s.fp { s.fp_key } else { 5. }));
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
    /// Compiles every shader of the scene ahead of the first frame.
    pub fn warm(&self) -> Promise {
        let s = self.state.borrow();
        if !get(&s.renderer, "compileAsync").is_function() {
            return Promise::resolve(&JsValue::UNDEFINED);
        }
        match try_call(&s.renderer, "compileAsync", &[s.scene.clone(), s.camera.clone()]) {
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
            s.enemy.as_ref().unwrap()
        } else {
            s.me.as_ref().unwrap()
        };
        call(&f.motion, "arm", &[gun.clone().into()]);
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
    pub fn hit(&self, who: String, zone: String, kind: String) {
        self.state.borrow_mut().hit(&who, &zone, &kind);
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
        call(&bounds, "setFromObject", &[model.clone(), true.into()]);
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
        call(&self.scene, "add", &[get(&flash, "sprite")]);
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
            fall_t: -1.,
            fall_side,
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
            pairs: a.zip(b).collect(),
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
                call(&f.feet, "update", &[n(dt), get(&f.motion, "planted")]);
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
            call(
                &f.flash,
                "fire",
                &[f.hand.clone(), json(&self.muzzle[&kind])],
            );
            self.flash_of = Some(is_me);
            if is_me && present(&self.viewmodel) {
                call(&self.viewmodel, "shot", &[]);
            }
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
        sync_mirror(self.mirror.as_ref().unwrap(), enemy);
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
        if let Some(is_me) = self.flash_of {
            if lit > 0. {
                let f = if is_me { me } else { enemy };
                copy(
                    &get(&self.muzzle_light, "position"),
                    &get(&get(&f.flash, "sprite"), "position"),
                );
            }
        }
        set(&self.muzzle_light, "intensity", &n(6. * lit));
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
        set(&self.hemi, "intensity", &n(if env { 0.22 } else { 0.6 }));
        call(&self.dust, "setCount", &[get(&q, "dust")]);
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
            set(&mirror.group, "visible", &get(&q, "mirror"));
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
        self.stage_enabled = string(&get(&f, "id")) == "hall1" && present(&self.projected_stage);
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
        self.k = Some(k);
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
        set(
            &self.me_key,
            "intensity",
            &n(if on { self.fp_key } else { 5. }),
        );
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
    fn hit(&mut self, who: &str, zone: &str, kind: &str) {
        if !self.ready {
            return;
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
        if who != "enemy" {
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
    for (a, b) in &mirror.pairs {
        copy(&get(b, "position"), &get(a, "position"));
        copy(&get(b, "quaternion"), &get(a, "quaternion"));
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
            try_call(
                &s.projected_stage,
                "load",
                &["img/battle-stage/manifest.json".into()],
            ),
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
                if !s.disposed
                    && s.stage_load_generation == generation
                    && string(&get(&s.frame, "id")) == "hall1"
                {
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
