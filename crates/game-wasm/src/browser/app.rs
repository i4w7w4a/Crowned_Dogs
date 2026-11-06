//! Application routes and cancellable loading. JavaScript supplies dynamic imports only.
use super::{bridge::*, dom_support::Listener};
use js_sys::Promise;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, future_to_promise};

fn result(value: Result<JsValue, JsValue>) -> JsValue {
    value.unwrap_or_else(|error| Promise::reject(&error).into())
}
fn abort_check(signal: &JsValue) -> Result<(), JsValue> {
    if present(signal) {
        try_call(signal, "throwIfAborted", &[])?;
    }
    Ok(())
}
#[wasm_bindgen]
pub fn activate_after_ready(
    prepare: JsValue,
    before: JsValue,
    enter: JsValue,
    signal: JsValue,
) -> Promise {
    let prepared =
        result(abort_check(&signal).and_then(|_| apply(&prepare, &JsValue::UNDEFINED, &[])));
    let ready_signal = signal.clone();
    let ready = Closure::<dyn FnMut(JsValue) -> JsValue>::new(move |_| {
        result(abort_check(&ready_signal).and_then(|_| apply(&before, &JsValue::UNDEFINED, &[])))
    })
    .into_js_value();
    let active = Closure::<dyn FnMut(JsValue) -> JsValue>::new(move |_| {
        result(abort_check(&signal).and_then(|_| apply(&enter, &JsValue::UNDEFINED, &[])))
    })
    .into_js_value();
    let p = call(&Promise::resolve(&prepared).into(), "then", &[ready]);
    call(&p, "then", &[active]).unchecked_into()
}

struct Pending {
    signal: JsValue,
    raced: Promise,
    _listener: Option<Listener>,
    _parent_listener: Option<Listener>,
}
impl Pending {
    fn new(launch: &JsValue, view: &JsValue, parent: &JsValue) -> Result<Self, JsValue> {
        let controller = construct(&get(&global(), "AbortController"), &[]);
        let signal = get(&controller, "signal");
        let parent_controller = controller.clone();
        let parent_listener = if present(parent) {
            if boolean(&get(parent, "aborted")) {
                call(&controller, "abort", &[]);
            }
            Some(Listener::new(parent, "abort", move |_| {
                call(&parent_controller, "abort", &[]);
            }))
        } else {
            None
        };
        let cancel = Closure::<dyn FnMut()>::new(move || {
            call(&controller, "abort", &[]);
        })
        .into_js_value();
        try_call(view, "show", &[cancel])?;
        let ready_view = view.clone();
        let ready = Closure::<dyn FnMut() -> JsValue>::new(move || {
            result(try_call(&ready_view, "ready", &[]))
        })
        .into_js_value();
        let running = Promise::resolve(&result(apply(
            launch,
            &JsValue::UNDEFINED,
            &[ready, signal.clone()],
        )));
        let mut listener = None;
        let cancelled = Promise::new(&mut |_, reject| {
            let reject = JsValue::from(reject);
            let abort = move || {
                let error = construct(
                    &get(&global(), "DOMException"),
                    &["Loading cancelled".into(), "AbortError".into()],
                );
                invoke(&reject, &[error]);
            };
            if boolean(&get(&signal, "aborted")) {
                abort();
            } else {
                listener = Some(Listener::new(&signal, "abort", move |_| abort()));
            }
        });
        Ok(Self {
            signal,
            raced: Promise::race(&array(&[running.into(), cancelled.into()])),
            _listener: listener,
            _parent_listener: parent_listener,
        })
    }
}
#[wasm_bindgen]
pub fn run_battle_loading(launch: JsValue, view: JsValue) -> Promise {
    load_with_parent(launch, view, JsValue::UNDEFINED)
}
fn load_with_parent(launch: JsValue, view: JsValue, parent: JsValue) -> Promise {
    // Install Back synchronously, matching an async JavaScript function before its first await.
    let first = Pending::new(&launch, &view, &parent);
    future_to_promise(async move {
        let outcome = async {
            let mut pending = first?;
            loop {
                let error = match JsFuture::from(pending.raced.clone()).await {
                    Ok(_) => return Ok(true.into()),
                    Err(error) => error,
                };
                if boolean(&get(&pending.signal, "aborted")) || string(&get(&error,"name"))=="AbortError" {
                    return Ok(false.into());
                }
                call(&get(&global(), "console"), "error", &["crowned-dogs: the arena failed to load".into(), error.clone()]);
                let detail = if error.is_object() { get(&error, "message") } else { error.clone() };
                let detail = detail.as_string().unwrap_or_else(|| string(&error));
                let choice = JsFuture::from(Promise::resolve(&try_call(&view, "failed", &[detail.into()])?)).await?;
                if string(&choice) != "retry" {
                    return Ok(false.into());
                }
                pending = Pending::new(&launch, &view, &parent)?;
            }
        }
        .await;
        try_call(&view, "hide", &[])?;
        outcome
    })
}

