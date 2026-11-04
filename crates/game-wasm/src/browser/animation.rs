//! Animation clip preparation, mixer command execution and event ownership.
use super::{bridge::*, kinematics::*};
use js_sys::Float32Array;
use serde_json::{Value, json};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::prelude::*;

fn is_leg(name: &str) -> bool {
    [
        "root",
        "pelvis",
        "thigh_",
        "calf_",
        "foot_",
        "ball_",
        "mixamorigHips",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
        || ["Left", "Right"].iter().any(|side| {
            ["UpLeg", "Leg", "Foot", "ToeBase", "Toe_End"]
                .iter()
                .any(|part| name.starts_with(&format!("mixamorig{side}{part}")))
        })
}
fn soften(clip: &JsValue) {
    for track in values(&get(clip, "tracks")) {
        let data = get(&track, "values");
        let is_quaternion = string(&get(&track, "ValueTypeName")) == "quaternion";
        let mut input = vec![
            num(&call(&track, "getValueSize", &[])),
            // A third of the authored sway, blurred over ~0.25 s: at 0.55 and 0.1 s the chest
            // jerked forward every loop (the enemy while you pick a move, your elbows in first person).
            0.3,
            6.,
            f64::from(is_quaternion),
        ];
        input.extend(numbers(&data));
        call(
            &data,
            "set",
            &[js_sys::Float64Array::from(math("soften", &input).as_slice()).into()],
        );
    }
}
struct MotionState {
    three: JsValue,
    mixer: JsValue,
    layers: [HashMap<String, JsValue>; 2],
    order: Vec<String>,
    current: [JsValue; 2],
    timeline: Value,
    pelvis_of: HashMap<String, JsValue>,
    pelvis: JsValue,
    spine: JsValue,
    mark: JsValue,
    hold: JsValue,
    disposed: bool,
}
impl MotionState {
    /// External callbacks are returned to the caller, after the RefCell borrow ends.
    fn event(&mut self, event: &str, extra: Value) -> Vec<(JsValue, JsValue)> {
        if self.disposed {
            return vec![];
        }
        // A frame's update with no shot, wait or re-arm under way changes nothing: skip the timeline.
        if event == "update"
            && ["rearm", "pending", "kind"]
                .iter()
                .all(|k| self.timeline[*k].is_null())
        {
            return vec![];
        }
        let mut args = extra;
        args["state"] = std::mem::take(&mut self.timeline);
        args["event"] = event.into();
        let mut result = game_motion::dispatch("motion.timeline", args).expect("motion timeline");
        self.timeline = result["state"].take();
        let mut callbacks = vec![];
        for command in result["commands"].as_array().expect("timeline commands") {
            let op = command["op"].as_str().unwrap_or_default();
            match op {
                "hold" | "mark" => {
                    let cb = if op == "hold" { &self.hold } else { &self.mark };
                    if present(cb) {
                        callbacks.push((cb.clone(), super::bridge::json(&command["value"])));
                    }
                    continue;
                }
                "sync" => {
                    if self.current.iter().all(present) {
                        set(&self.current[1], "time", &get(&self.current[0], "time"));
                    }
                    continue;
                }
                "stopAll" => {
                    for layer in &self.layers {
                        for name in &self.order {
                            call(&layer[name], "stop", &[]);
                        }
                    }
                    self.current = [JsValue::NULL, JsValue::NULL];
                    continue;
                }
                _ => {}
            }
            let layer = usize::from(command["layer"] == "legs");
            if op == "speed" || op == "time" {
                if present(&self.current[layer]) {
                    set(
                        &self.current[layer],
                        if op == "speed" { "timeScale" } else { "time" },
                        &super::bridge::json(&command["value"]),
                    );
                }
                continue;
            }
            let Some(action) = self.layers[layer].get(command["name"].as_str().unwrap_or_default())
            else {
                continue;
            };
            set(action, "timeScale", &super::bridge::json(&command["speed"]));
            if command["reset"].as_bool().unwrap_or(false) {
                call(action, "reset", &[]);
                let once = command["once"].as_bool().unwrap_or(false);
                call(
                    action,
                    "setLoop",
                    &[
                        get(&self.three, if once { "LoopOnce" } else { "LoopRepeat" }),
                        f64::INFINITY.into(),
                    ],
                );
                set(action, "clampWhenFinished", &once.into());
                call(action, "setEffectiveWeight", &[1.into()]);
                call(action, "fadeIn", &[super::bridge::json(&command["fade"])]);
                call(action, "play", &[]);
                let previous = &self.current[layer];
                if present(previous) && previous != action {
                    call(
                        previous,
                        "fadeOut",
                        &[super::bridge::json(&command["fade"])],
                    );
                }
                self.current[layer] = action.clone();
            }
        }
        callbacks
    }
}
fn send(state: &Rc<RefCell<MotionState>>, event: &str, extra: Value) {
    let callbacks = state.borrow_mut().event(event, extra);
    for (callback, value) in callbacks {
        if state.borrow().disposed {
            break;
        }
        invoke(&callback, &[value]);
    }
}

#[wasm_bindgen]
pub struct RustMotion {
    state: Rc<RefCell<MotionState>>,
    finished: Option<Closure<dyn FnMut(JsValue)>>,
}
#[wasm_bindgen]
impl RustMotion {
    #[wasm_bindgen(constructor)]
    pub fn new(three: JsValue, mixer: JsValue, clips: JsValue) -> Self {
        let clips = values(&clips);
        for clip in &clips {
            if string(&get(clip, "name")).starts_with("ready_") {
                soften(clip);
            }
        }
        let mut layers = [HashMap::new(), HashMap::new()];
        let mut order = vec![];
        let mut pelvis_of = HashMap::new();
        let mut durations = serde_json::Map::new();
        for clip in &clips {
            let name = string(&get(clip, "name"));
            if !order.contains(&name) {
                order.push(name.clone());
            }
            let duration = get(clip, "duration");
            durations.insert(name.clone(), num(&duration).into());
            let tracks = values(&get(clip, "tracks"));
            // Preserve original creation order: legs, then body.
            for (layer, suffix) in [(1, "legs"), (0, "body")] {
                let split = tracks
                    .iter()
                    .filter(|track| is_leg(&string(&get(track, "name"))) == (layer == 1))
                    .cloned()
                    .collect::<Vec<_>>();
                let split = make(
                    &three,
                    "AnimationClip",
                    &[
                        format!("{name}.{suffix}").into(),
                        duration.clone(),
                        array(&split).into(),
                    ],
                );
                layers[layer].insert(name.clone(), call(&mixer, "clipAction", &[split]));
            }
            if let Some(turn) = tracks.iter().find(|track| {
                matches!(
                    string(&get(track, "name")).as_str(),
                    "pelvis.quaternion" | "mixamorigHips.quaternion"
                )
            }) {
                pelvis_of.insert(
                    name,
                    make(
                        &three,
                        "QuaternionLinearInterpolant",
                        &[
                            get(turn, "times"),
                            get(turn, "values"),
                            4.into(),
                            Float32Array::new_with_length(4).into(),
                        ],
                    ),
                );
            }
        }
        let root = call(&mixer, "getRoot", &[]);
        let state = Rc::new(RefCell::new(MotionState {
            three,
            mixer: mixer.clone(),
            layers,
            order,
            current: [JsValue::NULL, JsValue::NULL],
            timeline: Value::Null,
            pelvis_of,
            pelvis: find(&root, &["pelvis", "mixamorigHips"]),
            spine: find(&root, &["spine_01", "mixamorigSpine"]),
            mark: JsValue::NULL,
            hold: JsValue::NULL,
            disposed: false,
        }));
        send(&state, "create", json!({"durations": durations}));
        let weak = Rc::downgrade(&state);
        let finished = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
            let Some(state) = weak.upgrade() else {
                return;
            };
            let done = {
                let s = state.borrow();
                !s.timeline["kind"].is_null() && get(&event, "action") == s.current[0]
            };
            if done {
                send(&state, "done", json!({}));
            }
        });
        call(
            &mixer,
            "addEventListener",
            &["finished".into(), finished.as_ref().clone()],
        );
        Self {
            state,
            finished: Some(finished),
        }
    }
    #[wasm_bindgen(getter, js_name = onMark)]
    pub fn on_mark(&self) -> JsValue {
        self.state.borrow().mark.clone()
    }
    #[wasm_bindgen(setter, js_name = onMark)]
    pub fn set_on_mark(&self, callback: JsValue) {
        self.state.borrow_mut().mark = callback;
    }
    #[wasm_bindgen(getter, js_name = onHold)]
    pub fn on_hold(&self) -> JsValue {
        self.state.borrow().hold.clone()
    }
    #[wasm_bindgen(setter, js_name = onHold)]
    pub fn set_on_hold(&self, callback: JsValue) {
        self.state.borrow_mut().hold = callback;
    }
    #[wasm_bindgen(getter)]
    pub fn stances(&self) -> JsValue {
        array(&[
            "ready_pistol".into(),
            "ready_rifle".into(),
            "ready_shotgun".into(),
        ])
        .into()
    }
    #[wasm_bindgen(getter, js_name = attackKind)]
    pub fn attack_kind(&self) -> JsValue {
        super::bridge::json(&self.state.borrow().timeline["kind"])
    }
    #[wasm_bindgen(getter)]
    pub fn planted(&self) -> JsValue {
        super::bridge::json(&self.state.borrow().timeline["planted"])
    }
    #[wasm_bindgen(getter)]
    pub fn gait(&self) -> JsValue {
        let s = self.state.borrow();
        if s.timeline["walking"].as_bool().unwrap_or(false) {
            super::bridge::json(&s.timeline["legs"])
        } else {
            JsValue::NULL
        }
    }
    #[wasm_bindgen(getter, js_name = pistolReady)]
    pub fn pistol_ready(&self) -> f64 {
        let s = self.state.borrow();
        s.layers[0]
            .get("ready_pistol")
            .filter(|action| boolean(&call(action, "isRunning", &[])))
            .map(|action| num(&call(action, "getEffectiveWeight", &[])))
            .unwrap_or(0.)
    }
    /// How much of the body holds a gun up in both hands, ready or firing (0..1).
    #[wasm_bindgen(getter, js_name = gunReady)]
    pub fn gun_ready(&self) -> f64 {
        let s = self.state.borrow();
        let held = ["ready_", "fire_"].iter().flat_map(|clip| {
            ["pistol", "rifle", "shotgun"].map(|gun| format!("{clip}{gun}"))
        });
        held.filter_map(|name| s.layers[0].get(&name))
            .filter(|action| boolean(&call(action, "isRunning", &[])))
            .map(|action| num(&call(action, "getEffectiveWeight", &[])))
            .sum::<f64>()
            .min(1.)
    }
    pub fn arm(&self, stance: &str) {
        send(&self.state, "arm", json!({"stance": stance}));
    }
    pub fn walk(&self, clip: &str, at: f64) {
        send(&self.state, "walk", json!({"clip": clip, "at": at}));
    }
    pub fn pace(&self, speed: f64) {
        send(&self.state, "pace", json!({"speed": speed}));
    }
    pub fn stand(&self, fade: Option<f64>) {
        send(&self.state, "stand", json!({"fade": fade.unwrap_or(0.35)}));
    }
    pub fn attack(&self, kind: &str, seconds: f64) {
        send(
            &self.state,
            "attack",
            json!({"kind": kind, "inSeconds": seconds}),
        );
    }
    pub fn fall(&self) {
        send(&self.state, "fall", json!({}));
    }
    pub fn reset(&self, phase: f64) {
        send(&self.state, "reset", json!({"phase": phase}));
    }
    pub fn update(&self, dt: f64) {
        let time = num(&get(&self.state.borrow().current[0], "time"));
        send(&self.state, "update", json!({"dt": dt, "time": time}));
    }
    pub fn settle(&self) {
        let s = self.state.borrow();
        if !present(&s.pelvis) || !present(&s.spine) || s.disposed {
            return;
        }
        let mut input = vec![0.; 8];
        super::packet::quat_of(&s.pelvis, &mut input[..4]);
        super::packet::quat_of(&s.spine, &mut input[4..]);
        for name in &s.order {
            let action = &s.layers[0][name];
            let weight = if boolean(&call(action, "isRunning", &[])) {
                num(&call(action, "getEffectiveWeight", &[]))
            } else {
                0.
            };
            if weight > 0. {
                if let Some(turn) = s.pelvis_of.get(name) {
                    input.extend(numbers(&call(turn, "evaluate", &[get(action, "time")])));
                    input.push(weight);
                }
            }
        }
        super::packet::set_quat(&s.spine, &math("settle", &input));
    }
    pub fn dispose(&self) {
        let mut s = self.state.borrow_mut();
        if s.disposed {
            return;
        }
        s.disposed = true;
        if let Some(listener) = &self.finished {
            call(
                &s.mixer,
                "removeEventListener",
                &["finished".into(), listener.as_ref().clone()],
            );
        }
        s.mark = JsValue::NULL;
        s.hold = JsValue::NULL;
    }
}
impl Drop for RustMotion {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[wasm_bindgen]
pub struct RustPose {
    /// The skeleton's bones; their positions and quaternions are saved as one packet.
    bones: js_sys::Array,
    saved: RefCell<Vec<f64>>,
}
#[wasm_bindgen]
impl RustPose {
    #[wasm_bindgen(constructor)]
    pub fn new(model: &JsValue) -> Self {
        let bones: js_sys::Array = descendants(model)
            .into_iter()
            .filter(|o| boolean(&get(o, "isBone")))
            .collect();
        let mut saved = vec![0.; bones.length() as usize * 7];
        super::packet::read_trs(&bones, &mut saved);
        Self {
            bones,
            saved: RefCell::new(saved),
        }
    }
    pub fn save(&self) {
        super::packet::read_trs(&self.bones, &mut self.saved.borrow_mut());
    }
    pub fn restore(&self) {
        super::packet::write_trs(&self.bones, &self.saved.borrow());
    }
}
