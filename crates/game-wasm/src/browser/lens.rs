//! Body-camera render graph. Rust owns targets, quality transitions and asynchronous image lifetimes.
use super::bridge::*;
use js_sys::{Float64Array, Promise, Uint8ClampedArray};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, future_to_promise, spawn_local};

fn n(v: f64) -> JsValue {
    v.into()
}
fn prop(o: &JsValue, key: &str) -> f64 {
    num(&get(o, key))
}
fn uniform(m: &JsValue, key: &str) -> JsValue {
    get(&get(&get(m, "uniforms"), key), "value")
}
fn put(m: &JsValue, key: &str, v: JsValue) {
    set(&get(&get(m, "uniforms"), key), "value", &v);
}
fn make(three: &JsValue, name: &str, args: &[JsValue]) -> JsValue {
    construct(&get(three, name), args)
}
fn dispose(v: &JsValue) {
    if present(v) {
        call(v, "dispose", &[]);
    }
}
fn texture(v: &JsValue) -> JsValue {
    if present(v) {
        get(v, "texture")
    } else {
        JsValue::NULL
    }
}
fn compute(kind: &str, input: &[f64]) -> Vec<f64> {
    game_client::arena_compute(&format!("arena.lens.{kind}"), input).expect("lens arithmetic")
}

struct LensState {
    disposed: bool,
    three: JsValue,
    renderer: JsValue,
    quad: JsValue,
    clock: JsValue,
    float: JsValue,
    fig: JsValue,
    near: JsValue,
    comp: JsValue,
    haze: JsValue,
    rays_a: JsValue,
    rays_b: JsValue,
    compose: JsValue,
    haze_mat: JsValue,
    mask: JsValue,
    blur: JsValue,
    lens: JsValue,
    quality: JsValue,
    look: JsValue,
    bg: JsValue,
    stage: JsValue,
    depth_map: JsValue,
    sun: JsValue,
    world_sun: JsValue,
    one: String,
    two: Option<String>,
    depth_src: String,
    loading: String,
    horizon: f64,
    night_vision: bool,
    heat: bool,
    sun_glare: bool,
    /// Near and far inverse depth of the shimmer, then its strength (the
    /// shader gets the horizon as a fourth value).
    heat_depth: [f64; 3],
    /// The match weather as the world clock describes it (`client.world.fx`).
    weather: serde_json::Value,
    /// Lightning strikes drawn so far, and whether one is lit right now.
    strikes: std::cell::Cell<(u32, bool)>,
    shader_features: u8,
    /// Minimal quality: 8-bit frame, the gun and the hands drawn into the fighters' picture.
    minimal: bool,
    /// A quad behind the gun and the hands that marks their pixels (alpha one half) for the compose.
    mark: Option<(JsValue, JsValue)>,
    px: [f64; 2],
    css: [f64; 2],
    view: [f64; 4],
}

#[wasm_bindgen]
pub struct RustLens {
    state: Rc<RefCell<LensState>>,
}

