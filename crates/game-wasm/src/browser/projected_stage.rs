//! Fixed-camera projection on independent world geometry, including backward overscan.
//! Rust owns loading, render passes, state restoration and cancellation of GPU resources.
use super::bridge::*;
use js_sys::{Float32Array, Promise};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, future_to_promise};

const VERTEX: &str = include_str!("shaders/projectedStageVertex.glsl");
const FRAGMENT: &str = include_str!("shaders/projectedStageFragment.glsl");
const DEPTH: &str = include_str!("shaders/projectedStageDepth.glsl");

fn n(value: f64) -> JsValue {
    value.into()
}
fn make(three: &JsValue, class: &str, args: &[JsValue]) -> JsValue {
    construct(&get(three, class), args)
}
fn error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}
fn release(value: &JsValue) {
    if present(value) {
        let _ = try_call(value, "dispose", &[]);
    }
}
fn uniforms(values: &[(&str, JsValue)]) -> JsValue {
    let out = object(&[]);
    for (key, value) in values {
        set(&out, key, &object(&[("value", value.clone())]));
    }
    out
}
fn shader(three: &JsValue, fragment: &str, values: &[(&str, JsValue)]) -> JsValue {
    make(
        three,
        "ShaderMaterial",
        &[object(&[
            ("vertexShader", VERTEX.into()),
            ("fragmentShader", fragment.into()),
            ("uniforms", uniforms(values)),
            ("side", get(three, "DoubleSide")),
            ("toneMapped", false.into()),
        ])],
    )
}

struct StageState {
    disposed: bool,
    ready: bool,
    request: u64,
    loading: Option<Promise>,
    source: String,
    abort: Option<web_sys::AbortController>,
    three: JsValue,
    renderer: JsValue,
    scene: JsValue,
    root: JsValue,
    color: JsValue,
    depth: JsValue,
    depth_material: JsValue,
    occlusion_material: JsValue,
    backdrop: JsValue,
    meshes: Vec<JsValue>,
    textures: Vec<JsValue>,
    manifest: serde_json::Value,
}

#[wasm_bindgen]
pub struct RustProjectedStage {
    state: Rc<RefCell<StageState>>,
}

