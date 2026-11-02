//! Loading presentation driven by arena readiness and browser visibility.
use super::view_utils::*;
use js_sys::{Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, spawn_local};

#[wasm_bindgen]
pub struct RustMawLoader {
    state: JsValue,
}
#[wasm_bindgen]
impl RustMawLoader {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue) -> Self {
        let dialog = call(&document(), "createElement", &[text("dialog")]);
        set(&dialog, "className", &text("cd-maw-loader"));
        attr(&dialog, "aria-label", "Загрузка боя");
        set(
            &dialog,
            "innerHTML",
            &text(&format!(
                r#"<div class="cd-maw-inner">
          <div class="cd-maw-stage" aria-hidden="true"><canvas></canvas></div>
          <div class="cd-maw-hud"><h1 class="cd-maw-word"><img src="{}brand/wordmark.svg" alt="CROWNED DOGS"></h1>
            <div class="cd-maw-progress" role="progressbar" aria-label="Загрузка арены"></div>
            <p class="cd-maw-status" role="status">Загрузка арены…</p>
            <div class="cd-maw-actions"><button class="glass cd-maw-retry" type="button" hidden>Повторить</button><button class="glass cd-maw-back" type="button">На базу</button></div>
          </div></div>"#,
                string(&host, "base")
            )),
        );
        let stage = query(&dialog, ".cd-maw-stage");
        let canvas = query(&dialog, "canvas");
        let reduced = yes(
            &call(
                &global(),
                "matchMedia",
                &[text("(prefers-reduced-motion: reduce)")],
            ),
            "matches",
        );
        let s = options(&[
            ("host", host.clone()),
            ("dialog", dialog.clone()),
            ("stage", stage.clone()),
            ("canvas", canvas.clone()),
            ("status", query(&dialog, ".cd-maw-status")),
            ("retry", query(&dialog, ".cd-maw-retry")),
            ("back", query(&dialog, ".cd-maw-back")),
            ("reduced", flag(reduced)),
            ("radius", num(1.6)),
            ("generation", num(0.0)),
            ("listeners", array(&[])),
        ]);
        let a = s.clone();
        listen(
            &s,
            &dialog,
            "cancel",
            function(move |e| {
                call(&e, "preventDefault", &[]);
                call(&get(&a, "back"), "click", &[]);
            }),
        );
        let a = s.clone();
        listen(
            &s,
            &canvas,
            "webglcontextlost",
            function(move |e| {
                call(&e, "preventDefault", &[]);
                class(&get(&a, "stage"), "remove", "is-3d");
                stop_animation(&a);
            }),
        );
        let a = s.clone();
        listen(
            &s,
            &canvas,
            "webglcontextrestored",
            function(move |_| {
                if yes(&a, "shown") && exists(&get(&a, "pivot")) {
                    class(&get(&a, "stage"), "add", "is-3d");
                    start_animation(&a);
                }
            }),
        );
        call(&get(&document(), "body"), "appendChild", &[dialog.clone()]);
        call(&host, "liquidGlass", &[dialog]);
        let a = s.clone();
        observe(&s, &stage, function(move |_| resize(&a)));
        let a = s.clone();
        listen(
            &s,
            &document(),
            "visibilitychange",
            function(move |_| {
                if yes(&document(), "hidden") {
                    stop_animation(&a);
                } else if yes(&a, "shown") {
                    start_animation(&a);
                }
            }),
        );
        Self { state: s }
    }
    pub fn show(&self, on_back: JsValue) {
        let s = &self.state;
        if yes(s, "disposed") {
            return;
        }
        set(s, "shown", &flag(true));
        class(&get(s, "dialog"), "remove", "is-ready");
        class(&get(s, "dialog"), "remove", "is-error");
        set(&get(s, "retry"), "hidden", &flag(true));
        set(&get(s, "status"), "textContent", &text("Загрузка арены…"));
        attr(&get(s, "dialog"), "aria-busy", "true");
        set(&get(s, "back"), "onclick", &on_back);
        if !yes(s, "dialog.open") {
            call(&get(s, "dialog"), "showModal", &[]);
        }
        focus(&get(s, "back"));
        if exists(&get(s, "pivot")) {
            resize(s);
            start_animation(s);
        } else if exists(&get(s, "pending")) {
            let a = s.clone();
            spawn_local(async move {
                let _ = wait(get(&a, "pending")).await;
                if yes(&a, "shown") && !exists(&get(&a, "renderer")) {
                    begin(&a);
                }
            });
        } else {
            begin(s);
        }
    }
    pub fn ready(&self) -> Promise {
        let s = &self.state;
        attr(&get(s, "dialog"), "aria-busy", "false");
        set(&get(s, "status"), "textContent", &text("Арена готова"));
        class(&get(s, "dialog"), "add", "is-ready");
        if yes(s, "reduced") || !yes(s, "shown") {
            hide(s);
            return Promise::resolve(&JsValue::UNDEFINED);
        }
        Promise::new(&mut |resolve, _| {
            set(s, "finishResolve", &resolve);
            let a = s.clone();
            let callback = function(move |_| {
                set(&a, "finishTimer", &JsValue::NULL);
                set(&a, "finishCallback", &JsValue::NULL);
                let resolve = get(&a, "finishResolve");
                set(&a, "finishResolve", &JsValue::NULL);
                hide(&a);
                if exists(&resolve) {
                    invoke(&resolve, &JsValue::UNDEFINED, &[]);
                }
            });
            set(s, "finishCallback", &callback);
            set(
                s,
                "finishTimer",
                &call(&global(), "setTimeout", &[callback, num(360.0)]),
            );
        })
    }
    /// `detail` is the error's own words, shown small under the message: a phone has no console.
    pub fn failed(&self, detail: Option<String>) -> Promise {
        let s = &self.state;
        class(&get(s, "dialog"), "remove", "is-ready");
        class(&get(s, "dialog"), "add", "is-error");
        attr(&get(s, "dialog"), "aria-busy", "false");
        set(
            &get(s, "status"),
            "textContent",
            &text("Не удалось загрузить арену"),
        );
        if let Some(detail) = detail.filter(|d| !d.is_empty()) {
            let small = call(&get(&global(), "document"), "createElement", &[text("small")]);
            set(&small, "textContent", &text(&detail.chars().take(160).collect::<String>()));
            set(&small, "style.cssText", &text("display:block;margin-top:6px;opacity:.6;font-size:11px;word-break:break-word"));
            call(&get(s, "status"), "append", &[small]);
        }
        set(&get(s, "retry"), "hidden", &flag(false));
        stop_animation(s);
        focus(&get(s, "retry"));
        Promise::new(&mut |resolve, _| {
            let f: JsValue = resolve.clone().into();
            set(
                &get(s, "retry"),
                "onclick",
                &function(move |_| {
                    invoke(&f, &JsValue::UNDEFINED, &[text("retry")]);
                }),
            );
            let f: JsValue = resolve.into();
            set(
                &get(s, "back"),
                "onclick",
                &function(move |_| {
                    invoke(&f, &JsValue::UNDEFINED, &[text("back")]);
                }),
            );
        })
    }
    pub fn hide(&self) {
        hide(&self.state);
    }
    pub fn dispose(&self) {
        let s = &self.state;
        if yes(s, "disposed") {
            return;
        }
        set(s, "disposed", &flag(true));
        hide(s);
        method_if(&get(s, "observer"), "disconnect", &[]);
        unlisten(s);
        set(&get(s, "back"), "onclick", &JsValue::NULL);
        set(&get(s, "retry"), "onclick", &JsValue::NULL);
        call(&get(s, "dialog"), "remove", &[]);
    }
}
fn focus(element: &JsValue) {
    call(
        element,
        "focus",
        &[options(&[("preventScroll", flag(true))])],
    );
}
fn hide(s: &JsValue) {
    set(s, "shown", &flag(false));
    stop_animation(s);
    if exists(&get(s, "finishTimer")) {
        call(&global(), "clearTimeout", &[get(s, "finishTimer")]);
    }
    set(s, "finishTimer", &JsValue::NULL);
    set(s, "finishCallback", &JsValue::NULL);
    let resolve = get(s, "finishResolve");
    set(s, "finishResolve", &JsValue::NULL);
    if exists(&resolve) {
        invoke(&resolve, &JsValue::UNDEFINED, &[]);
    }
    if yes(s, "dialog.open") {
        call(&get(s, "dialog"), "close", &[]);
    }
    release(s);
}
fn begin(s: &JsValue) {
    if exists(&get(s, "pending")) {
        return;
    }
    let a = s.clone();
    let p = future_to_promise(async move {
        let generation = number(&a, "generation") + 1.0;
        set(&a, "generation", &num(generation));
        if load(&a, generation).await.is_err() && generation == number(&a, "generation") {
            release(&a);
        }
        set(&a, "pending", &JsValue::NULL);
        Ok(JsValue::UNDEFINED)
    });
    set(s, "pending", &p);
}
async fn load(s: &JsValue, generation: f64) -> Result<(), JsValue> {
    let h = get(s, "host");
    let renderer = try_make(
        &h,
        "THREE.WebGLRenderer",
        &[options(&[
            ("canvas", get(s, "canvas")),
            ("antialias", flag(true)),
            ("alpha", flag(true)),
        ])],
    )?;
    set(s, "renderer", &renderer);
    call(
        &renderer,
        "setPixelRatio",
        &[num(number(&global(), "devicePixelRatio").min(2.0))],
    );
    set(
        &renderer,
        "toneMapping",
        &get(&h, "THREE.ACESFilmicToneMapping"),
    );
    set(&renderer, "toneMappingExposure", &num(0.95));
    set(&renderer, "shadowMap.enabled", &flag(true));
    set(&renderer, "shadowMap.type", &get(&h, "THREE.PCFShadowMap"));
    let scene = make(&h, "THREE.Scene", &[]);
    set(s, "scene", &scene);
    let pmrem = make(&h, "THREE.PMREMGenerator", &[renderer]);
    let room = make(&h, "RoomEnvironment", &[]);
    let environment = call(&pmrem, "fromScene", &[room.clone(), num(0.04)]);
    set(s, "environment", &environment);
    set(&scene, "environment", &get(&environment, "texture"));
    set(&scene, "environmentIntensity", &num(0.2));
    call(&room, "dispose", &[]);
    call(&pmrem, "dispose", &[]);
    set(
        s,
        "camera",
        &make(
            &h,
            "THREE.PerspectiveCamera",
            &[num(24.0), num(1.0), num(0.1), num(60.0)],
        ),
    );
    let key = make(
        &h,
        "THREE.DirectionalLight",
        &[num(0xffeedd as f64), num(3.6)],
    );
    call(
        &get(&key, "position"),
        "set",
        &[num(-4.2), num(4.2), num(2.6)],
    );
    set(&key, "castShadow", &flag(true));
    call(
        &get(&key, "shadow.mapSize"),
        "set",
        &[num(1024.0), num(1024.0)],
    );
    for (name, value) in [
        ("left", -2.2),
        ("bottom", -2.2),
        ("right", 2.2),
        ("top", 2.2),
    ] {
        set(&get(&key, "shadow.camera"), name, &num(value));
    }
    set(&key, "shadow.bias", &num(-0.0004));
    set(&key, "shadow.normalBias", &num(0.02));
    call(&scene, "add", &[key]);
    for (color, power, pos) in [
        (0xc8dcff, 4.2, [4.0, 2.2, -4.5]),
        (0xffffff, 0.35, [1.0, -4.0, 2.5]),
    ] {
        let light = make(
            &h,
            "THREE.DirectionalLight",
            &[num(color as f64), num(power)],
        );
        call(&get(&light, "position"), "fromArray", &[floats(&pos)]);
        call(&scene, "add", &[light]);
    }
    let response = wait(try_call(
        &global(),
        "fetch",
        &[text(&format!("{}brand/maw.json", string(&h, "base")))],
    )?)
    .await?;
    if !yes(&response, "ok") {
        return Err(error("Maw asset unavailable"));
    }
    let data = wait(try_call(&response, "json", &[])?).await?;
    let binary = try_call(&global(), "atob", &[get(&data, "data")])?
        .as_string()
        .unwrap_or_default();
    let bytes: Vec<u8> = binary.chars().map(|c| c as u8).collect();
    let gltf = wait(try_call(
        &make(&h, "GLTFLoader", &[]),
        "parseAsync",
        &[Uint8Array::from(bytes.as_slice()).buffer().into(), text("")],
    )?)
    .await?;
    let model = get(&gltf, "scene");
    if generation != number(s, "generation") || !yes(s, "shown") {
        dispose(&model, &[], true);
        return Ok(());
    }
    for n in nodes(&model, false) {
        if yes(&n, "isMesh") {
            set(&n, "castShadow", &flag(true));
            set(&n, "receiveShadow", &flag(true));
        }
    }
    let bounds = call(
        &make(&h, "THREE.Box3", &[]),
        "setFromObject",
        &[model.clone()],
    );
    call(
        &get(&model, "position"),
        "sub",
        &[call(
            &bounds,
            "getCenter",
            &[make(&h, "THREE.Vector3", &[])],
        )],
    );
    set(
        s,
        "radius",
        &num(number(
            &call(
                &bounds,
                "getBoundingSphere",
                &[make(&h, "THREE.Sphere", &[])],
            ),
            "radius",
        ) * 0.78),
    );
    let pivot = make(&h, "THREE.Group", &[]);
    call(&pivot, "add", &[model.clone()]);
    call(&scene, "add", &[pivot.clone()]);
    set(s, "pivot", &pivot);
    let clip = call(
        &get(&h, "THREE.AnimationClip"),
        "findByName",
        &[get(&gltf, "animations"), text("Idle")],
    );
    if exists(&clip) {
        let mixer = make(&h, "THREE.AnimationMixer", &[model]);
        let idle = call(&mixer, "clipAction", &[clip]);
        call(&idle, "play", &[]);
        set(s, "mixer", &mixer);
        set(s, "idle", &idle);
    }
    class(&get(s, "stage"), "add", "is-3d");
    resize(s);
    start_animation(s);
    Ok(())
}
fn resize(s: &JsValue) {
    let renderer = get(s, "renderer");
    let camera = get(s, "camera");
    if !exists(&renderer) || !exists(&camera) {
        return;
    }
    let w = number(s, "stage.clientWidth");
    let h = number(s, "stage.clientHeight");
    if w == 0.0 || h == 0.0 {
        return;
    }
    call(&renderer, "setSize", &[num(w), num(h), flag(false)]);
    set(&camera, "aspect", &num(w / h));
    let distance = ui(
        "ui.loaderDistance",
        &array(&[
            get(&camera, "fov"),
            get(&camera, "aspect"),
            get(s, "radius"),
        ]),
    );
    call(
        &get(&camera, "position"),
        "set",
        &[num(0.0), num(0.0), distance],
    );
    call(&camera, "lookAt", &[num(0.0), num(0.0), num(0.0)]);
    call(&camera, "updateProjectionMatrix", &[]);
    draw(s, 0.0);
}
fn stop_animation(s: &JsValue) {
    method_if(&get(s, "renderer"), "setAnimationLoop", &[JsValue::NULL]);
    set(s, "lastFrame", &num(0.0));
    set(s, "animationCallback", &JsValue::NULL);
}
fn start_animation(s: &JsValue) {
    if !yes(s, "shown")
        || yes(&document(), "hidden")
        || !exists(&get(s, "renderer"))
        || !exists(&get(s, "pivot"))
    {
        return;
    }
    draw(s, 0.0);
    if yes(s, "reduced") {
        return;
    }
    let a = s.clone();
    let callback = function(move |time| {
        let time = time.as_f64().unwrap_or(0.0);
        let last = number(&a, "lastFrame");
        let dt = if last != 0.0 {
            ((time - last) / 1000.0).min(0.05)
        } else {
            0.0
        };
        set(&a, "lastFrame", &num(time));
        draw(&a, dt);
    });
    set(s, "animationCallback", &callback);
    call(&get(s, "renderer"), "setAnimationLoop", &[callback]);
}
fn draw(s: &JsValue, dt: f64) {
    if ["renderer", "scene", "camera", "pivot"]
        .iter()
        .any(|name| !exists(&get(s, name)))
    {
        return;
    }
    set(s, "elapsed", &num(number(s, "elapsed") + dt));
    method_if(&get(s, "mixer"), "update", &[num(dt)]);
    let idle = get(s, "idle.time");
    let pose = ui(
        "ui.loaderPose",
        &array(&[
            get(s, "elapsed"),
            get(s, "reduced"),
            if exists(&idle) { idle } else { JsValue::NULL },
        ]),
    );
    let mut rotation = js_sys::Array::from(&get(&pose, "rotation"))
        .iter()
        .collect::<Vec<_>>();
    rotation.push(text("ZXY"));
    call(&get(s, "pivot.rotation"), "set", &rotation);
    call(&get(s, "pivot.scale"), "setScalar", &[get(&pose, "scale")]);
    call(
        &get(s, "renderer"),
        "render",
        &[get(s, "scene"), get(s, "camera")],
    );
}
fn release(s: &JsValue) {
    set(s, "generation", &num(number(s, "generation") + 1.0));
    stop_animation(s);
    class(&get(s, "stage"), "remove", "is-3d");
    method_if(&get(s, "mixer"), "stopAllAction", &[]);
    set(s, "mixer", &JsValue::NULL);
    set(s, "idle", &JsValue::NULL);
    let scene = get(s, "scene");
    if exists(&scene) {
        for n in nodes(&scene, false) {
            if exists(&get(&n, "shadow")) {
                method_if(&get(&n, "shadow"), "dispose", &[]);
            }
        }
        dispose(&scene, &[], true);
        call(&scene, "clear", &[]);
    }
    method_if(&get(s, "environment"), "dispose", &[]);
    method_if(&get(s, "renderer"), "dispose", &[]);
    for name in ["environment", "renderer", "scene", "pivot", "camera"] {
        set(s, name, &JsValue::NULL);
    }
}
