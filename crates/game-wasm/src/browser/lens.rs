//! Body-camera render graph. Rust owns targets, quality transitions and asynchronous image lifetimes.
use super::bridge::*;
use js_sys::{Float64Array, Promise, Uint8ClampedArray};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};

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
    depth_map: JsValue,
    sun: JsValue,
    one: String,
    two: Option<String>,
    depth_src: String,
    loading: String,
    horizon: f64,
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
        let shader = |fragment: &str, entries: &[(&str, JsValue)]| {
            let uniforms = object(&[]);
            for (key, value) in entries {
                set(&uniforms, key, &object(&[("value", value.clone())]));
            }
            make(
                &three,
                "ShaderMaterial",
                &[object(&[
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
            include_str!("shaders/raysBlurShader.glsl"),
            &[
                ("tSrc", JsValue::NULL),
                ("light", sun.clone()),
                ("reach", n(0.9)),
                ("decay", n(0.97)),
            ],
        );
        let lens = shader(
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
                depth_map: JsValue::NULL,
                sun,
                one: String::new(),
                two: None,
                depth_src: String::new(),
                loading: String::new(),
                horizon: 0.4,
                px: [1., 1.],
                css: [1., 1.],
                view: [0., 0., 1., 1.],
            })),
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
        put(&s.haze_mat, "density", get(&look, "haze"));
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
            if !present(&s.quality) || prop(&s.quality, "msaa") != prop(&q, "msaa") {
                set(&s.fig, "samples", &get(&q, "msaa"));
                dispose(&s.fig);
                set(&s.near, "samples", &n(prop(&q, "msaa").min(4.)));
                dispose(&s.near);
            }
            s.quality = q.clone();
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
        call(&s.clock, "update", &[]);
        call(&layers, "set", &[n(0.)]);
        call(r, "setRenderTarget", &[s.fig.clone()]);
        call(r, "clear", &[]);
        call(r, "render", &[scene.clone(), camera.clone()]);
        call(&layers, "set", &[n(near)]);
        set(&shadow_map, "autoUpdate", &false.into());
        call(r, "setRenderTarget", &[s.near.clone()]);
        call(r, "clear", &[]);
        call(r, "render", &[scene, camera.clone()]);
        set(&shadow_map, "autoUpdate", &auto);
        set(&layers, "mask", &mask);
        if present(&s.haze) && present(&s.look) && present(&s.bg) {
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
        if present(&s.rays_a) && present(&s.rays_b) && present(&s.look) && present(&s.bg) {
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
        put(&s.lens, "hasBg", n(if present(&s.bg) { 1. } else { 0. }));
        s.pass(&s.lens, &JsValue::NULL);
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

fn set_defines(material: &JsValue, defines: JsValue) {
    set(material, "defines", &defines);
    set(material, "needsUpdate", &true.into());
}

impl LensState {
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
        call(&self.near, "setSize", &[n(p[2]), n(p[3])]);
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