#[wasm_bindgen]
impl RustProjectedStage {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, renderer: JsValue) -> Self {
        let three = get(&host, "THREE");
        let scene = make(&three, "Scene", &[]);
        let root = make(&three, "Group", &[]);
        call(&scene, "add", &[root.clone()]);
        let color = make(
            &three,
            "WebGLRenderTarget",
            &[
                n(1.),
                n(1.),
                object(&[
                    ("samples", n(4.)),
                    ("generateMipmaps", true.into()),
                    ("minFilter", get(&three, "LinearMipmapLinearFilter")),
                ]),
            ],
        );
        // The stage's proxies are rough boxes; weather and focus read this depth
        // from a blurred mip so their edges never show as cut lines.
        let depth = make(
            &three,
            "WebGLRenderTarget",
            &[
                n(1.),
                n(1.),
                object(&[
                    ("generateMipmaps", true.into()),
                    ("minFilter", get(&three, "LinearMipmapLinearFilter")),
                ]),
            ],
        );
        for target in [&color, &depth] {
            set(
                &get(target, "texture"),
                "colorSpace",
                &get(&three, "NoColorSpace"),
            );
        }
        let depth_material = shader(
            &three,
            DEPTH,
            &[("sourceViewProjection", make(&three, "Matrix4", &[]))],
        );
        let occlusion_material = make(
            &three,
            "MeshBasicMaterial",
            &[object(&[
                ("colorWrite", false.into()),
                ("side", get(&three, "DoubleSide")),
                ("polygonOffset", true.into()),
                ("polygonOffsetFactor", n(1.)),
                ("polygonOffsetUnits", n(1.)),
            ])],
        );
        Self {
            state: Rc::new(RefCell::new(StageState {
                disposed: false,
                ready: false,
                request: 0,
                loading: None,
                source: String::new(),
                abort: None,
                three,
                renderer,
                scene,
                root,
                color,
                depth,
                depth_material,
                occlusion_material,
                backdrop: JsValue::NULL,
                meshes: vec![],
                textures: vec![],
                manifest: serde_json::Value::Null,
            })),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn ready(&self) -> bool {
        self.state.borrow().ready
    }

    pub fn load(&self, url: String, variant: Option<JsValue>) -> Promise {
        let variant = variant.unwrap_or(JsValue::NULL);
        let descriptor = data(&variant);
        let source = format!("{url}|{descriptor}");
        let (request, signal) = {
            let mut s = self.state.borrow_mut();
            if s.disposed {
                return Promise::reject(&error("Projected stage has been disposed"));
            }
            if s.source == source && s.ready {
                return Promise::resolve(&JsValue::UNDEFINED);
            }
            if s.source == source {
                if let Some(promise) = &s.loading {
                    return promise.clone();
                }
            }
            if let Some(abort) = s.abort.take() {
                abort.abort();
            }
            s.clear_assets();
            s.source = source;
            s.request += 1;
            s.abort = web_sys::AbortController::new().ok();
            (
                s.request,
                s.abort.as_ref().map(|a| JsValue::from(a.signal())),
            )
        };
        let weak = Rc::downgrade(&self.state);
        let promise = future_to_promise(async move {
            let result = load_assets(&weak, request, &url, &descriptor, signal).await;
            if let Some(state) = weak.upgrade() {
                let mut s = state.borrow_mut();
                if s.request == request {
                    s.loading = None;
                    s.abort = None;
                }
            }
            result.map(|()| JsValue::UNDEFINED)
        });
        self.state.borrow_mut().loading = Some(promise.clone());
        promise
    }

    #[wasm_bindgen(js_name = setTravel)]
    pub fn set_travel(&self, metres: f64, direction_x: Option<f64>) {
        let s = self.state.borrow();
        if !s.disposed && metres.is_finite() {
            let x = direction_x.filter(|v| v.is_finite()).unwrap_or(0.);
            call(
                &get(&s.root, "position"),
                "set",
                &[n(-x * metres), n(0.), n(metres)],
            );
        }
    }

    /// Exclude only the floor: the existing wet-floor reflection lives below y=0.
    /// Every renderer mutation is restored even when Three.js throws during the draw.
    pub fn occlude(&self, camera: JsValue) -> Result<(), JsValue> {
        let s = self.state.borrow();
        if s.disposed || !s.ready {
            return Ok(());
        }
        let r = &s.renderer;
        let shadows = get(&get(r, "shadowMap"), "autoUpdate");
        let auto_clear = get(r, "autoClear");
        let background = get(&s.scene, "background");
        let override_material = get(&s.scene, "overrideMaterial");
        let mask = get(&get(&camera, "layers"), "mask");
        set(r, "autoClear", &false.into());
        set(&get(r, "shadowMap"), "autoUpdate", &false.into());
        call(&get(&camera, "layers"), "set", &[n(1.)]);
        set(&s.scene, "background", &JsValue::NULL);
        set(&s.scene, "overrideMaterial", &s.occlusion_material);
        let result = try_call(r, "render", &[s.scene.clone(), camera.clone()]);
        set(&s.scene, "overrideMaterial", &override_material);
        set(&s.scene, "background", &background);
        set(&get(&camera, "layers"), "mask", &mask);
        set(r, "autoClear", &auto_clear);
        set(&get(r, "shadowMap"), "autoUpdate", &shadows);
        result.map(|_| ())
    }

    pub fn render(&self, camera: JsValue, width: u32, height: u32) -> Result<JsValue, JsValue> {
        let s = self.state.borrow();
        if s.disposed || !s.ready {
            return Err(error("Projected stage is not ready"));
        }
        let width = width.max(1);
        let height = height.max(1);
        if num(&get(&s.color, "width")) != width as f64
            || num(&get(&s.color, "height")) != height as f64
        {
            call(&s.color, "setSize", &[width.into(), height.into()]);
            call(&s.depth, "setSize", &[width.into(), height.into()]);
        }
        let r = &s.renderer;
        let target = call(r, "getRenderTarget", &[]);
        let clear = call(r, "getClearColor", &[make(&s.three, "Color", &[])]);
        let alpha = call(r, "getClearAlpha", &[]);
        let tone = get(r, "toneMapping");
        let space = get(r, "outputColorSpace");
        let shadows = get(&get(r, "shadowMap"), "autoUpdate");
        let background = get(&s.scene, "background");
        let override_material = get(&s.scene, "overrideMaterial");
        let result = (|| {
            set(r, "toneMapping", &get(&s.three, "NoToneMapping"));
            set(
                r,
                "outputColorSpace",
                &get(&s.three, "LinearSRGBColorSpace"),
            );
            set(&get(r, "shadowMap"), "autoUpdate", &false.into());
            try_call(r, "setClearColor", &[n(0.), n(1.)])?;
            set(&s.scene, "background", &s.backdrop);
            try_call(r, "setRenderTarget", &[s.color.clone()])?;
            try_call(r, "clear", &[])?;
            try_call(r, "render", &[s.scene.clone(), camera.clone()])?;
            set(&s.scene, "background", &JsValue::NULL);
            set(&s.scene, "overrideMaterial", &s.depth_material);
            try_call(r, "setRenderTarget", &[s.depth.clone()])?;
            try_call(r, "clear", &[])?;
            try_call(r, "render", &[s.scene.clone(), camera])?;
            Ok(object(&[
                ("color", get(&s.color, "texture")),
                ("depth", get(&s.depth, "texture")),
            ]))
        })();
        set(&s.scene, "overrideMaterial", &override_material);
        set(&s.scene, "background", &background);
        set(r, "toneMapping", &tone);
        set(r, "outputColorSpace", &space);
        set(&get(r, "shadowMap"), "autoUpdate", &shadows);
        let restore_clear = try_call(r, "setClearColor", &[clear, alpha]);
        let restore_target = try_call(r, "setRenderTarget", &[target]);
        result.and_then(|value| {
            restore_clear?;
            restore_target?;
            Ok(value)
        })
    }

    pub fn dispose(&self) {
        self.state.borrow_mut().dispose();
    }

    /// The base has no battle lens: draw its photo-calibrated set directly.
    #[wasm_bindgen(js_name = drawToScreen)]
    pub fn draw_to_screen(&self, camera: JsValue) -> Result<(), JsValue> {
        let s = self.state.borrow();
        if s.disposed || !s.ready {
            return Ok(());
        }
        let target = call(&s.renderer, "getRenderTarget", &[]);
        let background = get(&s.scene, "background");
        set(&s.scene, "background", &JsValue::NULL);
        let result = try_call(&s.renderer, "setRenderTarget", &[JsValue::NULL])
            .and_then(|_| try_call(&s.renderer, "render", &[s.scene.clone(), camera]));
        set(&s.scene, "background", &background);
        let restore = try_call(&s.renderer, "setRenderTarget", &[target]);
        result.and(restore).map(|_| ())
    }
}

impl RustProjectedStage {
    /// The home camera consumes the same calibration and animation that Blender exports.
    pub(crate) fn manifest_data(&self) -> serde_json::Value {
        self.state.borrow().manifest.clone()
    }
}

impl Drop for RustProjectedStage {
    fn drop(&mut self) {
        if let Ok(mut s) = self.state.try_borrow_mut() {
            s.dispose();
        }
    }
}
impl StageState {
    fn clear_assets(&mut self) {
        self.ready = false;
        for mesh in self.meshes.drain(..) {
            release(&get(&mesh, "geometry"));
            release(&get(&mesh, "material"));
        }
        call(&self.root, "clear", &[]);
        for texture in self.textures.drain(..) {
            release(&texture);
        }
        set(&self.scene, "background", &JsValue::NULL);
        self.backdrop = JsValue::NULL;
    }
    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        self.ready = false;
        self.manifest = serde_json::Value::Null;
        self.request += 1;
        self.loading = None;
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
        self.clear_assets();
        for value in [
            &self.depth_material,
            &self.occlusion_material,
            &self.color,
            &self.depth,
        ] {
            release(value);
        }
    }
}

