//! Authored Persona preview: its pose remains unchanged; Rust controls the scene.
use super::view_utils::*;
use js_sys::{Array, Object, Promise};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, spawn_local};

#[wasm_bindgen]
pub struct RustPersonaViewer {
    state: JsValue,
}
#[wasm_bindgen]
impl RustPersonaViewer {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, canvas: JsValue) -> Self {
        let stage = call(&host, "makeFittingStage", &[]);
        let equipment = make(&host, "THREE.Group", &[]);
        let scene = make(&host, "THREE.Scene", &[]);
        let key = make(
            &host,
            "THREE.DirectionalLight",
            &[num(0xe8eef2 as f64), num(3.2)],
        );
        let hemisphere = make(
            &host,
            "THREE.HemisphereLight",
            &[num(0xc3cacd as f64), num(0x2b2d2e as f64), num(0.6)],
        );
        let s = options(&[
            ("host", host.clone()),
            ("canvas", canvas.clone()),
            ("stage", stage.clone()),
            ("equipment", equipment.clone()),
            ("scene", scene.clone()),
            ("key", key.clone()),
            ("hemisphere", hemisphere),
            (
                "camera",
                make(
                    &host,
                    "THREE.PerspectiveCamera",
                    &[num(33.0), num(1.0), num(0.05), num(40.0)],
                ),
            ),
            ("quality", get(&host, "QUALITY.medium")),
            ("width", num(1.0)),
            ("height", num(1.0)),
            ("distance", num(3.0)),
            ("movedX", num(0.0)),
            ("movedY", num(0.0)),
            ("listeners", array(&[])),
            ("originalTextures", array(&[])),
            (
                "texturesPending",
                Promise::resolve(&JsValue::UNDEFINED).into(),
            ),
            ("previousTouchAction", get(&canvas, "style.touchAction")),
        ]);
        set(&canvas, "style.touchAction", &text("pan-y"));
        for event in [
            "pointerdown",
            "pointermove",
            "pointerup",
            "pointercancel",
            "lostpointercapture",
            "webglcontextlost",
            "webglcontextrestored",
        ] {
            let a = s.clone();
            listen(&s, &canvas, event, function(move |e| input(&a, event, &e)));
        }
        set(&stage, "rotating.rotation.y", &num(-0.16));
        set(&equipment, "name", &text("Body_Mounted_Equipment"));
        set(&equipment, "position.y", &get(&host, "PLATFORM_TOP"));
        call(&get(&stage, "rotating"), "add", &[equipment]);
        call(
            &scene,
            "add",
            &[
                get(&stage, "fixed"),
                get(&stage, "rotating"),
                get(&s, "hemisphere"),
            ],
        );
        call(
            &get(&key, "position"),
            "set",
            &[num(-2.4), num(4.2), num(2.7)],
        );
        call(
            &get(&key, "target.position"),
            "set",
            &[num(0.0), num(0.9), num(0.0)],
        );
        for (name, v) in [
            ("left", -1.7),
            ("bottom", -1.7),
            ("right", 1.7),
            ("top", 1.7),
            ("near", 0.5),
            ("far", 10.0),
        ] {
            set(&get(&key, "shadow.camera"), name, &num(v));
        }
        set(&key, "shadow.bias", &num(-0.00015));
        set(&key, "shadow.normalBias", &num(0.014));
        call(&scene, "add", &[key.clone(), get(&key, "target")]);
        for (color, power, pos) in [
            (0xc7ccd0, 0.85, [2.4, 1.9, 2.1]),
            (0xe8eef2, 2.0, [1.4, 2.7, -1.8]),
        ] {
            let light = make(
                &host,
                "THREE.DirectionalLight",
                &[num(color as f64), num(power)],
            );
            call(&get(&light, "position"), "fromArray", &[floats(&pos)]);
            call(&scene, "add", &[light]);
        }
        for x in [-1.3, 1.3] {
            let light = make(
                &host,
                "THREE.PointLight",
                &[num(0xe4cbb0 as f64), num(0.7), num(3.5), num(2.0)],
            );
            call(
                &get(&light, "position"),
                "set",
                &[num(x), num(0.65), num(0.25)],
            );
            call(&scene, "add", &[light]);
        }
        Self { state: s }
    }
    #[wasm_bindgen(getter,js_name=previewDescription)]
    pub fn preview_description(&self) -> String {
        "3D: комплект A и Варяг · остальные предметы пока показаны условно".into()
    }
    pub fn load(&self) -> Promise {
        let s = &self.state;
        if yes(s, "disposed") {
            return Promise::reject(&error("Просмотр 3D уже закрыт."));
        }
        if exists(&get(s, "pending")) {
            return get(s, "pending").into();
        }
        let a = s.clone();
        let p = future_to_promise(async move {
            match load_models(&a).await {
                Ok(()) => Ok(JsValue::UNDEFINED),
                Err(_) => {
                    if yes(&a, "disposed") {
                        return Ok(JsValue::UNDEFINED);
                    }
                    release_gpu(&a);
                    Err(error(
                        "Не удалось загрузить 3D-модель. Проверьте соединение и откройте примерочную ещё раз.",
                    ))
                }
            }
        });
        set(s, "pending", &p);
        p
    }
    pub fn resize(&self, width: f64, height: f64) {
        resize(&self.state, width, height);
    }
    #[wasm_bindgen(js_name=setActive)]
    pub fn set_active(&self, active: bool) {
        let s = &self.state;
        if yes(s, "disposed") {
            return;
        }
        set(s, "active", &flag(active));
        if active {
            let before = string(s, "quality.id");
            configure_quality(s);
            if before != string(s, "quality.id") {
                let pending = update_textures(s);
                spawn_local(async move {
                    let _ = wait(pending.into()).await;
                });
                resize(s, number(s, "width"), number(s, "height"));
            }
            render(s);
        } else {
            cancel_frame(s);
            set(s, "drag", &JsValue::NULL);
        }
    }
    pub fn shift(&self, x: f64, y: f64) {
        let s = &self.state;
        if (x - number(s, "movedX")).abs() + (y - number(s, "movedY")).abs() < 0.05 {
            return;
        }
        set(s, "movedX", &num(x));
        set(s, "movedY", &num(y));
        aim(s);
        render(s);
    }
    pub fn rotate(&self, delta: f64) {
        rotate(&self.state, delta);
    }
    #[wasm_bindgen(js_name=updateLoadout)]
    pub fn update_loadout(&self, loadout: JsValue) -> JsValue {
        let s = &self.state;
        set(s, "loadout", &loadout);
        apply_weapon(s);
        render(s);
        graphics(
            "graphics.fittingCoverage",
            &options(&[
                ("loadout", loadout),
                ("weapon", text(&weapon(s))),
                ("hasRifleModel", flag(exists(&get(s, "varyag")))),
            ]),
        )
    }
    pub fn dispose(&self) {
        let s = &self.state;
        if yes(s, "disposed") {
            return;
        }
        set(s, "disposed", &flag(true));
        set(s, "active", &flag(false));
        cancel_frame(s);
        let canvas = get(s, "canvas");
        let id = get(s, "drag.id");
        if exists(&id) && call(&canvas, "hasPointerCapture", &[id.clone()]).is_truthy() {
            call(&canvas, "releasePointerCapture", &[id]);
        }
        set(s, "drag", &JsValue::NULL);
        unlisten(s);
        set(&canvas, "style.touchAction", &get(s, "previousTouchAction"));
        release_gpu(s);
    }
}
async fn load_models(s: &JsValue) -> Result<(), JsValue> {
    let h = get(s, "host");
    let renderer = try_make(
        &h,
        "THREE.WebGLRenderer",
        &[options(&[
            ("canvas", get(s, "canvas")),
            ("alpha", flag(true)),
            ("antialias", flag(true)),
            ("powerPreference", text("high-performance")),
        ])],
    )?;
    set(s, "renderer", &renderer);
    call(&renderer, "setClearColor", &[num(0.0), num(0.0)]);
    set(
        &renderer,
        "outputColorSpace",
        &get(&h, "THREE.SRGBColorSpace"),
    );
    set(
        &renderer,
        "toneMapping",
        &get(&h, "THREE.ACESFilmicToneMapping"),
    );
    set(&renderer, "toneMappingExposure", &num(1.05));
    set(&renderer, "shadowMap.type", &get(&h, "THREE.PCFShadowMap"));
    configure_quality(s);
    resize(s, number(s, "width"), number(s, "height"));
    let pmrem = make(&h, "THREE.PMREMGenerator", &[renderer]);
    let room = make(&h, "RoomEnvironment", &[]);
    let environment = call(&pmrem, "fromScene", &[room.clone(), num(0.06)]);
    set(s, "environment", &environment);
    set(
        &get(s, "scene"),
        "environment",
        &if yes(s, "quality.env") {
            get(&environment, "texture")
        } else {
            JsValue::NULL
        },
    );
    call(&room, "dispose", &[]);
    call(&pmrem, "dispose", &[]);
    let a = s.clone();
    let gun = future_to_promise(async move {
        let h = get(&a, "host");
        if let Ok(p) = try_call(
            &h,
            "loadModel",
            &[text(&format!("{}preview/varyag.json", string(&h, "base")))],
        ) {
            if let Ok(gltf) = wait(p).await {
                let root = get(&gltf, "scene");
                if yes(&a, "disposed") || !exists(&get(&a, "renderer")) {
                    dispose(&root, &[], true);
                } else {
                    set(&a, "varyag", &root);
                    call(&h, "prepareVaryag", &[root.clone()]);
                    call(&get(&a, "equipment"), "add", &[root.clone()]);
                    tune(&a, &root);
                }
            }
        }
        Ok(JsValue::UNDEFINED)
    });
    let a = s.clone();
    let room = future_to_promise(async move {
        load_environment(&a).await;
        Ok(JsValue::UNDEFINED)
    });
    let url = format!("{}preview/persona-authored.json", string(&h, "base"));
    let gltf = wait(try_call(&h, "loadModel", &[text(&url)])?).await?;
    let figure = get(&gltf, "scene");
    if yes(s, "disposed") {
        dispose(&figure, &[], true);
        return Ok(());
    }
    set(s, "figure", &figure);
    // This export includes the user's exact mesh, static pose, and platform height.
    let stand_ins = make(&h, "THREE.Group", &[]);
    let anchor = make(&h, "THREE.Group", &[]);
    set(&anchor, "name", &text("hand_r"));
    call(&stand_ins, "add", &[anchor]);
    let material = make(
        &h,
        "THREE.MeshStandardMaterial",
        &[options(&[
            ("color", num(0x1b1c1d as f64)),
            ("roughness", num(0.5)),
            ("metalness", num(0.55)),
        ])],
    );
    let hand = make(
        &h,
        "Hand",
        &[stand_ins, make(&h, "THREE.Matrix4", &[]), material],
    );
    set(s, "hand", &hand);
    call(&get(s, "equipment"), "add", &[get(&hand, "mount")]);
    set(&figure, "name", &text("Persona_Authored_In_Blender"));
    for n in nodes(&figure, false) {
        if yes(&n, "isMesh") {
            set(&n, "frustumCulled", &flag(false));
            set(&n, "castShadow", &flag(true));
            set(&n, "receiveShadow", &flag(true));
        }
    }
    call(&get(s, "stage.rotating"), "add", &[figure.clone()]);
    set(s, "originalTextures", &array(&textures(&figure)));
    set(
        s,
        "tiers",
        &make(&h, "TextureTiers", &[array(&[figure]), text(&url)]),
    );
    wait(update_textures(s).into()).await?;
    if yes(s, "disposed") {
        return Ok(());
    }
    wait(Promise::all(&array(&[gun.into(), room.into()])).into()).await?;
    if yes(s, "disposed") {
        return Ok(());
    }
    apply_weapon(s);
    tune(s, &get(s, "scene"));
    set(s, "ready", &flag(true));
    resize(s, number(s, "width"), number(s, "height"));
    render(s);
    Ok(())
}
async fn load_environment(s: &JsValue) {
    let h = get(s, "host");
    let loader = make(&h, "THREE.TextureLoader", &[]);
    let Ok(p) = try_call(
        &loader,
        "loadAsync",
        &[text(&format!(
            "{}img/base/fitting-room-v2.png",
            string(&h, "base")
        ))],
    ) else {
        return;
    };
    let Ok(texture) = wait(p).await else {
        return;
    };
    if !yes(s, "disposed") && exists(&get(s, "renderer")) {
        set(&texture, "colorSpace", &get(&h, "THREE.SRGBColorSpace"));
        set(
            &texture,
            "mapping",
            &get(&h, "THREE.EquirectangularReflectionMapping"),
        );
        let pmrem = make(&h, "THREE.PMREMGenerator", &[get(s, "renderer")]);
        if let Ok(next) = try_call(&pmrem, "fromEquirectangular", &[texture.clone()]) {
            method_if(&get(s, "environment"), "dispose", &[]);
            set(s, "environment", &next);
            set(
                &get(s, "scene"),
                "environment",
                &if yes(s, "quality.env") {
                    get(&next, "texture")
                } else {
                    JsValue::NULL
                },
            );
        }
        call(&pmrem, "dispose", &[]);
    }
    call(&texture, "dispose", &[]);
}
fn configure_quality(s: &JsValue) {
    let renderer = get(s, "renderer");
    if !exists(&renderer) {
        return;
    }
    let h = get(s, "host");
    let selected = call(&h, "startQuality", &[call(&renderer, "getContext", &[])]);
    let q = get(&get(&h, "QUALITY"), &string(&selected, "id"));
    set(s, "quality", &q);
    call(&get(s, "stage"), "setQuality", &[get(&q, "id")]);
    set(&get(s, "canvas"), "dataset.previewQuality", &get(&q, "id"));
    set(
        &renderer,
        "shadowMap.enabled",
        &flag(number(&q, "shadow") > 0.0),
    );
    set(
        &get(s, "key"),
        "castShadow",
        &flag(number(&q, "shadow") > 0.0),
    );
    let shadow = get(s, "key.shadow");
    if number(&shadow, "mapSize.x") != number(&q, "shadow") && exists(&get(&shadow, "map")) {
        call(&get(&shadow, "map"), "dispose", &[]);
        set(&shadow, "map", &JsValue::NULL);
    }
    call(
        &get(&shadow, "mapSize"),
        "setScalar",
        &[num(number(&q, "shadow").max(1.0))],
    );
    set(&shadow, "radius", &get(&q, "shadowRadius"));
    set(
        &get(s, "hemisphere"),
        "intensity",
        &num(if yes(&q, "env") { 0.52 } else { 0.78 }),
    );
    let texture = get(s, "environment.texture");
    set(
        &get(s, "scene"),
        "environment",
        &if yes(&q, "env") && exists(&texture) {
            texture
        } else {
            JsValue::NULL
        },
    );
    tune(s, &get(s, "scene"));
}
fn anisotropy(s: &JsValue) -> f64 {
    let caps = get(s, "renderer.capabilities");
    number(s, "quality.anisotropy").min(if exists(&caps) {
        call(&caps, "getMaxAnisotropy", &[]).as_f64().unwrap_or(1.0)
    } else {
        1.0
    })
}
fn tune(s: &JsValue, root: &JsValue) {
    let value = anisotropy(s);
    for m in materials(root) {
        call(
            &get(s, "host"),
            "tuneFittingMaterial",
            &[m.clone(), get(s, "quality")],
        );
        set(&m, "alphaToCoverage", &flag(number(&m, "alphaTest") > 0.0));
    }
    for t in textures(root) {
        set(&t, "anisotropy", &num(value));
    }
}
fn update_textures(s: &JsValue) -> Promise {
    let tiers = get(s, "tiers");
    if !exists(&tiers) || !exists(&get(s, "renderer")) {
        return Promise::resolve(&JsValue::UNDEFINED);
    }
    let prior = get(s, "texturesPending");
    let size = get(s, "quality.textures");
    let aniso = anisotropy(s);
    let a = s.clone();
    let p = future_to_promise(async move {
        let _ = wait(prior).await;
        if !yes(&a, "disposed") {
            if let Ok(p) = try_call(&tiers, "set", &[size, num(aniso)]) {
                if wait(p).await.is_ok() && !yes(&a, "disposed") {
                    set(
                        &get(&a, "canvas"),
                        "dataset.previewTextureSize",
                        &text(&format!("{}", number(&tiers, "current"))),
                    );
                    render(&a);
                }
            }
        }
        Ok(JsValue::UNDEFINED)
    });
    set(s, "texturesPending", &p);
    p
}
fn resize(s: &JsValue, width: f64, height: f64) {
    if yes(s, "disposed") {
        return;
    }
    let args = Object::assign(&Object::new(), &get(s, "quality").into());
    let args: JsValue = args.into();
    for (name, v) in [
        ("width", width),
        ("height", height),
        ("fov", number(s, "camera.fov")),
        ("dpr", number(&global(), "devicePixelRatio").max(1.0)),
    ] {
        set(&args, name, &num(v));
    }
    let fit = graphics("graphics.fittingCamera", &args);
    set(s, "width", &get(&fit, "width"));
    set(s, "height", &get(&fit, "height"));
    set(&get(s, "camera"), "aspect", &get(&fit, "aspect"));
    call(&get(s, "camera"), "updateProjectionMatrix", &[]);
    set(s, "distance", &get(&fit, "distance"));
    aim(s);
    let renderer = get(s, "renderer");
    if exists(&renderer) {
        call(&renderer, "setPixelRatio", &[get(&fit, "ratio")]);
        call(
            &renderer,
            "setSize",
            &[get(s, "width"), get(s, "height"), flag(false)],
        );
        set(
            &get(s, "canvas"),
            "dataset.previewDpr",
            &text(&format!("{:.2}", number(&fit, "ratio"))),
        );
    }
    render(s);
}
fn aim(s: &JsValue) {
    let fit = graphics(
        "graphics.fittingAim",
        &options(&[
            ("distance", get(s, "distance")),
            ("height", get(s, "height")),
            ("fov", get(s, "camera.fov")),
            ("x", get(s, "movedX")),
            ("y", get(s, "movedY")),
        ]),
    );
    call(
        &get(s, "camera.position"),
        "fromArray",
        &[get(&fit, "position")],
    );
    call(
        &get(s, "camera"),
        "lookAt",
        &Array::from(&get(&fit, "target")).iter().collect::<Vec<_>>(),
    );
    call(&get(s, "camera"), "updateMatrixWorld", &[]);
}
fn rotate(s: &JsValue, delta: f64) {
    if yes(s, "disposed") || !delta.is_finite() {
        return;
    }
    set(
        &get(s, "stage.rotating"),
        "rotation.y",
        &graphics(
            "graphics.fittingRotation",
            &options(&[
                ("angle", get(s, "stage.rotating.rotation.y")),
                ("delta", num(delta)),
            ]),
        ),
    );
    render(s);
}
fn weapon(s: &JsValue) -> String {
    let id = get(s, "loadout.weapon1.item");
    if !exists(&id) {
        return "rifle".into();
    }
    let item = call(&get(s, "host"), "getItem", &[id]);
    if string(&item, "kind") == "weapon" {
        string(&item, "weapon")
    } else {
        "rifle".into()
    }
}
fn apply_weapon(s: &JsValue) {
    if !exists(&get(s, "figure")) {
        return;
    }
    let h = get(s, "host");
    let weapon = weapon(s);
    let varyag = get(s, "varyag");
    let real = weapon == "rifle" && exists(&varyag);
    method_if(
        &get(s, "hand"),
        "hold",
        &[if real { JsValue::NULL } else { text(&weapon) }],
    );
    if exists(&varyag) {
        set(&varyag, "visible", &flag(real));
        let loadout = get(s, "loadout");
        call(
            &h,
            "fitVaryag",
            &[
                varyag.clone(),
                if exists(&loadout) {
                    loadout
                } else {
                    JsValue::NULL
                },
            ],
        );
    }
    let mount = if real { varyag } else { get(s, "hand.mount") };
    if !exists(&mount) {
        return;
    }
    let pose = graphics(
        "graphics.equipmentPose",
        &options(&[("weapon", text(&weapon))]),
    );
    call(&get(&mount, "scale"), "setScalar", &[num(1.0)]);
    call(
        &get(&mount, "position"),
        "fromArray",
        &[get(&pose, "position")],
    );
    call(
        &get(&mount, "rotation"),
        "set",
        &Array::from(&get(&pose, "rotation"))
            .iter()
            .collect::<Vec<_>>(),
    );
    let old = get(s, "sling");
    if exists(&old) {
        call(&old, "removeFromParent", &[]);
        dispose(&old, &[], true);
    }
    let sling = if real {
        call(&h, "authoredWeaponFit", &[mount])
    } else {
        call(&h, "makeSling", &[mount, flag(weapon == "pistol")])
    };
    set(s, "sling", &sling);
    call(&get(s, "equipment"), "add", &[sling]);
    tune(s, &get(s, "equipment"));
}
fn cancel_frame(s: &JsValue) {
    call(&global(), "cancelAnimationFrame", &[get(s, "frame")]);
    set(s, "frame", &num(0.0));
    set(s, "frameCallback", &JsValue::NULL);
}
fn render(s: &JsValue) {
    if yes(s, "disposed") || !yes(s, "ready") || !yes(s, "active") || number(s, "frame") != 0.0 {
        return;
    }
    let a = s.clone();
    let callback = function(move |_| {
        set(&a, "frame", &num(0.0));
        set(&a, "frameCallback", &JsValue::NULL);
        if yes(&a, "active") && !yes(&a, "disposed") {
            method_if(
                &get(&a, "renderer"),
                "render",
                &[get(&a, "scene"), get(&a, "camera")],
            );
        }
    });
    set(s, "frameCallback", &callback);
    set(
        s,
        "frame",
        &call(&global(), "requestAnimationFrame", &[callback]),
    );
}
fn release_gpu(s: &JsValue) {
    if yes(s, "gpuReleased") {
        return;
    }
    set(s, "gpuReleased", &flag(true));
    let roots = make(&get(s, "host"), "THREE.Group", &[]);
    call(
        &roots,
        "add",
        &[get(s, "stage.fixed"), get(s, "stage.rotating")],
    );
    let pending = get(s, "texturesPending");
    let extra = Array::from(&get(s, "originalTextures"))
        .iter()
        .collect::<Vec<_>>();
    spawn_local(async move {
        let _ = wait(pending).await;
        dispose(&roots, &extra, true);
    });
    for name in ["figure", "hand", "varyag", "tiers", "sling"] {
        set(s, name, &JsValue::NULL);
    }
    set(&get(s, "scene"), "environment", &JsValue::NULL);
    method_if(&get(s, "environment"), "dispose", &[]);
    set(s, "environment", &JsValue::NULL);
    call(&get(s, "key.shadow"), "dispose", &[]);
    method_if(&get(s, "renderer"), "dispose", &[]);
    set(s, "renderer", &JsValue::NULL);
    set(s, "ready", &flag(false));
}
fn input(s: &JsValue, event: &str, e: &JsValue) {
    let canvas = get(s, "canvas");
    match event {
        "pointerdown" => {
            if !yes(s, "active") || number(e, "button") != 0.0 || !yes(e, "isPrimary") {
                return;
            }
            set(
                s,
                "drag",
                &options(&[("id", get(e, "pointerId")), ("x", get(e, "clientX"))]),
            );
            call(&canvas, "setPointerCapture", &[get(e, "pointerId")]);
        }
        "pointermove" => {
            if !exists(&get(s, "drag")) || get(s, "drag.id") != get(e, "pointerId") {
                return;
            }
            rotate(s, (number(e, "clientX") - number(s, "drag.x")) * 0.009);
            set(s, "drag.x", &get(e, "clientX"));
        }
        "pointerup" | "pointercancel" | "lostpointercapture" => {
            if get(s, "drag.id") != get(e, "pointerId") {
                return;
            }
            set(s, "drag", &JsValue::NULL);
            if call(&canvas, "hasPointerCapture", &[get(e, "pointerId")]).is_truthy() {
                call(&canvas, "releasePointerCapture", &[get(e, "pointerId")]);
            }
        }
        "webglcontextlost" => {
            call(e, "preventDefault", &[]);
            cancel_frame(s);
            let event = make(
                &global(),
                "CustomEvent",
                &[
                    text("persona-preview-error"),
                    options(&[
                        (
                            "detail",
                            text("3D-просмотр временно недоступен: графический контекст потерян."),
                        ),
                        ("bubbles", flag(true)),
                    ]),
                ],
            );
            call(&canvas, "dispatchEvent", &[event]);
        }
        "webglcontextrestored" => {
            configure_quality(s);
            render(s);
        }
        _ => {}
    }
}