fn view_hash() -> JsValue {
    match string(&get(&get(&global(), "location"), "hash")).as_str() {
        "#weapon" => "weapon".into(),
        "#fitting" => "fitting".into(),
        "#persona" => "persona".into(),
        "#region" => "region".into(),
        _ => "base".into(),
    }
}
fn available(s: &JsValue) -> bool {
    !boolean(&get(s, "disposed")) && !boolean(&get(s, "inBattle")) && !boolean(&get(s, "loading"))
}
fn show_base(s: &JsValue, view: JsValue) {
    call(&get(s, "base"), "show", &[view]);
    invoke(&get(&get(s, "host"), "showSoundSettings"), &[true.into()]);
}
fn return_base(s: &JsValue) {
    if boolean(&get(s, "disposed")) {
        return;
    }
    set(s, "inBattle", &false.into());
    call(
        &get(&global(), "history"),
        "replaceState",
        &[JsValue::NULL, "".into(), "#base".into()],
    );
    show_base(s, "base".into());
    call(&get(s, "base"), "setActive", &[true.into()]);
    let button = call(&get(s, "root"), "querySelector", &[".cd-enter".into()]);
    if present(&button) {
        call(&button, "focus", &[]);
    }
}
fn enter_battle(s: JsValue, loadout: JsValue) -> Promise {
    if !available(&s) {
        return Promise::resolve(&JsValue::UNDEFINED);
    }
    set(&s, "loading", &true.into());
    let controller = construct(&get(&global(), "AbortController"), &[]);
    set(&s, "controller", &controller);
    call(&get(&s, "base"), "setActive", &[false.into()]);
    invoke(&get(&get(&s, "host"), "showSoundSettings"), &[false.into()]);
    let state = s.clone();
    let launch = Closure::<dyn FnMut(JsValue, JsValue) -> JsValue>::new(move |before, signal| {
        let state = state.clone();
        let loadout = loadout.clone();
        future_to_promise(async move {
            abort_check(&signal)?;
            let battle = JsFuture::from(Promise::resolve(&apply(
                &get(&get(&state, "host"), "loadBattle"),
                &JsValue::UNDEFINED,
                &[],
            )?))
            .await?;
            abort_check(&signal)?;
            let on_exit = Closure::<dyn FnMut()>::new(move || return_base(&state)).into_js_value();
            JsFuture::from(Promise::resolve(&try_call(
                &battle,
                "startBattle",
                &[loadout, on_exit, before, signal],
            )?))
            .await
        })
        .into()
    })
    .into_js_value();
    let loading = load_with_parent(launch, get(&s, "loader"), get(&controller, "signal"));
    future_to_promise(async move {
        let outcome = JsFuture::from(loading).await;
        set(&s, "loading", &false.into());
        set(&s, "controller", &JsValue::NULL);
        let entered = outcome?;
        if !boolean(&get(&s, "disposed")) {
            if boolean(&entered) {
                set(&s, "inBattle", &true.into());
                call(
                    &get(&global(), "history"),
                    "replaceState",
                    &[JsValue::NULL, "".into(), "#battle".into()],
                );
            } else {
                return_base(&s);
            }
        }
        Ok(JsValue::UNDEFINED)
    })
}
#[wasm_bindgen]
pub struct RustApplication {
    state: JsValue,
    listeners: Vec<Listener>,
}
#[wasm_bindgen]
impl RustApplication {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue) -> Self {
        let root = call(&document(), "createElement", &["main".into()]);
        set(&root, "id", &"base-app".into());
        call(&get(&document(), "body"), "prepend", &[root.clone()]);
        let loader = construct(&get(&host, "MawLoader"), &[]);
        let state = object(&[
            ("root", root.clone()),
            ("host", host.clone()),
            ("loader", loader),
            ("inBattle", false.into()),
            ("loading", false.into()),
            ("disposed", false.into()),
        ]);
        let enter_state = state.clone();
        let enter = Closure::<dyn FnMut(JsValue) -> JsValue>::new(move |loadout| {
            enter_battle(enter_state.clone(), loadout).into()
        })
        .into_js_value();
        let base = construct(&get(&host, "BaseScreen"), &[root, enter]);
        set(&state, "base", &base);
        invoke(&get(&host, "mountSoundSettings"), &[]);
        let mut listeners = Vec::new();
        for event in ["pointerdown", "touchend", "click", "keydown"] {
            let audio = get(&host, "gameAudio");
            listeners.push(Listener::passive(&document(), event, move |_| {
                call(&audio, "unlock", &[]);
            }));
        }
        show_base(&state, view_hash());
        let s = state.clone();
        listeners.push(Listener::new(&global(), "hashchange", move |_| {
            if available(&s) {
                show_base(&s, view_hash());
            }
        }));
        let s = state.clone();
        listeners.push(Listener::new(&document(), "visibilitychange", move |_| {
            if available(&s) {
                call(
                    &get(&s, "base"),
                    "setActive",
                    &[(!boolean(&get(&document(), "hidden"))).into()],
                );
            }
        }));
        let s = state.clone();
        listeners.push(Listener::new(&global(), "pagehide", move |_| {
            call(&get(&s, "base"), "setActive", &[false.into()]);
        }));
        let s = state.clone();
        listeners.push(Listener::new(&global(), "pageshow", move |_| {
            if available(&s) {
                call(&get(&s, "base"), "setActive", &[true.into()]);
            }
        }));
        let search = string(&get(&get(&global(), "location"), "search"));
        if boolean(&get(&host,"resumeOnline")) || js_sys::RegExp::new("[?&]cal\\b", "").test(&search) {
            let _ = enter_battle(state.clone(), invoke(&get(&host, "loadLocalLoadout"), &[]));
        }
        Self { state, listeners }
    }
    pub fn dispose(&mut self) {
        if boolean(&get(&self.state, "disposed")) {
            return;
        }
        set(&self.state, "disposed", &true.into());
        let controller = get(&self.state, "controller");
        if present(&controller) {
            call(&controller, "abort", &[]);
        }
        self.listeners.clear();
        call(&get(&self.state, "base"), "dispose", &[]);
        call(&get(&self.state, "loader"), "dispose", &[]);
        call(&get(&self.state, "root"), "remove", &[]);
        for key in ["base", "host", "loader", "controller"] {
            set(&self.state, key, &JsValue::NULL);
        }
    }
}
impl Drop for RustApplication {
    fn drop(&mut self) {
        self.dispose();
    }
}

thread_local! {
    static APPLICATION: RefCell<Option<RustApplication>> = const { RefCell::new(None) };
}
#[wasm_bindgen]
pub fn mount_application(host: JsValue) {
    unmount_application();
    let application = RustApplication::new(host);
    APPLICATION.with(|slot| *slot.borrow_mut() = Some(application));
}
#[wasm_bindgen]
pub fn unmount_application() {
    let previous = APPLICATION.with(|slot| slot.borrow_mut().take());
    drop(previous);
}