fn active(
    state: &Weak<RefCell<StageState>>,
    request: u64,
) -> Result<Rc<RefCell<StageState>>, JsValue> {
    let state = state
        .upgrade()
        .ok_or_else(|| error("Projected stage load cancelled"))?;
    {
        let s = state.borrow();
        if s.disposed || s.request != request {
            return Err(error("Projected stage load cancelled"));
        }
    }
    Ok(state)
}
fn resolved_url(url: &str, base: &str) -> Result<String, JsValue> {
    let constructor = get(&global(), "URL");
    let value = js_sys::Reflect::construct(
        constructor.unchecked_ref::<js_sys::Function>(),
        &array(&[url.into(), base.into()]),
    )?;
    Ok(string(&get(&value, "href")))
}

async fn load_assets(
    state: &Weak<RefCell<StageState>>,
    request: u64,
    url: &str,
    variant: &serde_json::Value,
    signal: Option<JsValue>,
) -> Result<(), JsValue> {
    let manifest_url = resolved_url(url, &string(&get(&document(), "baseURI")))?;
    let mut args = vec![manifest_url.clone().into()];
    if let Some(signal) = signal {
        args.push(object(&[("signal", signal)]));
    }
    let response = JsFuture::from(Promise::resolve(&try_call(&global(), "fetch", &args)?)).await?;
    if !boolean(&get(&response, "ok")) {
        return Err(error(&format!(
            "Stage manifest: {}",
            num(&get(&response, "status"))
        )));
    }
    let manifest = JsFuture::from(Promise::resolve(&try_call(&response, "json", &[])?)).await?;
    let manifest_data = data(&manifest);
    validate_manifest(&manifest_data)?;
    let (three, anisotropy) = {
        let current = active(state, request)?;
        let s = current.borrow();
        (
            s.three.clone(),
            num(&call(
                &get(&s.renderer, "capabilities"),
                "getMaxAnisotropy",
                &[],
            ))
            .min(8.),
        )
    };
    let loader = make(&three, "TextureLoader", &[]);
    let keys = ["clean", "equipment", "floor", "wall", "roof"];
    // Devices that draw the backdrop sharp take the double-size pictures where there are any.
    let sharp = super::platform::sharp_pictures();
    let mut urls = vec![];
    for key in keys {
        // Variant paths are page-relative, just like Frame.src. The shared
        // geometry's default materials remain relative to its own manifest.
        let replacement = variant["textures2x"][key]
            .as_str()
            .filter(|_| sharp)
            .or_else(|| variant["textures"][key].as_str());
        let double = manifest_data["textures2x"][key].as_str().filter(|_| sharp);
        let file = replacement.unwrap_or_else(|| {
            double.unwrap_or(manifest_data["textures"][key].as_str().unwrap_or(""))
        });
        urls.push(if file.is_empty() {
            None
        } else {
            let base = if replacement.is_some() {
                string(&get(&document(), "baseURI"))
            } else {
                manifest_url.clone()
            };
            Some(resolved_url(file, &base)?)
        });
    }
    // A Blender-baked set supplies a single atlas. Legacy battle scenes keep
    // their fixed-camera projection and do not allocate an additional texture.
    urls.push(
        manifest_data["bakedTexture"]
            .as_str()
            .map(|file| resolved_url(file, &manifest_url))
            .transpose()?,
    );
    let mut promises = vec![];
    for url in urls {
        promises.push(if let Some(url) = url {
            match try_call(&loader, "loadAsync", &[url.into()]) {
                Ok(request) => Promise::resolve(&request).into(),
                Err(reason) => Promise::reject(&reason).into(),
            }
        } else {
            Promise::resolve(&JsValue::NULL).into()
        });
    }
    // Wait for every texture, including failed siblings, so partial loads never leak.
    let settled = JsFuture::from(Promise::all_settled(&array(&promises))).await?;
    let mut textures = Vec::with_capacity(keys.len() + 1);
    let mut failure = None;
    for result in values(&settled) {
        if string(&get(&result, "status")) == "fulfilled" {
            textures.push(get(&result, "value"));
        } else {
            failure = Some(get(&result, "reason"));
            textures.push(JsValue::NULL);
        }
    }
    if let Err(cancelled) = active(state, request) {
        failure = Some(cancelled);
    }
    if let Some(failure) = failure {
        for texture in textures {
            release(&texture);
        }
        return Err(failure);
    }
    for (index, texture) in textures.iter().enumerate().filter(|(_, t)| present(t)) {
        set(texture, "colorSpace", &get(&three, "NoColorSpace"));
        set(texture, "anisotropy", &n(anisotropy));
        if (2..=4).contains(&index) {
            set(texture, "wrapS", &get(&three, "RepeatWrapping"));
            set(texture, "wrapT", &get(&three, "RepeatWrapping"));
        }
    }
    let c = &manifest_data["calibration"];
    let number = |key: &str| c[key].as_f64().unwrap();
    let near = 0.1;
    let focal = 0.5 / (number("fov").to_radians() / 2.).tan();
    let source = make(&three, "Matrix4", &[]);
    call(
        &source,
        "makePerspective",
        &[
            n(-near * number("aspect") / (2. * focal)),
            n(near * number("aspect") / (2. * focal)),
            n(near * number("horizon") / focal),
            n(-near * (1. - number("horizon")) / focal),
            n(near),
            n(200.),
        ],
    );
    let translation = make(&three, "Matrix4", &[]);
    call(
        &translation,
        "makeTranslation",
        &[n(0.), n(-number("camHeight")), n(0.)],
    );
    call(&source, "multiply", &[translation]);
    let current = active(state, request)?;
    let mut s = current.borrow_mut();
    s.textures = textures.iter().filter(|t| present(t)).cloned().collect();
    // Use the original image everywhere except the traced silhouettes that need
    // repair after their independent objects move. The mask is rasterised from
    // geometry outlines, not inferred from a depth map.
    let repair = manifest_data["repairPolygons"]
        .as_array()
        .filter(|p| !p.is_empty());
    let repair_mask = if let Some(polygons) = repair {
        let canvas = call(&document(), "createElement", &["canvas".into()]);
        let w = 1024.;
        let h = (w / number("aspect")).round();
        set(&canvas, "width", &n(w));
        set(&canvas, "height", &n(h));
        let ctx = call(&canvas, "getContext", &["2d".into()]);
        set(&ctx, "fillStyle", &"black".into());
        call(&ctx, "fillRect", &[n(0.), n(0.), n(w), n(h)]);
        set(&ctx, "fillStyle", &"white".into());
        set(&ctx, "strokeStyle", &"white".into());
        set(&ctx, "lineWidth", &n(2.));
        for polygon in polygons {
            call(&ctx, "beginPath", &[]);
            for (i, p) in polygon.as_array().unwrap().iter().enumerate() {
                call(
                    &ctx,
                    if i == 0 { "moveTo" } else { "lineTo" },
                    &[n(p[0].as_f64().unwrap() * w), n(p[1].as_f64().unwrap() * h)],
                );
            }
            call(&ctx, "closePath", &[]);
            call(&ctx, "fill", &[]);
            call(&ctx, "stroke", &[]);
        }
        let texture = make(&three, "CanvasTexture", &[canvas]);
        set(&texture, "colorSpace", &get(&three, "NoColorSpace"));
        s.textures.push(texture.clone());
        texture
    } else {
        textures[0].clone()
    };
    for part in manifest_data["meshes"].as_array().unwrap() {
        let geometry = make(&three, "BufferGeometry", &[]);
        let positions: Vec<f32> = part["positions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
        let attribute = make(
            &three,
            "Float32BufferAttribute",
            &[Float32Array::from(positions.as_slice()).into(), n(3.)],
        );
        call(&geometry, "setAttribute", &["position".into(), attribute]);
        let baked_uv = present(&textures[5]) && part["uvs"].is_array();
        if baked_uv {
            let uvs: Vec<f32> = part["uvs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap() as f32)
                .collect();
            let attribute = make(
                &three,
                "Float32BufferAttribute",
                &[Float32Array::from(uvs.as_slice()).into(), n(2.)],
            );
            call(&geometry, "setAttribute", &["uv".into(), attribute]);
        }
        call(&geometry, "setIndex", &[json(&part["indices"])]);
        call(&geometry, "computeBoundingSphere", &[]);
        let kind = part["material"].as_str().unwrap();
        let projected_side =
            kind == "side" && manifest_data["projectSides"].as_bool() == Some(true);
        let tile_index = match kind {
            "floor" => 2,
            "wall" => 3,
            "roof" => 4,
            _ => 0,
        };
        let has_tile = tile_index > 0 && present(&textures[tile_index]);
        let surface_kind = if has_tile {
            (tile_index - 1) as f64
        } else {
            0.
        };
        let color = part["color"]
            .as_array()
            .map(|v| {
                v.iter()
                    .map(|v| v.as_f64().unwrap_or(0.1))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![0.10, 0.11, 0.12]);
        let material = shader(
            &three,
            FRAGMENT,
            &[
                ("sourceViewProjection", source.clone()),
                ("useBakedUv", n(if baked_uv { 1. } else { 0. })),
                (
                    "bakedTexture",
                    textures[if baked_uv { 5 } else { 0 }].clone(),
                ),
                (
                    "photo",
                    textures[usize::from(kind == "equipment" || projected_side)].clone(),
                ),
                ("originalPhoto", textures[1].clone()),
                ("repairMask", repair_mask.clone()),
                (
                    "useRepair",
                    n(
                        if repair.is_some() && kind != "equipment" && kind != "side" {
                            1.
                        } else {
                            0.
                        },
                    ),
                ),
                (
                    "surfaceTile",
                    textures[if has_tile { tile_index } else { 0 }].clone(),
                ),
                ("surfaceKind", n(surface_kind)),
                (
                    "extensionTint",
                    make(
                        &three,
                        "Color",
                        &[n(
                            variant["extensionTint"].as_u64().unwrap_or(0xffffff) as f64
                        )],
                    ),
                ),
                (
                    "extensionExposure",
                    n(variant["extensionExposure"]
                        .as_f64()
                        .filter(|v| v.is_finite())
                        .unwrap_or(1.)
                        .clamp(0., 4.)),
                ),
                (
                    "isSide",
                    n(if kind == "side" && !projected_side {
                        1.
                    } else {
                        0.
                    }),
                ),
                (
                    "sideColor",
                    make(&three, "Vector3", &[n(color[0]), n(color[1]), n(color[2])]),
                ),
                (
                    "sideTint",
                    make(
                        &three,
                        "Color",
                        &[n(variant["sideTint"].as_u64().unwrap_or(0xffffff) as f64)],
                    ),
                ),
                (
                    "sideExposure",
                    n(variant["sideExposure"]
                        .as_f64()
                        .filter(|v| v.is_finite())
                        .unwrap_or(1.)
                        .clamp(0., 4.)),
                ),
            ],
        );
        let mesh = make(&three, "Mesh", &[geometry, material]);
        set(
            &mesh,
            "name",
            &part["name"].as_str().unwrap_or("stage").into(),
        );
        if part["occludesFigures"].as_bool() != Some(false) {
            call(&get(&mesh, "layers"), "enable", &[n(1.)]);
        }
        call(&s.root, "add", &[mesh.clone()]);
        s.meshes.push(mesh);
    }
    s.backdrop = textures[0].clone();
    s.manifest = manifest_data;
    s.ready = true;
    Ok(())
}

fn validate_manifest(data: &serde_json::Value) -> Result<(), JsValue> {
    let invalid = || error("Invalid projected stage manifest");
    if data["version"].as_u64() != Some(1) {
        return Err(invalid());
    }
    if let Some(atlas) = data.get("bakedTexture") {
        if !atlas.as_str().is_some_and(|s| !s.trim().is_empty()) {
            return Err(invalid());
        }
    }
    let c = &data["calibration"];
    for key in ["fov", "aspect", "camHeight", "horizon"] {
        if !c[key].as_f64().is_some_and(f64::is_finite) {
            return Err(invalid());
        }
    }
    if !(1.0..179.0).contains(&c["fov"].as_f64().unwrap())
        || c["aspect"].as_f64().unwrap() <= 0.
        || !(0.0..1.0).contains(&c["horizon"].as_f64().unwrap())
    {
        return Err(invalid());
    }
    for key in ["clean", "equipment"] {
        if data["textures"][key].as_str().unwrap_or("").is_empty() {
            return Err(invalid());
        }
    }
    if let Some(polygons) = data.get("repairPolygons") {
        let polygons = polygons.as_array().ok_or_else(invalid)?;
        if polygons.len() > 128 {
            return Err(invalid());
        }
        for polygon in polygons {
            let points = polygon
                .as_array()
                .filter(|p| p.len() >= 3 && p.len() <= 128)
                .ok_or_else(invalid)?;
            for point in points {
                let point = point
                    .as_array()
                    .filter(|p| p.len() == 2)
                    .ok_or_else(invalid)?;
                if point.iter().any(|v| {
                    !v.as_f64()
                        .is_some_and(|v| v.is_finite() && (0. ..=1.).contains(&v))
                }) {
                    return Err(invalid());
                }
            }
        }
    }
    let parts = data["meshes"]
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or_else(invalid)?;
    for part in parts {
        let positions = part["positions"].as_array().ok_or_else(invalid)?;
        if positions.is_empty()
            || positions.len() % 3 != 0
            || positions.iter().any(|v| {
                !v.as_f64()
                    .is_some_and(|x| x.is_finite() && x.abs() <= f32::MAX as f64)
            })
        {
            return Err(invalid());
        }
        if let Some(uvs) = part.get("uvs") {
            let uvs = uvs.as_array().ok_or_else(invalid)?;
            if uvs.len() != positions.len() / 3 * 2
                || uvs.iter().any(|v| {
                    !v.as_f64()
                        .is_some_and(|x| x.is_finite() && x.abs() <= f32::MAX as f64)
                })
            {
                return Err(invalid());
            }
        }
        let indices = part["indices"].as_array().ok_or_else(invalid)?;
        if indices.is_empty()
            || indices.len() % 3 != 0
            || indices
                .iter()
                .any(|v| !v.as_u64().is_some_and(|i| i < (positions.len() / 3) as u64))
        {
            return Err(invalid());
        }
        if !matches!(
            part["material"].as_str(),
            Some("clean" | "equipment" | "side" | "floor" | "wall" | "roof")
        ) {
            return Err(invalid());
        }
        if let Some(color) = part["color"].as_array() {
            if color.len() != 3
                || color
                    .iter()
                    .any(|v| !v.as_f64().is_some_and(f64::is_finite))
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}