#[wasm_bindgen]
impl RustLens {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, renderer: JsValue) -> RustLens {
        let three = get(&host, "THREE");
        let ext = get(&renderer, "extensions");
        let half = boolean(&call(&ext, "has", &["EXT_color_buffer_float".into()]))
            || boolean(&call(&ext, "has", &["EXT_color_buffer_half_float".into()]));
        let float = get(
            &three,
            if half {
                "HalfFloatType"
            } else {
                "UnsignedByteType"
            },
        );
        let target = |opts: JsValue| make(&three, "WebGLRenderTarget", &[n(1.), n(1.), opts]);
        let fig = target(object(&[("samples", n(4.)), ("type", float.clone())]));
        let near = target(object(&[("samples", n(4.)), ("type", float.clone())]));
        let comp = target(object(&[
            ("type", float.clone()),
            ("generateMipmaps", true.into()),
            ("minFilter", get(&three, "LinearMipmapLinearFilter")),
        ]));
        let vector = |x, y| make(&three, "Vector2", &[n(x), n(y)]);
        let matrix = || make(&three, "Matrix4", &[]);
        let color = || make(&three, "Color", &[]);
        let sun = vector(0.8, 0.8);
        // the name picks each pass's node twin on WebGPU (apps/web/src/arena/gpu.ts)
        let shader = |name: &str, fragment: &str, entries: &[(&str, JsValue)]| {
            let uniforms = object(&[]);
            for (key, value) in entries {
                set(&uniforms, key, &object(&[("value", value.clone())]));
            }
            make(
                &three,
                "ShaderMaterial",
                &[object(&[
                    ("name", name.into()),
                    (
                        "vertexShader",
                        include_str!("shaders/vertexShader.glsl").into(),
                    ),
                    ("fragmentShader", fragment.into()),
                    ("uniforms", uniforms),
                    ("depthTest", false.into()),
                    ("depthWrite", false.into()),
                    ("toneMapped", false.into()),
                ])],
            )
        };
        let compose = shader(
            "compose",
            include_str!("shaders/composeShader.glsl"),
            &[
                ("tFig", texture(&fig)),
                ("tNear", texture(&near)),
                ("tBg", JsValue::NULL),
                ("tHaze", JsValue::NULL),
                ("tDepth", JsValue::NULL),
                ("hasDepth", n(0.)),
                ("focusInv", n(0.125)),
                ("dof", n(0.)),
                ("dofNear", n(0.)),
                ("bgSize", vector(1., 1.)),
                ("nearSize", vector(1., 1.)),
                ("hazeSize", vector(1., 1.)),
                ("hasBg", n(0.)),
                ("exposure", n(1.)),
                ("saturation", n(1.)),
                ("tint", make(&three, "Color", &[n(0xffffff as f64)])),
            ],
        );
        let sun_dir = make(&three, "Vector3", &[n(0.), n(0.5), n(-1.)]);
        call(&sun_dir, "normalize", &[]);
        let haze_mat = shader(
            "haze",
            include_str!("shaders/hazeShader.glsl"),
            &[
                ("invProj", matrix()),
                ("camWorld", matrix()),
                ("camPos", make(&three, "Vector3", &[])),
                ("tFig", texture(&fig)),
                ("enemyDist", n(10.)),
                ("tDepth", JsValue::NULL),
                ("hasDepth", n(0.)),
                ("tShadow", JsValue::NULL),
                ("shadowMatrix", matrix()),
                ("hasShadow", n(0.)),
                ("sunDir", sun_dir),
                ("sunColor", color()),
                ("skyColor", color()),
                ("density", n(0.)),
                ("time", n(0.)),
            ],
        );
        let mask = shader(
            "raysMask",
            include_str!("shaders/raysMaskShader.glsl"),
            &[
                ("tComp", texture(&comp)),
                ("tFig", texture(&fig)),
                ("lod", n(1.)),
                ("horizon", n(0.6)),
                ("threshold", n(0.72)),
            ],
        );
        let blur = shader(
            "raysBlur",
            include_str!("shaders/raysBlurShader.glsl"),
            &[
                ("tSrc", JsValue::NULL),
                ("light", sun.clone()),
                ("reach", n(0.9)),
                ("decay", n(0.97)),
            ],
        );
        let lens = shader(
            "lens",
            include_str!("shaders/lensShader.glsl"),
            &[
                ("tComp", texture(&comp)),
                ("tRays", JsValue::NULL),
                ("compSize", vector(1., 1.)),
                ("outSize", vector(1., 1.)),
                ("ssaa", n(1.)),
                ("hasBg", n(0.)),
                ("rays", n(0.)),
                ("glowLod", n(0.)),
                ("center", vector(0.5, 0.5)),
                ("halfView", vector(0.5, 0.5)),
                ("aspect", n(2. / 3.)),
                ("smear", n(0.)),
                ("soft", n(0.)),
                ("barrel", n(0.)),
                ("vignette", n(0.)),
                ("glow", n(0.)),
                ("contrast", n(0.)),
            ],
        );
        let quad = construct(&get(&host, "FullScreenQuad"), &[]);
        let clock = make(&three, "Timer", &[]);
        RustLens {
            state: Rc::new(RefCell::new(LensState {
                disposed: false,
                three,
                renderer,
                quad,
                clock,
                float,
                fig,
                near,
                comp,
                haze: JsValue::NULL,
                rays_a: JsValue::NULL,
                rays_b: JsValue::NULL,
                compose,
                haze_mat,
                mask,
                blur,
                lens,
                quality: JsValue::NULL,
                look: JsValue::NULL,
                bg: JsValue::NULL,
                stage: JsValue::NULL,
                depth_map: JsValue::NULL,
                sun,
                world_sun: JsValue::NULL,
                one: String::new(),
                two: None,
                depth_src: String::new(),
                loading: String::new(),
                horizon: 0.4,
                night_vision: false,
                heat: false,
                sun_glare: false,
                heat_depth: [0.2, 0.1, 1.0],
                weather: serde_json::Value::Null,
                strikes: Default::default(),
                shader_features: 0,
                minimal: false,
                mark: None,
                px: [1., 1.],
                css: [1., 1.],
                view: [0., 0., 1., 1.],
            })),
        }
    }

    #[wasm_bindgen(js_name = setStage)]
    pub fn set_stage(&self, stage: JsValue) {
        let mut s = self.state.borrow_mut();
        if s.disposed {
            return;
        }
        s.stage = stage;
        if present(&s.stage) {
            let _ = try_call(&s.stage, "setStill", &[s.minimal.into()]);
        }
        // The stage owns its resources. Detaching it restores the painted hall
        // without transferring or disposing either set of textures.
        if !present(&s.stage) {
            put(&s.compose, "tBg", s.bg.clone());
            put(&s.compose, "hasBg", n(if present(&s.bg) { 1. } else { 0. }));
            if present(&s.bg) {
                let image = get(&s.bg, "image");
                call(
                    &uniform(&s.compose, "bgSize"),
                    "set",
                    &[get(&image, "width"), get(&image, "height")],
                );
            }
            for m in [&s.compose, &s.haze_mat] {
                put(m, "tDepth", s.depth_map.clone());
                put(
                    m,
                    "hasDepth",
                    n(if present(&s.depth_map) { 1. } else { 0. }),
                );
            }
        }
    }

    #[wasm_bindgen(js_name = setBackdrop)]
    pub fn set_backdrop(&self, one: String, two: Option<String>, depth: Option<String>) {
        {
            let mut s = self.state.borrow_mut();
            if s.disposed {
                return;
            }
            s.one = one;
            s.two = two;
        }
        load_backdrop(&self.state);
        let url = depth.unwrap_or_default();
        let request = {
            let mut s = self.state.borrow_mut();
            if s.depth_src == url {
                return;
            }
            s.depth_src = url.clone();
            if url.is_empty() {
                s.set_depth(JsValue::NULL);
                return;
            }
            call(
                &make(&s.three, "TextureLoader", &[]),
                "loadAsync",
                &[url.clone().into()],
            )
        };
        let weak = Rc::downgrade(&self.state);
        spawn_local(async move {
            let Ok(tex) = JsFuture::from(Promise::resolve(&request)).await else {
                return;
            };
            let Some(state) = weak.upgrade() else {
                dispose(&tex);
                return;
            };
            let mut s = state.borrow_mut();
            if s.disposed || s.depth_src != url {
                dispose(&tex);
                return;
            }
            set(&tex, "colorSpace", &get(&s.three, "NoColorSpace"));
            set(&tex, "generateMipmaps", &false.into());
            set(&tex, "minFilter", &get(&s.three, "LinearFilter"));
            s.set_depth(tex);
        });
    }

    #[wasm_bindgen(js_name = setHorizon)]
    pub fn set_horizon(&self, value: f64) {
        self.state.borrow_mut().horizon = value;
    }

    #[wasm_bindgen(js_name = setLook)]
    pub fn set_look(&self, look: JsValue, exposure: f64) {
        let mut s = self.state.borrow_mut();
        s.look = look.clone();
        s.size_targets();
        for key in ["smear", "soft", "barrel", "vignette", "glow", "contrast"] {
            put(&s.lens, key, get(&look, key));
        }
        for key in ["saturation", "dof", "dofNear"] {
            put(&s.compose, key, get(&look, key));
        }
        call(&uniform(&s.compose, "tint"), "set", &[get(&look, "tint")]);
        put(&s.compose, "exposure", n(exposure));
        let color = make(&s.three, "Color", &[get(&look, "hazeColor")]);
        for (key, factor) in [("sunColor", 0.55), ("skyColor", 0.16)] {
            let c = uniform(&s.haze_mat, key);
            call(&c, "copy", &[color.clone()]);
            call(&c, "multiplyScalar", &[n(factor)]);
        }
        let haze = prop(&look, "haze") * s.weather_haze();
        put(&s.haze_mat, "density", n(haze));
    }
    /// White-phosphor gain is available only when the canonical match grants NVG.
    /// Keep the normal shader byte-for-byte intact when it is switched off.
    #[wasm_bindgen(js_name = setNightVision)]
    pub fn set_night_vision(&self, enabled: bool) {
        let mut s = self.state.borrow_mut();
        if s.disposed || s.night_vision == enabled {
            return;
        }
        s.night_vision = enabled;
        s.refresh_battle_shader();
    }
    #[wasm_bindgen(js_name = setWorldSun)]
    pub fn set_world_sun(&self, direction: JsValue) {
        let mut s = self.state.borrow_mut();
        s.world_sun = if present(&direction) {
            call(&direction, "clone", &[])
        } else {
            JsValue::NULL
        };
    }
    #[wasm_bindgen(js_name = setBattleAtmosphere)]
    pub fn set_battle_atmosphere(
        &self,
        heat: bool,
        sun_glare: bool,
        near_depth: f64,
        far_depth: f64,
        heat_power: Option<f64>,
    ) {
        let mut s = self.state.borrow_mut();
        if s.disposed {
            return;
        }
        s.heat = heat;
        s.sun_glare = sun_glare;
        if near_depth.is_finite()
            && far_depth.is_finite()
            && near_depth > far_depth
            && far_depth > 0.
        {
            s.heat_depth = [near_depth, far_depth, s.heat_depth[2]];
        }
        if let Some(power) = heat_power.filter(|p| p.is_finite()) {
            s.heat_depth[2] = power;
        }
        s.refresh_battle_shader();
    }

    /// Weather is drawn into the air of the battle picture; clear removes it and
    /// leaves the compose shader exactly as it was.
    #[wasm_bindgen(js_name = setWeather)]
    pub fn set_weather(&self, fx: JsValue) {
        let mut s = self.state.borrow_mut();
        if s.disposed {
            return;
        }
        s.weather = js_sys::JSON::stringify(&fx)
            .ok()
            .and_then(|t| t.as_string())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        if present(&s.look) {
            let haze = prop(&s.look, "haze") * s.weather_haze();
            put(&s.haze_mat, "density", n(haze));
        }
        s.refresh_battle_shader();
    }

    /// How many lightning strikes have flashed; the battle rolls thunder after each.
    #[wasm_bindgen(getter)]
    pub fn strikes(&self) -> u32 {
        self.state.borrow().strikes.get().0
    }

    #[wasm_bindgen(getter)]
    pub fn dof(&self) -> f64 {
        let s = self.state.borrow();
        if num(&uniform(&s.compose, "hasDepth")) != 0. {
            num(&uniform(&s.compose, "dof"))
        } else {
            0.
        }
    }

    #[wasm_bindgen(js_name = setQuality)]
    pub fn set_quality(&self, q: JsValue) {
        {
            let mut s = self.state.borrow_mut();
            let minimal = string(&get(&q, "id")) == "minimal";
            if !present(&s.quality)
                || prop(&s.quality, "msaa") != prop(&q, "msaa")
                || minimal != s.minimal
            {
                // Minimal quality keeps the frame in 8 bits: sRGB storage holds the dark tones, and
                // only highlights above white clip.
                let (kind, space) = if minimal {
                    (get(&s.three, "UnsignedByteType"), "SRGBColorSpace")
                } else {
                    (s.float.clone(), "NoColorSpace")
                };
                for t in [&s.fig, &s.comp] {
                    set(&get(t, "texture"), "type", &kind);
                }
                set(&get(&s.fig, "texture"), "colorSpace", &get(&s.three, space));
                // a lens without smear or glow (no taps) reads the frame only at full size, so
                // its mips are not rebuilt every frame
                let mips = prop(&q, "taps") > 0.;
                let comp = get(&s.comp, "texture");
                set(&comp, "generateMipmaps", &mips.into());
                let filter = if mips { "LinearMipmapLinearFilter" } else { "LinearFilter" };
                set(&comp, "minFilter", &get(&s.three, filter));
                set(&s.fig, "samples", &get(&q, "msaa"));
                set(&s.near, "samples", &n(prop(&q, "msaa").min(4.)));
                for t in [&s.fig, &s.near, &s.comp] {
                    dispose(t);
                }
            }
            s.minimal = minimal;
            if minimal && s.mark.is_none() {
                s.mark = Some(hand_mark(&s.three));
            }
            if present(&s.stage) {
                let _ = try_call(&s.stage, "setStill", &[minimal.into()]);
            }
            s.quality = q.clone();
            s.refresh_battle_shader();
            let rt = || {
                make(
                    &s.three,
                    "WebGLRenderTarget",
                    &[
                        n(1.),
                        n(1.),
                        object(&[("type", s.float.clone()), ("depthBuffer", false.into())]),
                    ],
                )
            };
            if prop(&q, "haze") > 0. && !present(&s.haze) {
                s.haze = rt();
            }
            if prop(&q, "haze") == 0. && present(&s.haze) {
                dispose(&s.haze);
                s.haze = JsValue::NULL;
            }
            if prop(&q, "rays") > 0. && !present(&s.rays_a) {
                s.rays_a = make(
                    &s.three,
                    "WebGLRenderTarget",
                    &[
                        n(1.),
                        n(1.),
                        object(&[("type", s.float.clone()), ("depthBuffer", false.into())]),
                    ],
                );
                s.rays_b = make(
                    &s.three,
                    "WebGLRenderTarget",
                    &[
                        n(1.),
                        n(1.),
                        object(&[("type", s.float.clone()), ("depthBuffer", false.into())]),
                    ],
                );
            }
            if prop(&q, "rays") == 0. && present(&s.rays_a) {
                dispose(&s.rays_a);
                dispose(&s.rays_b);
                s.rays_a = JsValue::NULL;
                s.rays_b = JsValue::NULL;
            }
            let defines = object(&[("DOF_TAPS", get(&q, "dofTaps"))]);
            if boolean(&get(&q, "sharpBackdrop")) {
                set(&defines, "SHARP_BG", &n(1.));
            }
            if prop(&q, "haze") > 0. {
                set(&defines, "HAZE", &n(1.));
            }
            set_defines(&s.compose, defines);
            let defines = object(&[("TAPS", get(&q, "taps"))]);
            if prop(&q, "rays") > 0. {
                set(&defines, "RAYS", &n(1.));
            }
            set_defines(&s.lens, defines);
            set_defines(&s.blur, object(&[("STEPS", n(prop(&q, "rays").max(1.)))]));
            set_defines(
                &s.haze_mat,
                object(&[
                    ("STEPS", n(prop(&q, "haze").max(1.))),
                    ("OCTAVES", n(if prop(&q, "haze") >= 48. { 3. } else { 2. })),
                ]),
            );
            put(&s.compose, "tHaze", texture(&s.haze));
            put(&s.lens, "tRays", texture(&s.rays_a));
            s.size_targets();
        }
        load_backdrop(&self.state);
    }

    #[wasm_bindgen(js_name = setSize)]
    pub fn set_size(
        &self,
        width: f64,
        height: f64,
        css_width: f64,
        css_height: f64,
        view: JsValue,
    ) {
        let mut s = self.state.borrow_mut();
        s.px = [width, height];
        s.css = [css_width, css_height];
        s.view = [
            prop(&view, "x"),
            prop(&view, "y"),
            prop(&view, "w"),
            prop(&view, "h"),
        ];
        s.size_targets();
        let [x, y, w, h] = s.view;
        call(
            &uniform(&s.lens, "center"),
            "set",
            &[n(x + w / 2.), n(1. - y - h / 2.)],
        );
        call(
            &uniform(&s.lens, "halfView"),
            "set",
            &[n(w / 2.), n(h / 2.)],
        );
        put(&s.lens, "aspect", n(w * width / (h * height)));
    }

    #[wasm_bindgen(js_name = toScreen)]
    pub fn to_screen(&self, x: f64, y: f64) -> Vec<f64> {
        let s = self.state.borrow();
        compute(
            "screen",
            &[
                x,
                y,
                s.view[0],
                s.view[1],
                s.view[2],
                s.view[3],
                s.px[0],
                s.px[1],
                prop(&s.look, "barrel"),
            ],
        )
    }

    pub fn render(&self, scene: JsValue, camera: JsValue, near: f64, haze: JsValue) {
        let s = self.state.borrow();
        if s.disposed {
            return;
        }
        let r = &s.renderer;
        let layers = get(&camera, "layers");
        let shadow_map = get(r, "shadowMap");
        let mask = get(&layers, "mask");
        let auto = get(&shadow_map, "autoUpdate");
        let stage = if present(&s.stage) && boolean(&get(&s.stage, "ready")) {
            s.stage.clone()
        } else {
            JsValue::NULL
        };
        let has_bg = present(&s.bg) || present(&stage);
        call(&s.clock, "update", &[]);
        if s.shader_features & 2 != 0 {
            put(
                &s.compose,
                "cdBattleTime",
                call(&s.clock, "getElapsed", &[]),
            );
        }
        if present(&s.world_sun) {
            let point = make(&s.three, "Vector3", &[]);
            call(&camera, "getWorldPosition", &[point.clone()]);
            call(&point, "addScaledVector", &[s.world_sun.clone(), n(100.)]);
            call(&point, "project", &[camera.clone()]);
            call(
                &s.sun,
                "set",
                &[
                    n(prop(&point, "x") * 0.5 + 0.5),
                    n(prop(&point, "y") * 0.5 + 0.5),
                ],
            );
        }
        if s.shader_features & 4 != 0 {
            call(&uniform(&s.compose, "cdGlareSun"), "copy", &[s.sun.clone()]);
        }
        if s.shader_features & 8 != 0 {
            let t = num(&call(&s.clock, "getElapsed", &[]));
            put(&s.compose, "cdWeatherTime", n(t));
            let flash = s.weather["flash"].as_f64().unwrap_or(0.) * lightning(t);
            set(&uniform(&s.compose, "cdWeather"), "w", &n(flash));
            let w2 = uniform(&s.compose, "cdWeather2");
            let surge = s.weather["surge"].as_f64().unwrap_or(0.) * dust_surge(t);
            set(&w2, "x", &n(surge));
            set(&w2, "w", &n(1. - s.horizon));
            let (count, lit) = s.strikes.get();
            s.strikes
                .set((count + u32::from(flash > 0.05 && !lit), flash > 0.05));
        }
        call(&layers, "set", &[n(0.)]);
        if present(&stage) {
            let projected = try_call(
                &stage,
                "render",
                &[camera.clone(), get(&s.fig, "width"), get(&s.fig, "height")],
            );
            if projected.is_err() {
                set(&layers, "mask", &mask);
                set(&shadow_map, "autoUpdate", &auto);
            }
            let projected = projected.unwrap_throw();
            put(&s.compose, "tBg", get(&projected, "color"));
            put(&s.compose, "hasBg", n(1.));
            call(
                &uniform(&s.compose, "bgSize"),
                "set",
                &[get(&s.fig, "width"), get(&s.fig, "height")],
            );
            for m in [&s.compose, &s.haze_mat] {
                put(m, "tDepth", get(&projected, "depth"));
                put(m, "hasDepth", n(1.));
            }
        }
        call(r, "setRenderTarget", &[s.fig.clone()]);
        call(r, "clear", &[]);
        if present(&stage) {
            let occluded = try_call(&stage, "occlude", &[camera.clone()]);
            if occluded.is_err() {
                set(&layers, "mask", &mask);
                set(&shadow_map, "autoUpdate", &auto);
            }
            occluded.unwrap_throw();
            let auto_clear = get(r, "autoClear");
            set(r, "autoClear", &false.into());
            let drawn = try_call(r, "render", &[scene.clone(), camera.clone()]);
            set(r, "autoClear", &auto_clear);
            if drawn.is_err() {
                set(&layers, "mask", &mask);
                set(&shadow_map, "autoUpdate", &auto);
            }
            drawn.unwrap_throw();
        } else {
            call(r, "render", &[scene.clone(), camera.clone()]);
        }
        call(&layers, "set", &[n(near)]);
        set(&shadow_map, "autoUpdate", &false.into());
        // nothing moved since the first pass, so its world matrices still hold
        let matrices = get(&scene, "matrixWorldAutoUpdate");
        set(&scene, "matrixWorldAutoUpdate", &false.into());
        let drawn = match (&s.mark, s.minimal) {
            // Minimal quality: the gun and the hands go straight over the fighters, on a cleared
            // depth, and the quad behind them marks their pixels; one picture fewer to fill.
            (Some((mark, ortho)), true) => {
                let auto_clear = get(r, "autoClear");
                set(r, "autoClear", &false.into());
                call(r, "clearDepth", &[]);
                let drawn = try_call(r, "render", &[scene.clone(), camera.clone()])
                    .and_then(|_| try_call(r, "render", &[mark.clone(), ortho.clone()]));
                set(r, "autoClear", &auto_clear);
                drawn
            }
            _ => {
                call(r, "setRenderTarget", &[s.near.clone()]);
                call(r, "clear", &[]);
                try_call(r, "render", &[scene.clone(), camera.clone()])
            }
        };
        set(&scene, "matrixWorldAutoUpdate", &matrices);
        drawn.unwrap_throw();
        set(&shadow_map, "autoUpdate", &auto);
        set(&layers, "mask", &mask);
        if present(&s.haze) && present(&s.look) && has_bg {
            let h = &s.haze_mat;
            call(
                &uniform(h, "invProj"),
                "copy",
                &[get(&camera, "projectionMatrixInverse")],
            );
            call(
                &uniform(h, "camWorld"),
                "copy",
                &[get(&camera, "matrixWorld")],
            );
            call(
                &uniform(h, "camPos"),
                "setFromMatrixPosition",
                &[get(&camera, "matrixWorld")],
            );
            put(h, "enemyDist", get(&haze, "enemyDepth"));
            put(h, "time", call(&s.clock, "getElapsed", &[]));
            s.sun_direction(&camera, &uniform(h, "sunDir"));
            let light = get(&haze, "light");
            let shadow = if boolean(&get(&light, "castShadow")) {
                get(&light, "shadow")
            } else {
                JsValue::NULL
            };
            let depth = get(&get(&shadow, "map"), "depthTexture");
            let has = present(&depth) && boolean(&get(&shadow_map, "enabled"));
            put(h, "hasShadow", n(if has { 1. } else { 0. }));
            put(h, "tShadow", if has { depth } else { JsValue::NULL });
            if present(&shadow) {
                call(
                    &uniform(h, "shadowMatrix"),
                    "copy",
                    &[get(&shadow, "matrix")],
                );
            }
            s.pass(h, &s.haze);
        }
        put(
            &s.compose,
            "focusInv",
            n(1. / prop(&haze, "enemyDepth").max(0.5)),
        );
        s.pass(&s.compose, &s.comp);
        if present(&s.rays_a) && present(&s.rays_b) && present(&s.look) && has_bg {
            put(&s.mask, "horizon", n(1. - s.horizon));
            s.pass(&s.mask, &s.rays_a);
            put(&s.blur, "tSrc", texture(&s.rays_a));
            put(&s.blur, "reach", n(0.92));
            put(&s.blur, "decay", n(0.975));
            s.pass(&s.blur, &s.rays_b);
            put(&s.blur, "tSrc", texture(&s.rays_b));
            put(
                &s.blur,
                "reach",
                n(0.92 / prop(&s.quality, "rays").max(1.) * 1.6),
            );
            put(&s.blur, "decay", n(1.));
            s.pass(&s.blur, &s.rays_a);
            put(&s.lens, "rays", get(&s.look, "rays"));
        }
        put(&s.lens, "hasBg", n(if has_bg { 1. } else { 0. }));
        s.pass(&s.lens, &JsValue::NULL);
    }

    /// Compiles the scene for both lens passes and the lens's own passes, each with the render
    /// target and layers `render` uses (programs differ by target), so the first frame links
    /// nothing; the browser compiles them in parallel where it can.
    pub fn warm(&self, scene: JsValue, camera: JsValue, near: f64) -> Promise {
        let s = self.state.borrow();
        let r = &s.renderer;
        if s.disposed || !get(r, "compileAsync").is_function() {
            return Promise::resolve(&JsValue::UNDEFINED);
        }
        let layers = get(&camera, "layers");
        let mask = get(&layers, "mask");
        let target = call(r, "getRenderTarget", &[]);
        let material = get(&s.quad, "material");
        let mesh = get(&s.quad, "_mesh");
        let mut jobs = vec![];
        for (layer, at) in [(0., &s.fig), (near, &s.near)] {
            call(&layers, "set", &[n(layer)]);
            call(r, "setRenderTarget", &[at.clone()]);
            jobs.extend(try_call(r, "compileAsync", &[scene.clone(), camera.clone()]).ok());
        }
        set(&layers, "mask", &mask);
        if present(&mesh) {
            let passes = [
                (&s.haze_mat, &s.haze),
                (&s.compose, &s.comp),
                (&s.mask, &s.rays_a),
                (&s.blur, &s.rays_b),
                (&s.lens, &JsValue::NULL),
            ];
            for (m, at) in passes {
                // only the last pass draws to the screen; the others skip when their target is off
                if !present(at) && !std::ptr::eq(m, &s.lens) {
                    continue;
                }
                set(&s.quad, "material", m);
                call(r, "setRenderTarget", &[at.clone()]);
                jobs.extend(try_call(r, "compileAsync", &[mesh.clone(), camera.clone()]).ok());
            }
            set(&s.quad, "material", &material);
        }
        call(r, "setRenderTarget", &[target]);
        let all = Promise::all_settled(&array(&jobs));
        future_to_promise(async move {
            let _ = JsFuture::from(all).await;
            Ok(JsValue::UNDEFINED)
        })
    }

    #[wasm_bindgen(js_name = sunDirection)]
    pub fn sun_direction(&self, camera: JsValue, out: JsValue) -> JsValue {
        self.state.borrow().sun_direction(&camera, &out);
        out
    }

    #[wasm_bindgen(getter, js_name = raysTexture)]
    pub fn rays_texture(&self) -> JsValue {
        texture(&self.state.borrow().rays_a)
    }

    pub fn dispose(&self) {
        self.state.borrow_mut().dispose();
    }
}

