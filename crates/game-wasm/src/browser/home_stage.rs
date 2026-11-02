//! One intact base photograph, a soft depth field and a bounded 5–7% approach.
//! Combat projection and the fitting-room backdrop have independent renderers.
//! On high and ultra the photograph lives (live.rs): the camera breathes and the base's lamps,
//! screens, water and air move.
use super::{base_region::REGIONS, bridge::*, live};
use js_sys::Promise;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, future_to_promise};

const VERTEX: &str = include_str!("shaders/homeDepthVertex.glsl");
const FRAGMENT: &str = include_str!("shaders/homeDepthFragment.glsl");
fn n(value: f64) -> JsValue {
    value.into()
}
fn error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}
fn make(three: &JsValue, name: &str, args: &[JsValue]) -> JsValue {
    construct(&get(three, name), args)
}
fn uniform(value: JsValue) -> JsValue {
    object(&[("value", value)])
}
fn uv(uniforms: &JsValue, key: &str) -> JsValue {
    get(&get(uniforms, key), "value")
}
fn put(uniforms: &JsValue, key: &str, value: JsValue) {
    set(&get(uniforms, key), "value", &value);
}
fn release(resource: &JsValue) {
    if present(resource) {
        let _ = try_call(resource, "dispose", &[]);
    }
}
fn resolved_url(url: &str, base: &str) -> Result<String, JsValue> {
    let value = js_sys::Reflect::construct(
        get(&global(), "URL").unchecked_ref::<js_sys::Function>(),
        &array(&[url.into(), base.into()]),
    )?;
    Ok(string(&get(&value, "href")))
}

#[derive(Clone, Copy)]
struct Config {
    aspect: f64,
    zoom: f64,
    parallax: f64,
    battle: [f64; 2],
    fitting: [f64; 2],
    persona: [f64; 2],
}
impl Config {
    fn parse(data: &serde_json::Value) -> Result<Self, JsValue> {
        let invalid = || error("Invalid base depth-photo manifest");
        if data["version"].as_u64() != Some(1) || data["kind"].as_str() != Some("depth-photo") {
            return Err(invalid());
        }
        for name in ["image", "depth"] {
            if !data[name].as_str().is_some_and(|s| !s.trim().is_empty()) {
                return Err(invalid());
            }
        }
        let aspect = data["aspect"]
            .as_f64()
            .filter(|v| v.is_finite() && (0.05..20.).contains(v))
            .ok_or_else(invalid)?;
        let number = |key: &str, fallback: f64| {
            data[key]
                .as_f64()
                .filter(|v| v.is_finite())
                .unwrap_or(fallback)
        };
        let target = |name: &str| -> Result<[f64; 2], JsValue> {
            let values = data["targets"][name]
                .as_array()
                .filter(|v| v.len() == 2)
                .ok_or_else(invalid)?;
            let mut out = [0.; 2];
            for (index, value) in values.iter().enumerate() {
                out[index] = value
                    .as_f64()
                    .filter(|v| v.is_finite() && (0. ..=1.).contains(v))
                    .ok_or_else(invalid)?;
            }
            // Authored targets use photograph coordinates measured from the top.
            out[1] = 1. - out[1];
            Ok(out)
        };
        Ok(Self {
            aspect,
            zoom: number("zoom", 1.06).clamp(1.05, 1.07),
            parallax: number("parallax", 0.002).clamp(0., 0.002),
            battle: target("battle")?,
            fitting: target("fitting")?,
            persona: target("persona")?,
        })
    }
}

struct State {
    three: JsValue,
    canvas: JsValue,
    renderer: JsValue,
    scene: JsValue,
    camera: JsValue,
    geometry: JsValue,
    material: JsValue,
    uniforms: JsValue,
    textures: Vec<JsValue>,
    abort: Option<web_sys::AbortController>,
    config: Option<Config>,
    width: f64,
    height: f64,
    last: String,
    clock: live::Clock,
    disposed: bool,
}
impl State {
    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        self.config = None;
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
        for texture in self.textures.drain(..) {
            release(&texture);
        }
        for resource in [&self.geometry, &self.material, &self.renderer] {
            release(resource);
        }
        put(&self.uniforms, "uColor", JsValue::NULL);
        put(&self.uniforms, "uDepth", JsValue::NULL);
        call(&self.scene, "clear", &[]);
    }
}

