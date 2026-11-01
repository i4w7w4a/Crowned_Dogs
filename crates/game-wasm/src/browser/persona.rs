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
            ("look", super::platform::current_look()),
        ]);
        set(&canvas, "style.touchAction", &text("pan-y"));
        for event in [
            "pointerdown",
            "pointermove",
            "pointerup",
            "pointercancel",
            "lostpointercapture",
            "pointerleave",
            "contextmenu",
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
        "3D: комплект A и стволы · остальные предметы пока показаны условно".into()
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
                        "Не удалось загрузить 3D-модель. Проверьте соединение и откройте «Снаряжение» ещё раз.",
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
            creation_loop(s);
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
    /// Glows the equipment worn in `slot`; an empty slot clears the glow.
    pub fn highlight(&self, slot: &str) {
        set(&self.state, "hover", &text(slot));
        highlight(&self.state, slot);
    }
    /// Shows the player's look; a different sex swaps the figure on the stand.
    #[wasm_bindgen(js_name=setLook)]
    pub fn set_look(&self, look: JsValue) {
        let s = &self.state;
        set(s, "look", &look);
        set(s, "head", &JsValue::NULL);
        let figure = get(s, "figure");
        if !exists(&figure) {
            return;
        }
        if string(s, "figureSex") == string(s, "look.sex") {
            super::look::apply(figure, look);
            render(s);
            return;
        }
        if yes(s, "figureLoading") {
            // The load in flight rechecks the sex when it lands.
            return;
        }
        let a = s.clone();
        spawn_local(async move {
            if load_figure(&a).await.is_ok() && !yes(&a, "disposed") {
                apply_weapon(&a);
                tune(&a, &get(&a, "scene"));
                render(&a);
            }
        });
    }
    /// Bare mode hides helmet, vest and gun, for choosing the look.
    #[wasm_bindgen(js_name=setBare)]
    pub fn set_bare(&self, bare: bool) {
        set(&self.state, "bare", &flag(bare));
        dress_figure(&self.state);
        set(&self.state, "head", &JsValue::NULL);
        render(&self.state);
    }
    /// Creation mode: the stand and the bay hide, the Persona stands on the backdrop's floor
    /// and the camera frames the body or the face (see `setFocus`).
    #[wasm_bindgen(js_name=setCreating)]
    pub fn set_creating(&self, on: bool) {
        let s = &self.state;
        if yes(s, "creating") == on {
            return;
        }
        set(s, "creating", &flag(on));
        pose(s);
        if on && !exists(&get(s, "bay")) {
            super::bay::build(s, anisotropy(s));
        }
        show_stand(s);
        super::persona_hd::update(s);
        let _ = update_textures(s);
        if !on {
            set(s, "focusT", &num(0.0));
            set(s, "focusGoal", &num(0.0));
            call(&get(s, "camera"), "updateProjectionMatrix", &[]);
            set(&get(s, "canvas"), "style.cursor", &text(""));
        }
        aim(s);
        render(s);
        creation_loop(s);
    }
    /// "face" glides the camera to a head close-up, anything else back to the whole body.
    #[wasm_bindgen(js_name=setFocus)]
    pub fn set_focus(&self, focus: &str) {
        let s = &self.state;
        set(
            s,
            "focusGoal",
            &num(if focus == "face" { 1.0 } else { 0.0 }),
        );
        if !yes(s, "creating") || !yes(s, "ready") || !yes(s, "active") {
            // Not on screen: jump straight there.
            set(s, "focusT", &get(s, "focusGoal"));
            aim(s);
            return;
        }
        if !exists(&get(s, "focusFrame")) {
            set(s, "focusLast", &num(now()));
            focus_frame(s);
        }
    }
    #[wasm_bindgen(js_name=updateLoadout)]
    pub fn update_loadout(&self, loadout: JsValue) -> JsValue {
        let s = &self.state;
        set(s, "loadout", &loadout);
        dress_figure(s);
        apply_weapon(s);
        render(s);
        graphics(
            "graphics.fittingCoverage",
            &options(&[
                ("loadout", loadout),
                ("weapon", text(&weapon(s))),
                ("gunModel", flag(exists(&shown_gun(s)))),
                ("modMeshes", mod_meshes(&shown_gun(s))),
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
        call(&global(), "cancelAnimationFrame", &[get(s, "focusFrame")]);
        set(s, "focusFrame", &JsValue::NULL);
        call(&global(), "cancelAnimationFrame", &[get(s, "loopFrame")]);
        set(s, "loopFrame", &JsValue::NULL);
        set(s, "looping", &flag(false));
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
    set(s, "guns", &Object::new());
    let gun = load_gun(s, weapon(s));
    let a = s.clone();
    let room = future_to_promise(async move {
        load_environment(&a).await;
        Ok(JsValue::UNDEFINED)
    });
    // A stand-in right hand: the authored pose has no rig, so the held gun hangs off this.
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
    load_figure(s).await?;
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
    creation_loop(s);
    Ok(())
}
/// Loads the Persona of the look's sex onto the stand, replacing the one already there.
async fn load_figure(s: &JsValue) -> Result<(), JsValue> {
    // One load at a time: quick taps on the sex switch must not race two figures in.
    set(s, "figureLoading", &flag(true));
    let done = load_latest(s).await;
    set(s, "figureLoading", &flag(false));
    done
}
async fn load_latest(s: &JsValue) -> Result<(), JsValue> {
    loop {
        let sex = string(s, "look.sex");
        if exists(&get(s, "figure")) && string(s, "figureSex") == sex {
            super::look::apply(get(s, "figure"), get(s, "look"));
            return Ok(());
        }
        if install_figure(s, &sex).await? || yes(s, "disposed") {
            return Ok(());
        }
    }
}
/// Loads and installs the `sex` figure; false when the look changed sex meanwhile.
async fn install_figure(s: &JsValue, sex: &str) -> Result<bool, JsValue> {
    let h = get(s, "host");
    let sex = sex.to_string();
    let female = sex == "female";
    let url = format!("{}models/persona-{sex}.json", string(&h, "base"));
    let gltf = wait(try_call(&h, "loadModel", &[text(&url)])?).await?;
    if !exists(&get(s, "poses")) {
        let url = format!("{}models/persona-poses.json", string(&h, "base"));
        if let Ok(poses) = wait(try_call(&h, "loadModel", &[text(&url)])?).await {
            set(s, "poses", &get(&poses, "animations"));
        }
    }
    let figure = get(&gltf, "scene");
    if yes(s, "disposed") || string(s, "look.sex") != sex {
        dispose(&figure, &[], true);
        return Ok(yes(s, "disposed"));
    }
    let old = get(s, "figure");
    if exists(&old) {
        call(&old, "removeFromParent", &[]);
        super::persona_hd::restore(&old);
        let extra = Array::from(&get(s, "originalTextures"))
            .iter()
            .collect::<Vec<_>>();
        let pending = get(s, "texturesPending");
        spawn_local(async move {
            let _ = wait(pending).await;
            dispose(&old, &extra, true);
        });
    }
    set(s, "figure", &figure);
    set(s, "figureSex", &text(&sex));
    set(s, "mixer", &JsValue::NULL);
    set(s, "head", &JsValue::NULL);
    // Both figures are the rigged game models: sized and centred at the first idle frame,
    // then posed (see `pose`) with the feet on the platform.
    let mixer = make(&h, "THREE.AnimationMixer", &[figure.clone()]);
    set(s, "mixer", &mixer);
    set(s, "figureClips", &get(&gltf, "animations"));
    hold(&mixer, &clip(&h, &get(&gltf, "animations"), "idle"));
    let box3 = lowest_box(&h, &figure);
    let height = if female { FEMALE_HEIGHT } else { MALE_HEIGHT };
    let k = height / (number(&box3, "max.y") - number(&box3, "min.y")).max(0.1);
    call(&get(&figure, "scale"), "setScalar", &[num(k)]);
    let centre = call(&box3, "getCenter", &[make(&h, "THREE.Vector3", &[])]);
    call(
        &get(&figure, "position"),
        "set",
        &[
            num(-number(&centre, "x") * k),
            num(0.0),
            num(-number(&centre, "z") * k),
        ],
    );
    // Where the feet are in each pose, measured while the figure stands alone.
    let lows = Object::new();
    for name in [
        format!("{sex}_stand"),
        format!("{sex}_idle"),
        "idle".to_string(),
    ] {
        let c = pose_clip(s, &name);
        if exists(&c) {
            hold(&mixer, &c);
            set(
                &lows,
                &name,
                &num(number(&lowest_box(&h, &figure), "min.y")),
            );
        }
    }
    set(s, "poseLows", &lows);
    set(&figure, "name", &text("Persona_Authored_In_Blender"));
    for n in nodes(&figure, false) {
        if yes(&n, "isMesh") {
            // Blender object names; the loader may swap spaces for underscores.
            if let Some(slot) = super::scene_assets::gear_slot(&string(&n, "name")) {
                set(&n, "userData.slot", &text(slot));
            }
            set(&n, "frustumCulled", &flag(false));
            set(&n, "castShadow", &flag(true));
            set(&n, "receiveShadow", &flag(true));
        }
    }
    call(&get(s, "stage.rotating"), "add", &[figure.clone()]);
    pose(s);
    set(s, "originalTextures", &array(&textures(&figure)));
    set(
        s,
        "tiers",
        &make(&h, "TextureTiers", &[array(&[figure.clone()]), text(&url)]),
    );
    dress_figure(s);
    super::look::apply(figure, get(s, "look"));
    super::persona_hd::update(s);
    wait(update_textures(s).into()).await?;
    Ok(true)
}
/// Heights on the stand, with the helmet; the male as the authored Blender scene he replaced.
const FEMALE_HEIGHT: f64 = 1.72;
const MALE_HEIGHT: f64 = 1.8;
/// Art's poses (models/persona-poses, retargeted from his Mixamo files, 2026-11-01). The male
/// idles in "Neutral Idle" while his look is made and stands in the first frame of "Standing
/// Idle" in the fitting room, where the kit is hung on that frame; the female keeps her one pose.
fn pose_name(s: &JsValue) -> String {
    let sex = string(s, "figureSex");
    if sex == "male" && yes(s, "creating") {
        "male_idle".into()
    } else {
        format!("{sex}_stand")
    }
}
/// Puts the figure in the pose for the screen, feet on the platform; the game's first idle
/// frame when the poses did not load.
fn pose(s: &JsValue) {
    let figure = get(s, "figure");
    let mixer = get(s, "mixer");
    if !exists(&figure) || !exists(&mixer) {
        return;
    }
    let h = get(s, "host");
    let mut name = pose_name(s);
    if !exists(&pose_clip(s, &name)) {
        name = "idle".into();
    }
    hold(&mixer, &pose_clip(s, &name));
    if let Some(low) = get(&get(s, "poseLows"), &name).as_f64() {
        set(
            &get(&figure, "position"),
            "y",
            &num(number(&h, "PLATFORM_TOP") - low),
        );
    }
    set(s, "head", &JsValue::NULL);
}
fn pose_clip(s: &JsValue, name: &str) -> JsValue {
    let h = get(s, "host");
    let from = if name == "idle" {
        get(s, "figureClips")
    } else {
        get(s, "poses")
    };
    if exists(&from) {
        clip(&h, &from, name)
    } else {
        JsValue::NULL
    }
}
fn clip(h: &JsValue, clips: &JsValue, name: &str) -> JsValue {
    call(
        &get(h, "THREE.AnimationClip"),
        "findByName",
        &[clips.clone(), text(name)],
    )
}
/// Plays only `clip`, from its start.
fn hold(mixer: &JsValue, clip: &JsValue) {
    if !exists(clip) {
        return;
    }
    call(mixer, "stopAllAction", &[]);
    call(&call(mixer, "clipAction", &[clip.clone()]), "play", &[]);
    call(mixer, "setTime", &[num(0.0)]);
}
/// The box of the plain kit as the figure stands (skinned meshes follow their bones).
fn lowest_box(h: &JsValue, figure: &JsValue) -> JsValue {
    call(figure, "updateMatrixWorld", &[flag(true)]);
    let box3 = make(h, "THREE.Box3", &[]);
    for n in nodes(figure, false) {
        if yes(&n, "isMesh") && super::scene_assets::plain_kit(&string(&n, "name")) {
            call(&box3, "expandByObject", &[n, flag(true)]);
        }
    }
    box3
}
/// Shows the worn kit, or only suit and boots while the look is being chosen.
fn dress_figure(s: &JsValue) {
    let figure = get(s, "figure");
    if !exists(&figure) {
        return;
    }
    let bare = yes(s, "bare");
    let kit = get(s, "loadout");
    let wear = |slot: &str| {
        if bare && (slot == "head" || slot == "body") {
            JsValue::NULL
        } else if exists(&kit) {
            get(&kit, slot)
        } else {
            flag(true)
        }
    };
    super::scene_assets::dress(
        get(s, "host"),
        figure.clone(),
        options(&["head", "body", "suit", "boots"].map(|k| (k, wear(k)))),
    );
    set(&get(s, "equipment"), "visible", &flag(!bare));
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
        if yes(&m, "isShaderMaterial") {
            continue;
        }
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
    // A look is made up close: there the Persona wears 2K on any device that takes 1K.
    let mut size = get(s, "quality.textures");
    if yes(s, "creating") && number(s, "quality.textures") >= 1024.0 {
        size = num(number(s, "quality.textures").max(2048.0));
    }
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
    if yes(s, "creating") {
        creation_view(s);
        return;
    }
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
    let v = |key: &str| {
        let c = components(&get(&fit, key));
        [c[0], c[1], c[2]]
    };
    call(
        &get(s, "camera.position"),
        "fromArray",
        &[floats(&v("position"))],
    );
    call(&get(s, "camera"), "lookAt", &v("target").map(num));
    call(&get(s, "camera"), "updateMatrixWorld", &[]);
}
fn now() -> f64 {
    js_sys::Date::now()
}
/// One step of the glide between the body and the face.
fn focus_frame(s: &JsValue) {
    let a = s.clone();
    let id = call(
        &global(),
        "requestAnimationFrame",
        &[function(move |_| {
            set(&a, "focusFrame", &JsValue::NULL);
            if yes(&a, "disposed") {
                return;
            }
            let time = now();
            let dt = (time - number(&a, "focusLast")).clamp(0.0, 50.0);
            set(&a, "focusLast", &num(time));
            let (t, goal) = (number(&a, "focusT"), number(&a, "focusGoal"));
            let step = dt / FOCUS_MS;
            let next = if goal > t {
                (t + step).min(goal)
            } else {
                (t - step).max(goal)
            };
            set(&a, "focusT", &num(next));
            aim(&a);
            render(&a);
            if next != goal && yes(&a, "active") {
                focus_frame(&a);
            } else {
                set(&a, "focusT", &num(goal));
            }
        })],
    );
    set(s, "focusFrame", &id);
}
/// How long the camera takes between the body and the face.
const FOCUS_MS: f64 = 520.0;
/// The creation camera. The whole Persona is seen from exactly where the bay's picture was
/// taken, so the picture shows as it was drawn; the face close-up really moves the camera into
/// the bay. The frame's centre sits over the sheet, which covers the bottom of the canvas.
fn creation_view(s: &JsValue) {
    let h = get(s, "host");
    let camera = get(s, "camera");
    let (width, height) = (number(s, "width").max(1.0), number(s, "height").max(1.0));
    let covered = sheet_inset(s).clamp(0.0, height * 0.7);
    let visible = height - covered;
    let tangent = (FACE_FOV / 2.0).to_radians().tan();
    let head = head(s);
    let d = 0.28_f64.max(0.19 / (width / visible)) / tangent;
    let face = [head[0], head[1] + 0.015, head[2] + d];
    let look = make(&h, "THREE.Matrix4", &[]);
    call(
        &look,
        "lookAt",
        &[
            make(&h, "THREE.Vector3", &face.map(num)),
            make(
                &h,
                "THREE.Vector3",
                &[num(head[0]), num(head[1] - 0.035), num(head[2])],
            ),
            make(&h, "THREE.Vector3", &[num(0.0), num(1.0), num(0.0)]),
        ],
    );
    let turn = call(
        &make(&h, "THREE.Quaternion", &[]),
        "setFromRotationMatrix",
        &[look],
    );
    let t = number(s, "focusT").clamp(0.0, 1.0);
    let k = t * t * (3.0 - 2.0 * t);
    super::bay::close_in(s, k);
    let body = super::bay::EYE;
    let at = [0, 1, 2].map(|i| body[i] + (face[i] - body[i]) * k);
    call(&get(&camera, "position"), "fromArray", &[floats(&at)]);
    call(
        &get(&camera, "quaternion"),
        "slerpQuaternions",
        &[super::bay::eye_rotation(&h), turn, num(k)],
    );
    call(&camera, "updateMatrixWorld", &[]);
    // The visible part of the frame spans these picture-plane tangents; the canvas goes on below.
    let top = super::bay::BODY_TOP + (tangent - super::bay::BODY_TOP) * k;
    let bottom = super::bay::BODY_BOTTOM + (-tangent - super::bay::BODY_BOTTOM) * k;
    let per_pixel = (top - bottom) / visible;
    let near = number(&camera, "near");
    let half = width / 2.0 * per_pixel * near;
    let matrix = get(&camera, "projectionMatrix");
    call(
        &matrix,
        "makePerspective",
        &[
            num(-half),
            num(half),
            num(top * near),
            num((bottom - covered * per_pixel) * near),
            num(near),
            get(&camera, "far"),
        ],
    );
    call(
        &call(&get(&camera, "projectionMatrixInverse"), "copy", &[matrix]),
        "invert",
        &[],
    );
}
/// The close-up's vertical field of view over the visible part of the canvas.
const FACE_FOV: f64 = 33.0;
/// How much of the canvas the creation sheet covers, in CSS pixels.
fn sheet_inset(s: &JsValue) -> f64 {
    let canvas = get(s, "canvas");
    let fitting = call(&canvas, "closest", &[text(".cd-fitting")]);
    if !exists(&fitting) {
        return 0.0;
    }
    let sheet = call(&fitting, "querySelector", &[text(".cd-create")]);
    if !exists(&sheet) || yes(&sheet, "hidden") {
        return 0.0;
    }
    let a = call(&canvas, "getBoundingClientRect", &[]);
    let b = call(&sheet, "getBoundingClientRect", &[]);
    if number(&b, "height") <= 0.0 {
        return 0.0;
    }
    (number(&a, "bottom") - number(&b, "top")).max(0.0)
}
/// While a look is being made the bay is alive: dust drifts, lamps breathe, she breathes.
fn creation_loop(s: &JsValue) {
    if !yes(s, "creating") || !yes(s, "ready") || !yes(s, "active") || yes(s, "disposed") {
        return;
    }
    if exists(&get(s, "loopFrame")) {
        return;
    }
    set(s, "looping", &flag(true));
    set(s, "loopLast", &num(now()));
    loop_frame(s);
}
fn loop_frame(s: &JsValue) {
    let a = s.clone();
    let id = call(
        &global(),
        "requestAnimationFrame",
        &[function(move |_| {
            set(&a, "loopFrame", &JsValue::NULL);
            if !yes(&a, "creating")
                || !yes(&a, "ready")
                || !yes(&a, "active")
                || yes(&a, "disposed")
            {
                set(&a, "looping", &flag(false));
                return;
            }
            let time = now();
            let dt = ((time - number(&a, "loopLast")) / 1000.0).clamp(0.0, 0.1);
            set(&a, "loopLast", &num(time));
            let mixer = get(&a, "mixer");
            if exists(&mixer) {
                call(&mixer, "update", &[num(dt)]);
            }
            let t = number(&a, "focusT").clamp(0.0, 1.0);
            let renderer = get(&a, "renderer");
            let buffer = call(
                &renderer,
                "getDrawingBufferSize",
                &[make(&get(&a, "host"), "THREE.Vector2", &[])],
            );
            super::bay::tick(
                &a,
                time / 1000.0,
                t * t * (3.0 - 2.0 * t),
                number(&buffer, "y"),
            );
            aim(&a);
            call(&renderer, "render", &[get(&a, "scene"), get(&a, "camera")]);
            loop_frame(&a);
        })],
    );
    set(s, "loopFrame", &id);
}
/// Centre of the head in world space. Measured once per figure and dress in the turntable's
/// frame, so it follows the figure as it turns.
fn head(s: &JsValue) -> [f64; 3] {
    let h = get(s, "host");
    let rotating = get(s, "stage.rotating");
    call(&rotating, "updateMatrixWorld", &[flag(true)]);
    if !exists(&get(s, "head")) {
        let figure = get(s, "figure");
        let local = if exists(&figure) {
            measure_head(&h, &rotating, &figure)
        } else {
            [0.0, number(&h, "PLATFORM_TOP") + 1.66, 0.04]
        };
        set(s, "head", &floats(&local));
    }
    let c = components(&get(s, "head"));
    let v = make(&h, "THREE.Vector3", &[num(c[0]), num(c[1]), num(c[2])]);
    call(&rotating, "localToWorld", &[v.clone()]);
    [number(&v, "x"), number(&v, "y"), number(&v, "z")]
}
/// Eye height a little under the crown; the face's depth from a ray shot at it from the front.
fn measure_head(h: &JsValue, rotating: &JsValue, figure: &JsValue) -> [f64; 3] {
    let top = crown(h, figure);
    let eye = make(h, "THREE.Vector3", &[num(0.0), num(top - 0.115), num(0.0)]);
    call(rotating, "worldToLocal", &[eye.clone()]);
    let y = number(&eye, "y");
    let origin = make(h, "THREE.Vector3", &[num(0.0), num(y), num(3.0)]);
    call(rotating, "localToWorld", &[origin.clone()]);
    let direction = make(h, "THREE.Vector3", &[num(0.0), num(0.0), num(-1.0)]);
    call(
        &direction,
        "transformDirection",
        &[get(rotating, "matrixWorld")],
    );
    let ray = make(
        h,
        "THREE.Raycaster",
        &[origin, direction, num(0.0), num(6.0)],
    );
    let hits = call(&ray, "intersectObject", &[figure.clone(), flag(true)]);
    for hit in Array::from(&hits).iter() {
        let mut node = get(&hit, "object");
        let mut seen = true;
        while exists(&node) {
            seen &= yes(&node, "visible");
            node = get(&node, "parent");
        }
        if seen {
            let point = get(&hit, "point");
            call(rotating, "worldToLocal", &[point.clone()]);
            // The skull's centre sits about a hand behind the face.
            return [number(&point, "x"), y, number(&point, "z") - 0.09];
        }
    }
    [0.0, y, 0.03]
}
/// Top of the visible figure as it stands. A skinned mesh's own box follows its bones; its
/// geometry's box is the bind pose, which sits far lower on the rigged female.
fn crown(h: &JsValue, figure: &JsValue) -> f64 {
    let b = make(h, "THREE.Box3", &[]);
    for n in nodes(figure, true) {
        if !yes(&n, "isMesh") {
            continue;
        }
        let local = if yes(&n, "isSkinnedMesh") {
            call(&n, "computeBoundingBox", &[]);
            get(&n, "boundingBox")
        } else {
            let g = get(&n, "geometry");
            call(&g, "computeBoundingBox", &[]);
            get(&g, "boundingBox")
        };
        if exists(&local) {
            let world = call(&local, "clone", &[]);
            call(&world, "applyMatrix4", &[get(&n, "matrixWorld")]);
            call(&b, "union", &[world]);
        }
    }
    number(&b, "max.y")
}
/// The turntable and the bay floor show only in the fitting room.
fn show_stand(s: &JsValue) {
    let on = !yes(s, "creating");
    super::bay::show(s, !on);
    set(&get(s, "stage.fixed"), "visible", &flag(on));
    let keep = [
        get(s, "figure"),
        get(s, "equipment"),
        get(s, "stage.feet"),
        get(s, "podium"),
    ];
    for child in Array::from(&get(s, "stage.rotating.children")).iter() {
        if !keep.contains(&child) {
            set(&child, "visible", &flag(on));
        }
    }
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
    if yes(s, "creating") {
        // The close-up keeps the head centred as it turns.
        aim(s);
    }
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
/// The real model of the shown weapon class, once loaded.
fn shown_gun(s: &JsValue) -> JsValue {
    get(&get(s, "guns"), &weapon(s))
}
fn mod_meshes(gun: &JsValue) -> JsValue {
    let names = Array::new();
    if exists(gun) {
        for n in nodes(gun, false) {
            if get(&n, "userData.factory").as_bool() == Some(false) {
                names.push(&get(&n, "name"));
            }
        }
    }
    names.into()
}
/// Loads the class's gun once; until then the class shows its placeholder boxes.
fn load_gun(s: &JsValue, class: String) -> Promise {
    let guns = get(s, "guns");
    let Some(&(_, model)) = super::scene_assets::GUN_MODELS
        .iter()
        .find(|(c, _)| *c == class)
    else {
        return Promise::resolve(&JsValue::UNDEFINED);
    };
    if !exists(&guns) || !get(&guns, &class).is_undefined() {
        return Promise::resolve(&JsValue::UNDEFINED);
    }
    set(&guns, &class, &JsValue::NULL);
    let a = s.clone();
    future_to_promise(async move {
        let h = get(&a, "host");
        let url = format!("{}preview/{model}.json", string(&h, "base"));
        if let Ok(p) = try_call(&h, "loadModel", &[text(&url)]) {
            if let Ok(gltf) = wait(p).await {
                let root = get(&gltf, "scene");
                if yes(&a, "disposed") || !exists(&get(&a, "renderer")) {
                    dispose(&root, &[], true);
                } else {
                    set(&root, "userData.slot", &text("weapon1"));
                    set(&root, "visible", &flag(false));
                    call(&h, "prepareVaryag", &[root.clone()]);
                    call(&get(&a, "equipment"), "add", &[root.clone()]);
                    tune(&a, &root);
                    set(&get(&a, "guns"), &class, &root);
                    if yes(&a, "ready") {
                        apply_weapon(&a);
                        render(&a);
                    }
                }
            }
        }
        Ok(JsValue::UNDEFINED)
    })
}
fn apply_weapon(s: &JsValue) {
    if !exists(&get(s, "figure")) {
        return;
    }
    let h = get(s, "host");
    let weapon = weapon(s);
    let gun = shown_gun(s);
    let real = exists(&gun);
    if !real {
        let _ = load_gun(s, weapon.clone());
    }
    method_if(
        &get(s, "hand"),
        "hold",
        &[if real { JsValue::NULL } else { text(&weapon) }],
    );
    let guns = get(s, "guns");
    for class in Object::keys(guns.unchecked_ref::<Object>()).iter() {
        let other = get(&guns, &class.as_string().unwrap_or_default());
        if exists(&other) {
            set(&other, "visible", &flag(Object::is(&other, &gun)));
        }
    }
    if real {
        let loadout = get(s, "loadout");
        call(
            &h,
            "fitVaryag",
            &[
                gun.clone(),
                if exists(&loadout) {
                    loadout
                } else {
                    JsValue::NULL
                },
            ],
        );
    }
    let mount = if real { gun } else { get(s, "hand.mount") };
    if !exists(&mount) {
        return;
    }
    set(&mount, "userData.slot", &text("weapon1"));
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
    // A real gun hangs as fitted to this Persona's pose; anything without a fit keeps the generic sling.
    let carry = if real {
        call(
            &h,
            "makeCarry",
            &[mount.clone(), text(&string(s, "figureSex")), text(&weapon)],
        )
    } else {
        JsValue::NULL
    };
    let sling = if exists(&carry) {
        carry
    } else {
        call(&h, "makeSling", &[mount, flag(weapon == "pistol")])
    };
    set(&sling, "userData.slot", &text("weapon1"));
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
    // While the creation bay is alive its own loop draws every frame.
    if yes(s, "looping") {
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
    method_if(&get(s, "hand"), "dispose", &[]);
    if yes(s, "gpuReleased") {
        return;
    }
    set(s, "gpuReleased", &flag(true));
    super::persona_hd::release(s);
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
    for name in ["figure", "hand", "guns", "tiers", "sling"] {
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
/// Fresnel rim drawn over the hovered equipment: cold light hugging the silhouette.
const GLOW_VERTEX: &str = "varying vec3 vN; varying vec3 vV;
void main() {
  vec4 mv = modelViewMatrix * vec4(position, 1.0);
  vN = normalize(normalMatrix * normal);
  vV = normalize(-mv.xyz);
  gl_Position = projectionMatrix * mv;
}";
const GLOW_FRAGMENT: &str = "uniform vec3 uColor; varying vec3 vN; varying vec3 vV;
void main() {
  float rim = pow(1.0 - abs(dot(normalize(vN), normalize(vV))), 3.0);
  gl_FragColor = vec4(uColor * (0.07 + rim * 0.75), 1.0);
}";
/// Overlays live on layer 1: the camera draws them, the picking ray ignores them.
const GLOW_LAYER: f64 = 1.0;
/// A finger held still this long inspects instead of turning the stand.
const HOLD_MS: f64 = 380.0;
fn glow_material(s: &JsValue) -> JsValue {
    let existing = get(s, "glowMaterial");
    if exists(&existing) {
        return existing;
    }
    let h = get(s, "host");
    let color = make(&h, "THREE.Color", &[num(0xdfe9ee as f64)]);
    let material = make(
        &h,
        "THREE.ShaderMaterial",
        &[options(&[
            ("vertexShader", text(GLOW_VERTEX)),
            ("fragmentShader", text(GLOW_FRAGMENT)),
            (
                "uniforms",
                options(&[("uColor", options(&[("value", color)]))]),
            ),
            ("transparent", flag(true)),
            ("depthWrite", flag(false)),
            ("depthFunc", get(&h, "THREE.LessEqualDepth")),
            ("blending", get(&h, "THREE.AdditiveBlending")),
            ("polygonOffset", flag(true)),
            ("polygonOffsetFactor", num(-1.0)),
            ("polygonOffsetUnits", num(-1.0)),
        ])],
    );
    set(s, "glowMaterial", &material);
    call(&get(s, "camera.layers"), "enable", &[num(GLOW_LAYER)]);
    material
}
/// The slot an object belongs to, or "" when it or a parent is hidden.
fn owner(node: &JsValue) -> String {
    let mut slot = String::new();
    let mut o = node.clone();
    while exists(&o) {
        if get(&o, "visible") == JsValue::FALSE {
            return String::new();
        }
        if slot.is_empty() {
            slot = string(&o, "userData.slot");
        }
        o = get(&o, "parent");
    }
    slot
}
fn highlight(s: &JsValue, slot: &str) {
    if yes(s, "disposed") || string(s, "glowing") == slot && exists(&get(s, "glows")) {
        return;
    }
    set(s, "glowing", &text(slot));
    let old = get(s, "glows");
    if exists(&old) {
        for overlay in Array::from(&old).iter() {
            call(&overlay, "removeFromParent", &[]);
        }
    }
    let glows = Array::new();
    if !slot.is_empty() {
        let h = get(s, "host");
        let material = glow_material(s);
        for n in nodes(&get(s, "stage.rotating"), true) {
            if !yes(&n, "isMesh") || yes(&n, "isSkinnedMesh") || yes(&n, "userData.glow") {
                continue;
            }
            if owner(&n) != slot {
                continue;
            }
            let overlay = make(&h, "THREE.Mesh", &[get(&n, "geometry"), material.clone()]);
            set(&overlay, "userData.glow", &flag(true));
            set(&overlay, "renderOrder", &num(2.0));
            set(&overlay, "frustumCulled", &flag(false));
            call(&get(&overlay, "layers"), "set", &[num(GLOW_LAYER)]);
            call(&n, "add", &[overlay.clone()]);
            glows.push(&overlay);
        }
    }
    set(s, "glows", &glows);
    render(s);
}
/// The slot under a screen point: only the nearest surface counts, so the body blocks gear behind it.
fn pick(s: &JsValue, x: f64, y: f64) -> String {
    let hit = first_hit(s, &[get(s, "stage.rotating")], x, y);
    if exists(&hit) {
        owner(&get(&hit, "object"))
    } else {
        String::new()
    }
}
/// The creation step for the part of the Persona under a screen point: hair, eyes, face, skin or suit.
fn pick_part(s: &JsValue, x: f64, y: f64) -> String {
    // She and what she wears, not the podium under her.
    let hit = first_hit(s, &[get(s, "figure"), get(s, "equipment")], x, y);
    if !exists(&hit) {
        return String::new();
    }
    let object = get(&hit, "object");
    let material = get(&object, "material");
    let name = string(&material, "name");
    if name.contains("Hair") {
        return "hair".into();
    }
    if name.starts_with("M_Head_") {
        // The eye shader keeps both eye spots in the head's own space; a tap near one picks the eyes.
        let positions = get(&object, "geometry.attributes.position");
        let at = number(&hit, "face.a");
        let p = ["getX", "getY", "getZ"]
            .map(|k| call(&positions, k, &[num(at)]).as_f64().unwrap_or(0.0));
        for key in ["cdEyeA", "cdEyeB"] {
            let eye = components(&get(&material, &format!("userData.cdLook.{key}.value")));
            if eye.len() == 4
                && (0..3).map(|i| (p[i] - eye[i]).powi(2)).sum::<f64>().sqrt() < eye[3] * 2.2
            {
                return "eyes".into();
            }
        }
        return "face".into();
    }
    if name == "M_Male_Body" || name == "M_Fem_Body" {
        return "skin".into();
    }
    // Whatever else she wears opens the suit.
    "suit".into()
}
/// The nearest shown mesh under a screen point inside `roots`.
fn first_hit(s: &JsValue, roots: &[JsValue], x: f64, y: f64) -> JsValue {
    let roots: Vec<JsValue> = roots.iter().filter(|r| exists(r)).cloned().collect();
    if !yes(s, "ready") || roots.is_empty() {
        return JsValue::UNDEFINED;
    }
    let h = get(s, "host");
    let rect = call(&get(s, "canvas"), "getBoundingClientRect", &[]);
    let (w, hgt) = (number(&rect, "width"), number(&rect, "height"));
    if w <= 0.0 || hgt <= 0.0 {
        return JsValue::UNDEFINED;
    }
    let point = make(
        &h,
        "THREE.Vector2",
        &[
            num((x - number(&rect, "left")) / w * 2.0 - 1.0),
            num(1.0 - (y - number(&rect, "top")) / hgt * 2.0),
        ],
    );
    let mut ray = get(s, "raycaster");
    if !exists(&ray) {
        ray = make(&h, "THREE.Raycaster", &[]);
        set(s, "raycaster", &ray);
    }
    call(&ray, "setFromCamera", &[point, get(s, "camera")]);
    let hits = call(&ray, "intersectObjects", &[array(&roots), flag(true)]);
    for hit in Array::from(&hits).iter() {
        let object = get(&hit, "object");
        if yes(&object, "userData.glow") || !yes(&object, "isMesh") {
            continue;
        }
        // Hidden parts (the rifle while another gun is held) must not catch the ray.
        let mut o = object.clone();
        let mut shown = true;
        while exists(&o) {
            shown &= get(&o, "visible") != JsValue::FALSE;
            o = get(&o, "parent");
        }
        if shown {
            return hit;
        }
    }
    JsValue::UNDEFINED
}
/// Glows the slot and tells the fitting screen so its button can open.
fn hover(s: &JsValue, slot: &str) {
    if string(s, "hover") == slot {
        return;
    }
    set(s, "hover", &text(slot));
    highlight(s, slot);
    let event = make(
        &global(),
        "CustomEvent",
        &[
            text("persona-hover"),
            options(&[
                (
                    "detail",
                    if slot.is_empty() {
                        JsValue::NULL
                    } else {
                        text(slot)
                    },
                ),
                ("bubbles", flag(true)),
            ]),
        ],
    );
    call(&get(s, "canvas"), "dispatchEvent", &[event]);
}
fn clear_hold(s: &JsValue) {
    let timer = get(s, "holdTimer");
    if exists(&timer) {
        call(&global(), "clearTimeout", &[timer]);
        set(s, "holdTimer", &JsValue::NULL);
    }
}
fn input(s: &JsValue, event: &str, e: &JsValue) {
    let canvas = get(s, "canvas");
    let mouse = string(e, "pointerType") == "mouse";
    match event {
        "pointerdown" => {
            if !yes(s, "active") || number(e, "button") != 0.0 || !yes(e, "isPrimary") {
                return;
            }
            let (x, y) = (number(e, "clientX"), number(e, "clientY"));
            set(
                s,
                "drag",
                &options(&[
                    ("id", get(e, "pointerId")),
                    ("x", num(x)),
                    ("y", num(y)),
                    ("startX", num(x)),
                    ("startY", num(y)),
                    ("at", num(now())),
                    ("hold", flag(false)),
                ]),
            );
            call(&canvas, "setPointerCapture", &[get(e, "pointerId")]);
            clear_hold(s);
            if !mouse && !yes(s, "creating") {
                let a = s.clone();
                let id = get(e, "pointerId");
                let timer = call(
                    &global(),
                    "setTimeout",
                    &[
                        function(move |_| {
                            set(&a, "holdTimer", &JsValue::NULL);
                            if get(&a, "drag.id") != id {
                                return;
                            }
                            set(&a, "drag.hold", &flag(true));
                            let slot = pick(&a, number(&a, "drag.x"), number(&a, "drag.y"));
                            hover(&a, &slot);
                        }),
                        num(HOLD_MS),
                    ],
                );
                set(s, "holdTimer", &timer);
            }
        }
        "pointermove" => {
            let (x, y) = (number(e, "clientX"), number(e, "clientY"));
            if !exists(&get(s, "drag")) || get(s, "drag.id") != get(e, "pointerId") {
                if mouse && yes(s, "active") && yes(s, "creating") {
                    let part = pick_part(s, x, y);
                    let cursor = if part.is_empty() { "grab" } else { "pointer" };
                    set(&canvas, "style.cursor", &text(cursor));
                } else if mouse && yes(s, "active") {
                    let slot = pick(s, x, y);
                    hover(s, &slot);
                }
                return;
            }
            if yes(s, "drag.hold") {
                // Holding: the finger scans the figure instead of turning it.
                set(s, "drag.x", &num(x));
                set(s, "drag.y", &num(y));
                let slot = pick(s, x, y);
                hover(s, &slot);
                return;
            }
            if (x - number(s, "drag.startX")).hypot(y - number(s, "drag.startY")) > 8.0 {
                clear_hold(s);
            }
            rotate(s, (x - number(s, "drag.x")) * 0.009);
            set(s, "drag.x", &num(x));
            set(s, "drag.y", &num(y));
            if mouse && !yes(s, "creating") {
                let slot = pick(s, x, y);
                hover(s, &slot);
            }
        }
        "pointerup" | "pointercancel" | "lostpointercapture" => {
            if get(s, "drag.id") != get(e, "pointerId") {
                return;
            }
            clear_hold(s);
            if !mouse {
                hover(s, "");
            }
            let (x, y) = (number(e, "clientX"), number(e, "clientY"));
            let tap = event == "pointerup"
                && (x - number(s, "drag.startX")).hypot(y - number(s, "drag.startY")) < 8.0
                && now() - number(s, "drag.at") < 500.0;
            set(s, "drag", &JsValue::NULL);
            if tap && yes(s, "creating") {
                let part = pick_part(s, x, y);
                if !part.is_empty() {
                    let event = make(
                        &global(),
                        "CustomEvent",
                        &[
                            text("persona-part"),
                            options(&[("detail", text(&part)), ("bubbles", flag(true))]),
                        ],
                    );
                    call(&canvas, "dispatchEvent", &[event]);
                }
            }
            if call(&canvas, "hasPointerCapture", &[get(e, "pointerId")]).is_truthy() {
                call(&canvas, "releasePointerCapture", &[get(e, "pointerId")]);
            }
        }
        "pointerleave" => {
            if mouse && !exists(&get(s, "drag")) {
                hover(s, "");
            }
        }
        "contextmenu" => {
            // A long press must inspect the gear, not open the browser menu.
            call(e, "preventDefault", &[]);
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