impl Drop for RustLens {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.dispose();
        }
    }
}

/// A screen quad just short of the far plane that passes only where something nearer was drawn
/// after the depth was cleared, and there sets the alpha to one half, leaving the colour.
fn hand_mark(three: &JsValue) -> (JsValue, JsValue) {
    let material = make(
        three,
        "ShaderMaterial",
        &[object(&[
            ("name", "handMark".into()),
            (
                "vertexShader",
                "void main() { gl_Position = vec4(position.xy, 0.9999, 1.0); }".into(),
            ),
            (
                "fragmentShader",
                "void main() { gl_FragColor = vec4(0.0, 0.0, 0.0, 0.5); }".into(),
            ),
            ("depthWrite", false.into()),
            ("depthFunc", get(three, "GreaterDepth")),
            ("blending", get(three, "CustomBlending")),
            ("blendSrc", get(three, "ZeroFactor")),
            ("blendDst", get(three, "OneFactor")),
            ("blendSrcAlpha", get(three, "OneFactor")),
            ("blendDstAlpha", get(three, "ZeroFactor")),
            ("toneMapped", false.into()),
        ])],
    );
    let quad = make(
        three,
        "Mesh",
        &[make(three, "PlaneGeometry", &[n(2.), n(2.)]), material],
    );
    set(&quad, "frustumCulled", &false.into());
    let scene = make(three, "Scene", &[]);
    call(&scene, "add", &[quad]);
    let camera = make(
        three,
        "OrthographicCamera",
        &[n(-1.), n(1.), n(1.), n(-1.), n(0.), n(1.)],
    );
    (scene, camera)
}

