//! Item-preview controller. Rust owns requests, framing, input and resource lifetimes.
use super::view_utils::*;
use js_sys::{Array, Promise};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

#[wasm_bindgen]
pub struct RustItemViewer {
    state: JsValue,
}

#[wasm_bindgen]
impl RustItemViewer {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, element: JsValue, config: JsValue) -> Self {
        let s = options(&[
            ("host", host.clone()),
            ("element", element.clone()),
            ("config", config),
            ("models", object()),
            ("listeners", array(&[])),
            ("quality", get(&host, "QUALITY.medium")),
            ("radius", num(1.0)),
            ("serial", num(0.0)),
        ]);
        class(&element, "add", "ci-viewer");
        set(
            &element,
            "innerHTML",
            &text(
                "<canvas tabindex=\"0\" role=\"img\"></canvas><div class=\"ci-message\" role=\"status\"><span></span><button class=\"glass\" type=\"button\" hidden>Повторить</button></div><p class=\"ci-hint\">Потяни, чтобы повернуть</p>",
            ),
        );
        if yes(&s, "config.workbench") {
            set(
                &query(&element, ".ci-hint"),
                "textContent",
                &text("Влево-вправо — поворот · вверх-вниз — наклон"),
            );
        }
        let canvas = query(&element, "canvas");
        set(&s, "canvas", &canvas);
        set(&s, "status", &query(&element, ".ci-message span"));
        let retry = query(&element, "button");
        set(&s, "retry", &retry);
        let a = s.clone();
        listen(
            &s,
            &retry,
            "click",
            function(move |_| {
                let r = get(&a, "request");
                if exists(&r) {
                    let pending = show(
                        a.clone(),
                        string(&r, "id"),
                        string(&r, "name"),
                        get(&r, "loadout"),
                    );
                    wasm_bindgen_futures::spawn_local(async move {
                        let _ = wait(pending.into()).await;
                    });
                }
            }),
        );
        for event in [
            "pointerdown",
            "pointermove",
            "pointerup",
            "pointerleave",
            "pointercancel",
            "lostpointercapture",
            "keydown",
            "webglcontextlost",
            "webglcontextrestored",
        ] {
            let a = s.clone();
            listen(&s, &canvas, event, function(move |e| input(&a, event, &e)));
        }
        let a = s.clone();
        observe(&s, &element, function(move |_| resize(&a)));
        let scene = make(&host, "THREE.Scene", &[]);
        let pivot = make(&host, "THREE.Group", &[]);
        let offset = make(&host, "THREE.Group", &[]);
        call(&scene, "add", &[pivot.clone()]);
        call(&pivot, "add", &[offset.clone()]);
        set(&s, "scene", &scene);
        set(&s, "pivot", &pivot);
        set(&s, "offset", &offset);
        set(
            &s,
            "camera",
            &make(
                &host,
                "THREE.PerspectiveCamera",
                &[num(33.0), num(1.0), num(0.01), num(50.0)],
            ),
        );
        call(
            &scene,
            "add",
            &[make(
                &host,
                "THREE.HemisphereLight",
                &[num(0xdce2e8 as f64), num(0x323335 as f64), num(1.2)],
            )],
        );
        for [x, y, z, power] in [
            [-2.0, 3.0, 3.0, 3.6],
            [3.0, 1.0, 2.0, 1.4],
            [1.0, 2.0, -3.0, 2.8],
        ] {
            let light = make(
                &host,
                "THREE.DirectionalLight",
                &[num(0xe9eef2 as f64), num(power)],
            );
            call(&get(&light, "position"), "set", &[num(x), num(y), num(z)]);
            call(&scene, "add", &[light]);
        }
        Self { state: s }
    }
    pub fn show(&self, id: String, name: String, loadout: JsValue) -> Promise {
        show(self.state.clone(), id, name, loadout)
    }
    #[wasm_bindgen(js_name=setActive)]
    pub fn set_active(&self, active: bool) {
        set(&self.state, "active", &flag(active));
        if active {
            resize(&self.state);
        } else {
            set(&self.state, "drag", &JsValue::NULL);
        }
    }
    /// Turns the model back to the pose it opened with.
    #[wasm_bindgen(js_name=resetView)]
    pub fn reset_view(&self) {
        let s = &self.state;
        if exists(&get(s, "home")) {
            call(&get(s, "pivot.quaternion"), "copy", &[get(s, "home")]);
            render(s);
        }
    }
    pub fn hide(&self) {
        let s = &self.state;
        set(s, "serial", &num(number(s, "serial") + 1.0));
        set(s, "active", &flag(false));
        set(&get(s, "element"), "hidden", &flag(true));
        set(s, "drag", &JsValue::NULL);
    }
    pub fn dispose(&self) {
        let s = &self.state;
        if yes(s, "disposed") {
            return;
        }
        set(s, "disposed", &flag(true));
        set(s, "serial", &num(number(s, "serial") + 1.0));
        method_if(&get(s, "observer"), "disconnect", &[]);
        unlisten(s);
        for pending in js_sys::Object::values(&get(s, "models").into()).iter() {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(entry) = wait(pending).await {
                    release(&entry);
                }
            });
        }
        set(s, "models", &object());
        method_if(&get(s, "environment"), "dispose", &[]);
        method_if(&get(s, "renderer"), "dispose", &[]);
        call(&get(s, "element"), "replaceChildren", &[]);
    }
}