#[wasm_bindgen]
pub struct RustHomeStage {
    state: Rc<RefCell<State>>,
    ready: Promise,
}
#[wasm_bindgen]
impl RustHomeStage {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, canvas: JsValue, url: String) -> Result<Self, JsValue> {
        let three = get(&host, "THREE");
        let renderer = js_sys::Reflect::construct(
            get(&three, "WebGLRenderer").unchecked_ref::<js_sys::Function>(),
            &array(&[object(&[
                ("canvas", canvas.clone()),
                ("antialias", false.into()),
                ("alpha", false.into()),
                ("powerPreference", "low-power".into()),
            ])]),
        )?;
        // Shader samples display-referred RGB directly, with no additional grade.
        set(&renderer, "toneMapping", &get(&three, "NoToneMapping"));
        set(
            &renderer,
            "outputColorSpace",
            &get(&three, "LinearSRGBColorSpace"),
        );
        let scene = make(&three, "Scene", &[]);
        let camera = make(
            &three,
            "OrthographicCamera",
            &[n(-1.), n(1.), n(1.), n(-1.), n(0.), n(1.)],
        );
        let uniforms = object(&[
            ("uColor", uniform(JsValue::NULL)),
            ("uDepth", uniform(JsValue::NULL)),
            ("uCover", uniform(make(&three, "Vector2", &[n(1.), n(1.)]))),
            (
                "uPivot",
                uniform(make(&three, "Vector2", &[n(0.5), n(0.5)])),
            ),
            (
                "uDepthTexel",
                uniform(make(&three, "Vector2", &[n(1. / 512.), n(1. / 912.)])),
            ),
            ("uZoom", uniform(n(1.))),
            ("uParallax", uniform(n(0.))),
        ]);
        let life = REGIONS
            .iter()
            .find(|region| url.ends_with(region.home))
            .map_or(&live::STILL, |region| region.home_life);
        for (key, value) in live::uniforms(life, 1., &|v| make(&three, "Vector4", &v.map(n))) {
            set(&uniforms, key, &value);
        }
        let material = make(
            &three,
            "ShaderMaterial",
            &[object(&[
                ("uniforms", uniforms.clone()),
                ("vertexShader", VERTEX.into()),
                ("fragmentShader", format!("{}{FRAGMENT}", live::SCENE).into()),
                ("toneMapped", false.into()),
                ("depthTest", false.into()),
                ("depthWrite", false.into()),
            ])],
        );
        let geometry = make(&three, "PlaneGeometry", &[n(2.), n(2.)]);
        let quad = make(&three, "Mesh", &[geometry.clone(), material.clone()]);
        set(&quad, "name", &"Intact base photograph".into());
        set(&quad, "frustumCulled", &false.into());
        call(&scene, "add", &[quad]);
        let state = Rc::new(RefCell::new(State {
            three,
            canvas,
            renderer,
            scene,
            camera,
            geometry,
            material,
            uniforms,
            textures: vec![],
            abort: web_sys::AbortController::new().ok(),
            config: None,
            width: 0.,
            height: 0.,
            last: String::new(),
            clock: live::Clock::default(),
            disposed: false,
        }));
        let weak = Rc::downgrade(&state);
        let ready = future_to_promise(async move {
            load_assets(&weak, &url).await.map(|()| JsValue::UNDEFINED)
        });
        Ok(Self { state, ready })
    }
    #[wasm_bindgen(getter)]
    pub fn ready(&self) -> Promise {
        self.ready.clone()
    }
    pub fn draw(&mut self, target: &str, progress: f64) -> Result<(), JsValue> {
        let mut s = self.state.borrow_mut();
        if s.disposed {
            return Ok(());
        }
        let Some(config) = s.config else {
            return Ok(());
        };
        let w = num(&get(&s.canvas, "clientWidth"));
        let h = num(&get(&s.canvas, "clientHeight"));
        if !w.is_finite() || !h.is_finite() || w <= 0. || h <= 0. {
            return Ok(());
        }
        let (pivot, p) = match target {
            "battle" => (config.battle, progress),
            "fitting" => (config.fitting, progress),
            "persona" => (config.persona, progress),
            _ => ([0.5, 0.5], 0.),
        };
        let p = if p.is_finite() { p.clamp(0., 1.) } else { 0. };
        let (alive, time) = s.clock.tick();
        let mut key = format!("{target}:{p:.6}:{w}:{h}");
        if alive > 0. {
            key.push_str(&format!(":{alive}:{time}"));
        }
        if key == s.last {
            return Ok(());
        }
        let eased = p * p * p * (p * (p * 6. - 15.) + 10.);
        let zoom = 1. + (config.zoom - 1.) * eased;
        if w != s.width || h != s.height {
            let dpr = num(&get(&global(), "devicePixelRatio"));
            let dpr = if dpr.is_finite() {
                dpr.clamp(1., 1.5)
            } else {
                1.
            };
            call(&s.renderer, "setPixelRatio", &[n(dpr)]);
            call(&s.renderer, "setSize", &[n(w), n(h), false.into()]);
            let full_w = (h * config.aspect).max(w);
            let full_h = (w / config.aspect).max(h);
            call(
                &uv(&s.uniforms, "uCover"),
                "set",
                &[n(w / full_w), n(h / full_h)],
            );
            s.width = w;
            s.height = h;
        }
        call(&uv(&s.uniforms, "uPivot"), "set", &pivot.map(n));
        put(&s.uniforms, "uZoom", n(zoom));
        put(&s.uniforms, "uParallax", n(config.parallax * eased));
        put(&s.uniforms, "uLive", n(alive));
        put(&s.uniforms, "uTime", n(time));
        for (key, value) in [
            ("data-home-progress", format!("{p:.4}")),
            ("data-home-target", target.to_string()),
            ("data-home-zoom", format!("{zoom:.6}")),
            ("data-depth-photo", "true".into()),
        ] {
            call(&s.canvas, "setAttribute", &[key.into(), value.into()]);
        }
        try_call(&s.renderer, "render", &[s.scene.clone(), s.camera.clone()])?;
        s.last = key;
        Ok(())
    }
    pub fn dispose(&mut self) {
        self.state.borrow_mut().dispose();
    }
}
impl Drop for RustHomeStage {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.dispose();
        }
    }
}