fn set_defines(material: &JsValue, defines: JsValue) {
    set(material, "defines", &defines);
    set(material, "needsUpdate", &true.into());
}

/// A lightning strike every eight to twenty seconds: a bright double flicker
/// that dies away within a second. Returns 0..1.
/// Dust storms come in waves: every half minute or so a gust thickens for a few
/// seconds until it nearly closes the view, then thins out again.
pub(crate) fn dust_surge(t: f64) -> f64 {
    let period = 30.0;
    let k = (t / period).floor();
    let h = |x: f64| ((x * 78.233).sin() * 43758.5453).fract().abs();
    let dt = t - (k * period + 4.0 + h(k) * (period - 16.0));
    let smooth = |x: f64| {
        let x = x.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    // two seconds to thicken, three held, four to clear
    smooth(dt / 2.0) * (1.0 - smooth((dt - 5.0) / 4.0)) * (0.8 + 0.2 * h(k + 1.7))
}
pub(crate) fn lightning(t: f64) -> f64 {
    let period = 14.0;
    let k = (t / period).floor();
    let h = |x: f64| ((x * 12.9898).sin() * 43758.5453).fract().abs();
    let dt = t - (k * period + h(k) * (period - 2.0));
    if !(0.0..1.4).contains(&dt) {
        return 0.0;
    }
    let first = (-dt * 9.0).exp();
    let second = if dt > 0.18 {
        (-(dt - 0.18) * 4.0).exp() * 0.85
    } else {
        0.0
    };
    (first + second).min(1.0) * (0.55 + 0.45 * h(k + 3.1))
}

impl LensState {
    fn weather_haze(&self) -> f64 {
        self.weather["haze"].as_f64().unwrap_or(1.)
    }
    fn weather_on(&self) -> bool {
        self.weather["active"].as_bool() == Some(true)
    }
    fn refresh_battle_shader(&mut self) {
        let light = present(&self.quality)
            && matches!(string(&get(&self.quality, "id")).as_str(), "minimal" | "low");
        let heat = self.heat && !self.night_vision && present(&self.quality) && !light;
        let weather = self.weather_on();
        // Low and minimal draw the weather with fewer layers of noise, rain and drops.
        let lite = weather && light;
        let minimal = self.minimal;
        let features = u8::from(self.night_vision)
            | (u8::from(heat) << 1)
            | (u8::from(self.sun_glare) << 2)
            | (u8::from(weather) << 3)
            | (u8::from(lite) << 4)
            | (u8::from(minimal) << 5);
        if weather {
            self.weather_uniforms();
        }
        if features == self.shader_features {
            if heat {
                call(
                    &uniform(&self.compose, "cdHeatDepth"),
                    "set",
                    &[
                        n(self.heat_depth[0]),
                        n(self.heat_depth[1]),
                        n(self.heat_depth[2]),
                        n(1. - self.horizon),
                    ],
                );
            }
            return;
        }
        self.shader_features = features;
        // on WebGPU the compose pass is built from nodes, from the same bits
        if get(&self.compose, "setFeatures").is_function() {
            call(&self.compose, "setFeatures", &[n(f64::from(features))]);
            return;
        }
        let original = include_str!("shaders/composeShader.glsl");
        let mut fragment = original.to_string();
        let uniforms = get(&self.compose, "uniforms");
        for key in [
            "cdBattleTime",
            "cdHeatDepth",
            "cdGlareSun",
            "cdWeather",
            "cdWeatherTime",
            "cdWeatherDim",
            "cdFogColor",
            "cdDustColor",
            "cdRainColor",
            "cdWeather2",
        ] {
            let _ = js_sys::Reflect::delete_property(
                &js_sys::Object::from(uniforms.clone()),
                &key.into(),
            );
        }
        let mut declarations = String::new();
        if heat {
            declarations.push_str("uniform float cdBattleTime;\nuniform vec4 cdHeatDepth;\n");
            set(&uniforms, "cdBattleTime", &object(&[("value", n(0.))]));
            set(
                &uniforms,
                "cdHeatDepth",
                &object(&[(
                    "value",
                    make(
                        &self.three,
                        "Vector4",
                        &[
                            n(self.heat_depth[0]),
                            n(self.heat_depth[1]),
                            n(self.heat_depth[2]),
                            n(1. - self.horizon),
                        ],
                    ),
                )]),
            );
            fragment = fragment.replace(
                "vec3 bg = hasBg > 0.5 ? focused(vUv) : vec3(0.0);",
                include_str!("shaders/battleHeat.glsl"),
            );
        }
        if self.sun_glare {
            declarations.push_str("uniform vec2 cdGlareSun;\n");
            set(
                &uniforms,
                "cdGlareSun",
                &object(&[("value", make(&self.three, "Vector2", &[n(0.7), n(0.75)]))]),
            );
            fragment = fragment.replace("gl_FragColor = vec4(c, max(a, n.a));", "vec2 glareDelta = (vUv - cdGlareSun) * vec2(bgSize.x / max(bgSize.y, 1.0), 1.0);\n  float glare = exp(-length(glareDelta) * 5.5);\n  c += vec3(1.0, 0.83, 0.62) * glare * 0.14;\n  gl_FragColor = vec4(c, max(a, n.a));");
        }
        if weather {
            declarations.push_str(if minimal {
                // one flat layer: veils without noise, one rain layer, coarse drops on the lens
                "#define CD_WX_LITE
#define CD_WX_MINIMAL
#define CD_WX_LAYERS 1
"
            } else if lite {
                "#define CD_WX_LITE
#define CD_WX_LAYERS 2
"
            } else {
                "#define CD_WX_LAYERS 3
"
            });
            declarations.push_str(include_str!("shaders/weatherCommon.glsl"));
            fragment = fragment.replace(
                "vec4 n = smoothRead(tNear, vUv, nearSize);",
                include_str!("shaders/weather.glsl"),
            );
            // what sits on the lens itself comes after the gun and the hands
            fragment = fragment.replace(
                "c = tone(n) + c * (1.0 - n.a);",
                include_str!("shaders/weatherLens.glsl"),
            );
            self.weather_uniforms();
        }
        if minimal {
            // the gun and the hands are in the fighters' picture, marked by an alpha of one half;
            // the air's weather stays behind them and the lens's lies over them
            fragment = fragment.replace(
                "vec4 f = texture(tFig, vUv);",
                r#"vec4 f = texture(tFig, vUv);
  float hand = 1.0 - step(0.01, abs(f.a - 0.5));
  f.a = mix(f.a, 1.0, hand);
  // beside the gun a soft fringe of its colour, as the separate picture's smooth read gave
  vec4 rim = vec4(0.0);
  if (hand < 0.5) {
    ivec2 at = ivec2(gl_FragCoord.xy), top = textureSize(tFig, 0) - 1;
    for (int i = 0; i < 4; i++) {
      ivec2 o = i < 2 ? ivec2(i * 2 - 1, 0) : ivec2(0, i * 2 - 5);
      vec4 t = texelFetch(tFig, clamp(at + o, ivec2(0), top), 0);
      rim += vec4(t.rgb, 1.0) * (1.0 - step(0.01, abs(t.a - 0.5)));
    }
  }"#,
            );
            fragment = fragment.replace(
                "float a = hasBg > 0.5 ? 1.0 : f.a;",
                "float a = hasBg > 0.5 ? 1.0 : f.a;\n  vec3 c0 = c;",
            );
            fragment = fragment.replace(
                "vec4 n = smoothRead(tNear, vUv, nearSize);",
                r#"c = mix(c, c0, hand);
  float fringe = min(rim.a * 0.2, 0.6);
  if (rim.a > 0.0) c = mix(c, tone(vec4(rim.rgb / rim.a, 1.0)), fringe);
  vec4 n = vec4(0.0, 0.0, 0.0, max(hand, fringe));"#,
            );
            fragment = fragment.replace("c = tone(n) + c * (1.0 - n.a);", "");
        }
        if self.night_vision {
            // Daylight ACES subtracts a black offset and clamps faint HDR
            // radiance to zero before the intensifier can amplify it. Preserve
            // that signal for both fighters and the first-person weapon only
            // in the NVG shader; unaided exposure keeps the original ACES path.
            fragment = fragment.replace(
                "vec3 c = toSRGB(aces(lin));",
                "vec3 c = toSRGB(max(lin, vec3(0.0)));",
            );
            fragment = fragment.replace(
                "gl_FragColor = vec4(c, max(a, n.a));",
                include_str!("shaders/nightVision.glsl"),
            );
        }
        set(
            &self.compose,
            "fragmentShader",
            &format!("{declarations}{fragment}").into(),
        );
        set(&self.compose, "needsUpdate", &true.into());
    }
    /// Creates the weather uniforms when missing and fills them from the fx.
    fn weather_uniforms(&self) {
        let uniforms = get(&self.compose, "uniforms");
        let w = &self.weather;
        let f = |k: &str| w[k].as_f64().unwrap_or(0.);
        let three = &self.three;
        let ensure = |key: &str, value: JsValue| {
            if !present(&get(&uniforms, key)) {
                set(&uniforms, key, &object(&[("value", value)]));
            }
        };
        ensure("cdWeather", make(three, "Vector4", &[]));
        ensure("cdWeather2", make(three, "Vector4", &[]));
        ensure("cdWeatherTime", n(0.));
        ensure("cdWeatherDim", n(1.));
        for key in ["cdFogColor", "cdDustColor", "cdRainColor"] {
            ensure(key, make(three, "Vector3", &[]));
        }
        call(
            &uniform(&self.compose, "cdWeather"),
            "set",
            &[n(f("fog")), n(f("dust")), n(f("rain")), n(0.)],
        );
        let w2 = uniform(&self.compose, "cdWeather2");
        set(&w2, "y", &n(f("dust")));
        set(&w2, "z", &n(f("wet")));
        set(&w2, "w", &n(1. - self.horizon));
        put(
            &self.compose,
            "cdWeatherDim",
            n(w["dim"].as_f64().unwrap_or(1.)),
        );
        for (key, field) in [
            ("cdFogColor", "fogColor"),
            ("cdDustColor", "dustColor"),
            ("cdRainColor", "rainColor"),
        ] {
            let c = |i: usize| n(w[field][i].as_f64().unwrap_or(0.));
            call(&uniform(&self.compose, key), "set", &[c(0), c(1), c(2)]);
        }
    }
    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        // Stop pending images from replacing resources after the owner is gone.
        self.loading.clear();
        self.depth_src.clear();
        for resource in [
            &self.fig,
            &self.near,
            &self.comp,
            &self.haze,
            &self.rays_a,
            &self.rays_b,
            &self.compose,
            &self.haze_mat,
            &self.mask,
            &self.blur,
            &self.lens,
            &self.quad,
            &self.clock,
            &self.bg,
            &self.depth_map,
        ] {
            if present(resource) {
                let _ = try_call(resource, "dispose", &[]);
            }
        }
        self.bg = JsValue::NULL;
        self.stage = JsValue::NULL;
        self.depth_map = JsValue::NULL;
        self.haze = JsValue::NULL;
        self.rays_a = JsValue::NULL;
        self.rays_b = JsValue::NULL;
    }
    fn set_depth(&mut self, tex: JsValue) {
        dispose(&self.depth_map);
        self.depth_map = tex.clone();
        for m in [&self.compose, &self.haze_mat] {
            put(m, "tDepth", tex.clone());
            put(m, "hasDepth", n(if present(&tex) { 1. } else { 0. }));
        }
    }
    fn pass(&self, material: &JsValue, target: &JsValue) {
        set(&self.quad, "material", material);
        call(&self.renderer, "setRenderTarget", &[target.clone()]);
        call(&self.quad, "render", &[self.renderer.clone()]);
    }
    fn sun_direction(&self, camera: &JsValue, out: &JsValue) {
        if present(&self.world_sun) {
            call(out, "copy", &[self.world_sun.clone()]);
            call(out, "normalize", &[]);
            return;
        }
        call(
            out,
            "set",
            &[
                n(prop(&self.sun, "x") * 2. - 1.),
                n(prop(&self.sun, "y") * 2. - 1.),
                n(1.),
            ],
        );
        call(
            out,
            "applyMatrix4",
            &[get(camera, "projectionMatrixInverse")],
        );
        call(out, "transformDirection", &[get(camera, "matrixWorld")]);
    }
    fn size_targets(&self) {
        if !present(&self.quality) {
            return;
        }
        let q = &self.quality;
        let near = if present(&self.look) {
            prop(&self.look, "near")
        } else {
            1.
        };
        let p = compute(
            "size",
            &[
                self.px[0],
                self.px[1],
                self.css[0],
                self.css[1],
                prop(&get(&self.renderer, "capabilities"), "maxTextureSize"),
                prop(q, "ssaa"),
                near,
                prop(q, "hazeScale"),
                prop(q, "raysScale"),
            ],
        );
        for t in [&self.fig, &self.comp] {
            call(t, "setSize", &[n(p[0]), n(p[1])]);
        }
        // at minimal quality the near picture goes unused; it keeps a single pixel
        let (nw, nh) = if self.minimal { (1., 1.) } else { (p[2], p[3]) };
        call(&self.near, "setSize", &[n(nw), n(nh)]);
        call(
            &uniform(&self.compose, "nearSize"),
            "set",
            &[n(p[2]), n(p[3])],
        );
        if present(&self.haze) {
            call(&self.haze, "setSize", &[n(p[4]), n(p[5])]);
            call(
                &uniform(&self.compose, "hazeSize"),
                "set",
                &[n(p[4]), n(p[5])],
            );
        }
        if present(&self.rays_a) && present(&self.rays_b) {
            for t in [&self.rays_a, &self.rays_b] {
                call(t, "setSize", &[n(p[6]), n(p[7])]);
            }
            put(&self.mask, "lod", n(p[8]));
        }
        call(&uniform(&self.lens, "compSize"), "set", &[n(p[0]), n(p[1])]);
        call(
            &uniform(&self.lens, "outSize"),
            "set",
            &[n(self.px[0]), n(self.px[1])],
        );
        put(&self.lens, "ssaa", n(p[9]));
        put(&self.lens, "glowLod", n(p[10]));
    }
    fn find_sun(&self, image: &JsValue) {
        let w = 48.;
        let h = (48. * prop(image, "height") / prop(image, "width")).round();
        let canvas = call(&document(), "createElement", &["canvas".into()]);
        set(&canvas, "width", &n(w));
        set(&canvas, "height", &n(h));
        let context = call(
            &canvas,
            "getContext",
            &["2d".into(), object(&[("willReadFrequently", true.into())])],
        );
        if !present(&context) {
            return;
        }
        call(
            &context,
            "drawImage",
            &[image.clone(), n(0.), n(0.), n(w), n(h)],
        );
        let pixels = Uint8ClampedArray::new(&get(
            &call(&context, "getImageData", &[n(0.), n(0.), n(w), n(h)]),
            "data",
        ));
        let mut input = vec![
            w,
            h,
            self.horizon,
            prop(&self.sun, "x"),
            prop(&self.sun, "y"),
        ];
        input.extend(pixels.to_vec().into_iter().map(f64::from));
        call(
            &self.sun,
            "fromArray",
            &[Float64Array::from(compute("sun", &input).as_slice()).into()],
        );
    }
}

fn load_backdrop(state: &Rc<RefCell<LensState>>) {
    let (url, request) = {
        let mut s = state.borrow_mut();
        let url = if boolean(&get(&s.quality, "sharpBackdrop")) {
            s.two
                .clone()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| s.one.clone())
        } else {
            s.one.clone()
        };
        if s.disposed || url.is_empty() || url == s.loading {
            return;
        }
        s.loading = url.clone();
        let request = call(
            &make(&s.three, "TextureLoader", &[]),
            "loadAsync",
            &[url.clone().into()],
        );
        (url, request)
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
        if s.disposed || s.loading != url {
            dispose(&tex);
            return;
        }
        dispose(&s.bg);
        set(&tex, "colorSpace", &get(&s.three, "NoColorSpace"));
        set(
            &tex,
            "minFilter",
            &get(&s.three, "LinearMipmapLinearFilter"),
        );
        set(&tex, "anisotropy", &n(4.));
        s.bg = tex.clone();
        let img = get(&tex, "image");
        s.find_sun(&img);
        put(&s.compose, "tBg", tex);
        put(&s.compose, "hasBg", n(1.));
        call(
            &uniform(&s.compose, "bgSize"),
            "set",
            &[get(&img, "width"), get(&img, "height")],
        );
    });
}