fn init(s: &JsValue) -> Result<(), JsValue> {
    if exists(&get(s, "renderer")) {
        return Ok(());
    }
    let h = get(s, "host");
    let renderer = try_make(
        &h,
        "THREE.WebGLRenderer",
        &[options(&[
            ("canvas", get(s, "canvas")),
            ("alpha", flag(true)),
            ("antialias", flag(true)),
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
    set(&renderer, "toneMappingExposure", &num(0.95));
    let quality = call(&h, "startQuality", &[call(&renderer, "getContext", &[])]);
    let quality = get(&get(&h, "QUALITY"), &string(&quality, "id"));
    set(s, "quality", &quality);
    let pmrem = make(&h, "THREE.PMREMGenerator", &[renderer]);
    let room = make(&h, "RoomEnvironment", &[]);
    let env = call(&pmrem, "fromScene", &[room.clone(), num(0.06)]);
    call(&room, "dispose", &[]);
    call(&pmrem, "dispose", &[]);
    set(s, "environment", &env);
    set(
        &get(s, "scene"),
        "environment",
        &if yes(&quality, "env") {
            get(&env, "texture")
        } else {
            JsValue::NULL
        },
    );
    Ok(())
}
fn release(entry: &JsValue) {
    dispose(
        &get(entry, "root"),
        &Array::from(&get(entry, "textures"))
            .iter()
            .collect::<Vec<_>>(),
        true,
    );
}
fn model(s: &JsValue, url: String) -> Promise {
    let cache = get(s, "models");
    let cached = js_sys::Reflect::get(&cache, &text(&url)).unwrap_or(JsValue::UNDEFINED);
    if exists(&cached) {
        return cached.into();
    }
    let state = s.clone();
    let key = url.clone();
    let p = future_to_promise(async move {
        let h = get(&state, "host");
        let result = async {
            let root = get(
                &wait(try_call(&h, "loadModel", &[text(&url)])?).await?,
                "scene",
            );
            let entry = options(&[
                ("root", root.clone()),
                ("textures", array(&textures(&root))),
            ]);
            if !yes(&state, "disposed") {
                call(&h, "prepareVaryag", &[root.clone()]);
                let tiers = make(&h, "TextureTiers", &[array(&[root.clone()]), text(&url)]);
                let anisotropy = number(&state, "quality.anisotropy").min(
                    call(
                        &get(&state, "renderer.capabilities"),
                        "getMaxAnisotropy",
                        &[],
                    )
                    .as_f64()
                    .unwrap_or(1.0),
                );
                if let Err(e) = wait(try_call(
                    &tiers,
                    "set",
                    &[get(&state, "quality.textures"), num(anisotropy)],
                )?)
                .await
                {
                    release(&entry);
                    return Err(e);
                }
                let mut all = Array::from(&get(&entry, "textures"))
                    .iter()
                    .collect::<Vec<_>>();
                for t in textures(&root) {
                    unique(&mut all, t);
                }
                set(&entry, "textures", &array(&all));
                for m in materials(&root) {
                    call(&h, "tune", &[m, get(&state, "quality")]);
                }
            }
            Ok(entry)
        }
        .await;
        if result.is_err() {
            js_sys::Reflect::delete_property(&get(&state, "models").into(), &text(&url))?;
        }
        result
    });
    js_sys::Reflect::set(&cache, &text(&key), &p).unwrap();
    p
}
fn show(s: JsValue, id: String, name: String, loadout: JsValue) -> Promise {
    future_to_promise(async move {
        let spec = get(&get(&s, "host.ITEM_MODELS"), &id);
        if yes(&s, "disposed") || !exists(&spec) {
            return Ok(JsValue::UNDEFINED);
        }
        let keep_angle = string(&s, "request.id") == id;
        set(
            &s,
            "request",
            &options(&[
                ("id", text(&id)),
                ("name", text(&name)),
                ("loadout", loadout.clone()),
            ]),
        );
        let serial = number(&s, "serial") + 1.0;
        set(&s, "serial", &num(serial));
        set(&s, "active", &flag(true));
        set(&get(&s, "element"), "hidden", &flag(false));
        set(&get(&s, "canvas"), "hidden", &flag(true));
        set(&get(&s, "retry"), "hidden", &flag(true));
        set(&get(&s, "status"), "textContent", &text("Загрузка…"));
        let hint = if yes(&s, "config.workbench") {
            " Влево-вправо — поворот, вверх-вниз — наклон."
        } else {
            ""
        };
        attr(
            &get(&s, "canvas"),
            "aria-label",
            &format!("{name}. Объёмная модель, вращение перетаскиванием или стрелками.{hint}"),
        );
        set(&get(&s, "element"), "dataset.item", &text(&id));
        let spec = spec.as_string().unwrap_or_default();
        let (file, only) = spec.split_once('#').unwrap_or((&spec, ""));
        let (file, only) = (file.to_string(), only.to_string());
        let result = async {
            init(&s)?;
            let entry = wait(
                model(
                    &s,
                    format!("{}preview/{file}.json", string(&s, "host.base")),
                )
                .into(),
            )
            .await?;
            if yes(&s, "disposed") || number(&s, "serial") != serial {
                return Ok(());
            }
            let h = get(&s, "host");
            let root = get(&entry, "root");
            if !only.is_empty() {
                // A garment cut out of the dressed Persona: the body and other gear stay hidden.
                for n in nodes(&root, false) {
                    if yes(&n, "isMesh") {
                        let keep = string(&n, "name").starts_with(&only)
                            || string(&n, "parent.name").starts_with(&only);
                        set(&n, "visible", &flag(keep));
                    }
                }
            }
            let offset = get(&s, "offset");
            let pivot = get(&s, "pivot");
            call(&offset, "clear", &[]);
            call(
                &get(&offset, "position"),
                "set",
                &[num(0.0), num(0.0), num(0.0)],
            );
            call(&get(&offset, "scale"), "setScalar", &[num(1.0)]);
            let angle = call(&get(&pivot, "rotation"), "clone", &[]);
            call(
                &get(&pivot, "rotation"),
                "set",
                &[num(0.0), num(0.0), num(0.0)],
            );
            call(&offset, "add", &[root.clone()]);
            set(&s, "displayed", &root);
            let rifle = exists(&call(&root, "getObjectByName", &[text("socket_muzzle")]));
            if rifle {
                // Mods show only on the gun they were fitted to.
                let fitted = exists(&loadout) && string(&loadout, "weapon1.item") == id;
                call(
                    &h,
                    "fitVaryag",
                    &[root.clone(), if fitted { loadout } else { JsValue::NULL }],
                );
            }
            call(&get(&s, "scene"), "updateMatrixWorld", &[flag(true)]);
            let box3 = bounds(&h, &root);
            let centre = call(&box3, "getCenter", &[make(&h, "THREE.Vector3", &[])]);
            let size = call(&box3, "getSize", &[make(&h, "THREE.Vector3", &[])]);
            let scale = 1.8
                / number(&size, "x")
                    .max(number(&size, "y"))
                    .max(number(&size, "z"))
                    .max(0.001);
            call(&get(&offset, "scale"), "setScalar", &[num(scale)]);
            call(
                &get(&offset, "position"),
                "copy",
                &[call(&centre, "multiplyScalar", &[num(-scale)])],
            );
            set(
                &s,
                "radius",
                &num(call(&size, "length", &[]).as_f64().unwrap_or(0.0) * scale / 2.0),
            );
            if keep_angle {
                call(&get(&pivot, "rotation"), "copy", &[angle]);
            } else {
                call(
                    &get(&pivot, "rotation"),
                    "set",
                    &[
                        num(0.06),
                        num(if rifle { -0.22 } else { 0.35 }),
                        num(if rifle {
                            if yes(&s, "config.workbench") {
                                0.68
                            } else {
                                -0.3
                            }
                        } else {
                            0.0
                        }),
                    ],
                );
                // The opening pose, for "reset view"; fitted mods do not change it.
                set(&s, "home", &call(&get(&pivot, "quaternion"), "clone", &[]));
            }
            set(&get(&s, "status"), "textContent", &text(""));
            set(&get(&s, "canvas"), "hidden", &flag(false));
            set(&get(&s, "element"), "dataset.state", &text("ready"));
            resize(&s);
            Ok::<(), JsValue>(())
        }
        .await;
        if result.is_err() && !yes(&s, "disposed") && number(&s, "serial") == serial {
            set(&get(&s, "element"), "dataset.state", &text("error"));
            set(
                &get(&s, "status"),
                "textContent",
                &text("Модель не загрузилась"),
            );
            set(&get(&s, "retry"), "hidden", &flag(false));
        }
        Ok(JsValue::UNDEFINED)
    })
}
fn resize(s: &JsValue) {
    let renderer = get(s, "renderer");
    if !exists(&renderer) || !yes(s, "active") || yes(s, "disposed") {
        return;
    }
    let w = number(s, "element.clientWidth");
    let h = number(s, "element.clientHeight");
    if w == 0.0 || h == 0.0 {
        return;
    }
    let dpr = (number(&global(), "devicePixelRatio") * number(s, "quality.ssaa"))
        .min(number(s, "quality.maxDpr"))
        .min((number(s, "quality.maxPixels") / (w * h)).sqrt());
    call(&renderer, "setPixelRatio", &[num(dpr)]);
    call(&renderer, "setSize", &[num(w), num(h), flag(false)]);
    let camera = get(s, "camera");
    set(&camera, "aspect", &num(w / h));
    call(&camera, "updateProjectionMatrix", &[]);
    let half = number(&camera, "fov").to_radians() / 2.0;
    let mut distance = number(s, "radius") / half.min((half.tan() * w / h).atan()).sin() * 1.10;
    let root = get(s, "displayed");
    if exists(&root) {
        call(&get(s, "scene"), "updateMatrixWorld", &[flag(true)]);
        let b = bounds(&get(s, "host"), &root);
        let mut corners = vec![
            half.tan(),
            half.tan() * w / h,
            if yes(s, "config.workbench") {
                1.28
            } else {
                1.16
            },
        ];
        for x in [number(&b, "min.x"), number(&b, "max.x")] {
            for y in [number(&b, "min.y"), number(&b, "max.y")] {
                for z in [number(&b, "min.z"), number(&b, "max.z")] {
                    corners.extend([x, y, z]);
                }
            }
        }
        distance = game_motion::compute("cameraDistance", &corners).unwrap()[0];
    }
    call(
        &get(&camera, "position"),
        "set",
        &[num(0.0), num(0.0), num(distance)],
    );
    call(&camera, "lookAt", &[num(0.0), num(0.0), num(0.0)]);
    render(s);
}
fn render(s: &JsValue) {
    if !yes(s, "active") || yes(s, "disposed") {
        return;
    }
    method_if(
        &get(s, "renderer"),
        "render",
        &[get(s, "scene"), get(s, "camera")],
    );
    let callback = get(s, "config.onSockets");
    let root = get(s, "displayed");
    if exists(&callback) && exists(&root) {
        let points = object();
        for slot in Array::from(&get(s, "host.MOD_SLOTS")).iter() {
            let name = slot.as_string().unwrap();
            let socket = call(&root, "getObjectByName", &[text(&format!("socket_{name}"))]);
            if !exists(&socket) {
                continue;
            }
            let p = call(
                &socket,
                "getWorldPosition",
                &[make(&get(s, "host"), "THREE.Vector3", &[])],
            );
            call(&p, "project", &[get(s, "camera")]);
            if number(&p, "z") < 1.0 {
                set(
                    &points,
                    &name,
                    &floats(&[
                        (number(&p, "x") + 1.0) * number(s, "element.clientWidth") / 2.0,
                        (1.0 - number(&p, "y")) * number(s, "element.clientHeight") / 2.0,
                    ]),
                );
            }
        }
        invoke(&callback, &JsValue::UNDEFINED, &[points]);
    }
}
fn turn(s: &JsValue, h: f64, v: f64) {
    let pivot = get(s, "pivot");
    if yes(s, "config.workbench") {
        let mut input = components(&call(&get(&pivot, "quaternion"), "toArray", &[]));
        input.extend([h, v]);
        call(
            &get(&pivot, "quaternion"),
            "fromArray",
            &[floats(&game_motion::compute("viewerTurn", &input).unwrap())],
        );
    } else {
        set(&pivot, "rotation.y", &num(number(&pivot, "rotation.y") + h));
        set(
            &pivot,
            "rotation.x",
            &num((number(&pivot, "rotation.x") + v).clamp(-1.1, 1.1)),
        );
    }
}
/// The mod slot of the visible gun part under the pointer, or null.
fn part(s: &JsValue, e: &JsValue) -> JsValue {
    let root = get(s, "displayed");
    let rect = call(&get(s, "canvas"), "getBoundingClientRect", &[]);
    let (w, h) = (number(&rect, "width"), number(&rect, "height"));
    if !exists(&root) || w == 0.0 || h == 0.0 {
        return JsValue::NULL;
    }
    let host = get(s, "host");
    if !exists(&get(s, "ray")) {
        set(s, "ray", &make(&host, "THREE.Raycaster", &[]));
    }
    let ray = get(s, "ray");
    let ndc = make(
        &host,
        "THREE.Vector2",
        &[
            num((number(e, "clientX") - number(&rect, "left")) / w * 2.0 - 1.0),
            num(1.0 - (number(e, "clientY") - number(&rect, "top")) / h * 2.0),
        ],
    );
    call(&ray, "setFromCamera", &[ndc, get(s, "camera")]);
    // Three.js also hits hidden meshes, so the first hit whose whole chain is visible wins.
    'hits: for hit in Array::from(&call(&ray, "intersectObject", &[root.clone(), flag(true)])).iter() {
        let mut node = get(&hit, "object");
        let mut slot = JsValue::NULL;
        while exists(&node) {
            if !yes(&node, "visible") {
                continue 'hits;
            }
            if slot.is_null() && get(&node, "userData.slot").is_string() {
                slot = get(&node, "userData.slot");
            }
            if node == root {
                break;
            }
            node = get(&node, "parent");
        }
        return slot;
    }
    JsValue::NULL
}
fn point(s: &JsValue, slot: JsValue, e: &JsValue) {
    let callback = get(s, "config.onPart");
    if exists(&callback) {
        invoke(
            &callback,
            &JsValue::UNDEFINED,
            &[slot, get(e, "clientX"), get(e, "clientY")],
        );
    }
}
fn input(s: &JsValue, event: &str, e: &JsValue) {
    let canvas = get(s, "canvas");
    match event {
        "pointerdown" => {
            if number(e, "button") != 0.0 || !yes(e, "isPrimary") {
                return;
            }
            set(
                s,
                "drag",
                &options(&[
                    ("id", get(e, "pointerId")),
                    ("x", get(e, "clientX")),
                    ("y", get(e, "clientY")),
                ]),
            );
            call(&canvas, "setPointerCapture", &[get(e, "pointerId")]);
            point(s, JsValue::NULL, e);
        }
        "pointerleave" => point(s, JsValue::NULL, e),
        "pointermove" => {
            if !exists(&get(s, "drag")) {
                if string(e, "pointerType") == "mouse" {
                    point(s, part(s, e), e);
                }
                return;
            }
            if get(s, "drag.id") != get(e, "pointerId") {
                return;
            }
            turn(
                s,
                (number(e, "clientX") - number(s, "drag.x")) * 0.012,
                (number(e, "clientY") - number(s, "drag.y")) * 0.008,
            );
            set(s, "drag.x", &get(e, "clientX"));
            set(s, "drag.y", &get(e, "clientY"));
            render(s);
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
        "keydown" => {
            let (h, v) = match string(e, "key").as_str() {
                "ArrowLeft" => (-0.2, 0.0),
                "ArrowRight" => (0.2, 0.0),
                "ArrowUp" => (0.0, -0.15),
                "ArrowDown" => (0.0, 0.15),
                _ => return,
            };
            call(e, "preventDefault", &[]);
            turn(s, h, v);
            render(s);
        }
        "webglcontextlost" => {
            call(e, "preventDefault", &[]);
            set(
                &get(s, "status"),
                "textContent",
                &text("Восстановление изображения…"),
            );
        }
        "webglcontextrestored" => {
            set(&get(s, "status"), "textContent", &text(""));
            resize(s);
        }
        _ => {}
    }
}