fn active(weak: &Weak<RefCell<State>>) -> Result<Rc<RefCell<State>>, JsValue> {
    let state = weak
        .upgrade()
        .ok_or_else(|| error("Base depth-photo load cancelled"))?;
    if state.borrow().disposed {
        return Err(error("Base depth-photo load cancelled"));
    }
    Ok(state)
}
async fn load_assets(weak: &Weak<RefCell<State>>, url: &str) -> Result<(), JsValue> {
    let (three, signal) = {
        let current = active(weak)?;
        let s = current.borrow();
        (
            s.three.clone(),
            s.abort.as_ref().map(|a| JsValue::from(a.signal())),
        )
    };
    let url = resolved_url(url, &string(&get(&document(), "baseURI")))?;
    let mut fetch_args = vec![url.clone().into()];
    if let Some(signal) = signal {
        fetch_args.push(object(&[("signal", signal)]));
    }
    let response = JsFuture::from(Promise::resolve(&try_call(
        &global(),
        "fetch",
        &fetch_args,
    )?))
    .await?;
    if !boolean(&get(&response, "ok")) {
        return Err(error("Base depth-photo manifest unavailable"));
    }
    let manifest = JsFuture::from(Promise::resolve(&try_call(&response, "json", &[])?)).await?;
    let manifest = data(&manifest);
    let config = Config::parse(&manifest)?;
    active(weak)?;
    let loader = make(&three, "TextureLoader", &[]);
    // Resolve both before starting either, so an invalid URL cannot strand a sibling request.
    let urls = ["image", "depth"].map(|key| resolved_url(manifest[key].as_str().unwrap(), &url));
    let urls = [urls[0].clone()?, urls[1].clone()?];
    let tasks: Vec<JsValue> = urls
        .into_iter()
        .map(|url| match try_call(&loader, "loadAsync", &[url.into()]) {
            Ok(task) => Promise::resolve(&task).into(),
            Err(reason) => Promise::reject(&reason).into(),
        })
        .collect();
    let settled = JsFuture::from(Promise::all_settled(&array(&tasks))).await?;
    let mut textures = vec![];
    let mut failure = None;
    for result in values(&settled) {
        if string(&get(&result, "status")) == "fulfilled" {
            textures.push(get(&result, "value"));
        } else {
            failure = Some(get(&result, "reason"));
        }
    }
    let current = active(weak);
    if let Err(reason) = &current {
        failure = Some(reason.clone());
    }
    if let Some(reason) = failure {
        for texture in textures {
            release(&texture);
        }
        return Err(reason);
    }
    for texture in &textures {
        set(texture, "colorSpace", &get(&three, "NoColorSpace"));
        set(texture, "wrapS", &get(&three, "ClampToEdgeWrapping"));
        set(texture, "wrapT", &get(&three, "ClampToEdgeWrapping"));
        set(texture, "generateMipmaps", &false.into());
        set(texture, "minFilter", &get(&three, "LinearFilter"));
        set(texture, "magFilter", &get(&three, "LinearFilter"));
    }
    let current = current?;
    let mut s = current.borrow_mut();
    put(&s.uniforms, "uColor", textures[0].clone());
    put(&s.uniforms, "uDepth", textures[1].clone());
    let image = get(&textures[1], "image");
    let width = num(&get(&image, "width")).max(1.);
    let height = num(&get(&image, "height")).max(1.);
    call(
        &uv(&s.uniforms, "uDepthTexel"),
        "set",
        &[n(1. / width), n(1. / height)],
    );
    put(&s.uniforms, "uAspect", n(config.aspect));
    s.textures = textures;
    s.config = Some(config);
    s.abort = None;
    Ok(())
}
