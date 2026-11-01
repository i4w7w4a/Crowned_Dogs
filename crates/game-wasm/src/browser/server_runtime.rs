//! Node host bindings for duel scheduling and atomic operator persistence.
//! No browser APIs are used: clocks, filesystem and cryptographic entropy are injected.
use super::bridge::*;
use js_sys::{Array, Map, Object, Promise};
use std::rc::Rc;
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};

pub(super) fn engine(method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    game_client::dispatch(method, data(&array(args).into()))
        .map(|v| json(&v))
        .map_err(|error| js_sys::Error::new(&error).into())
}
pub(super) fn log(level: &str, message: &str, error: &JsValue) {
    let _ = try_call(
        &get(&global(), "console"),
        level,
        &[message.into(), error.clone()],
    );
}
fn entries(map: &Map) -> Vec<JsValue> {
    values(&call(map, "entries", &[]))
}
fn map_values(map: &Map) -> Vec<JsValue> {
    values(&call(map, "values", &[]))
}
fn assign(target: &JsValue, source: &JsValue) {
    Object::assign(
        target.unchecked_ref::<Object>(),
        source.unchecked_ref::<Object>(),
    );
}

struct Duels {
    // JS handles keep live duel references stable without a Rust mutable borrow across callbacks.
    state: JsValue,
    references: Map,
    timers: Map,
    opts: JsValue,
    clock: JsValue,
    seed: JsValue,
    error: JsValue,
}
impl Duels {
    fn opts(&self) -> JsValue {
        call(&self.opts, "deref", &[])
    }
    fn reference(&self, duel: JsValue) -> JsValue {
        let id = get(&duel, "id");
        let known = self.references.get(&id);
        if present(&known) {
            assign(&known, &duel);
            known
        } else {
            self.references.set(&id, &duel);
            duel
        }
    }
    fn send(self: &Rc<Self>, method: &str, args: JsValue) -> Result<JsValue, JsValue> {
        let seeds = Array::new();
        let now = try_call(&self.clock, "now", &[])?;
        let result = loop {
            let result = engine(
                "server.duels.step",
                &[
                    get(&self.state, "value"),
                    method.into(),
                    args.clone(),
                    now.clone(),
                    seeds.clone().into(),
                    get(&self.opts(), "setting"),
                ],
            )?;
            if !boolean(&get(&result, "needsSeed")) {
                break result;
            }
            seeds.push(&apply(&self.seed, &JsValue::UNDEFINED, &[])?);
        };
        let state = get(&result, "state");
        set(&self.state, "value", &state);
        let duels = get(&state, "duels");
        for id in Object::keys(duels.unchecked_ref::<Object>()).iter() {
            let key = string(&id);
            let duel = self.reference(get(&duels, &key));
            set(&duels, &key, &duel);
        }
        let value = get(&result, "value");
        let returned = if present(&get(&value, "match")) {
            self.reference(value)
        } else {
            value
        };
        self.sync_timers()?;
        for event in values(&get(&result, "events")) {
            set(&event, "duel", &self.reference(get(&event, "duel")));
            self.emit(event);
        }
        // onEvent is allowed to reenter the service, so inspect the current state again.
        let current = get(&get(&self.state, "value"), "duels");
        for entry in entries(&self.references) {
            let id = at(&entry, 0);
            if !present(&get(&current, &string(&id))) {
                self.references.delete(&id);
            }
        }
        let error = get(&result, "error");
        if present(&error) {
            return Err(construct(&self.error, &[error]));
        }
        Ok(if returned.is_null() {
            JsValue::UNDEFINED
        } else {
            returned
        })
    }
    fn sync_timers(self: &Rc<Self>) -> Result<(), JsValue> {
        let duels = get(&get(&self.state, "value"), "duels");
        for entry in entries(&self.timers) {
            let id = at(&entry, 0);
            if !present(&get(&duels, &string(&id))) {
                try_call(
                    &self.clock,
                    "clearTimeout",
                    &[get(&at(&entry, 1), "handle")],
                )?;
                self.timers.delete(&id);
            }
        }
        for duel in Object::values(duels.unchecked_ref::<Object>()).iter() {
            let id = get(&duel, "id");
            let turn = get(&get(&duel, "match"), "turn");
            let deadline = get(&duel, "deadline");
            let previous = self.timers.get(&id);
            if present(&previous)
                && get(&previous, "turn") == turn
                && get(&previous, "deadline") == deadline
            {
                continue;
            }
            if present(&previous) {
                try_call(&self.clock, "clearTimeout", &[get(&previous, "handle")])?;
            }
            let weak = Rc::downgrade(self);
            let timeout_args: JsValue = array(&[id.clone(), turn.clone()]).into();
            let callback = Closure::<dyn FnMut()>::new(move || {
                if let Some(service) = weak.upgrade() {
                    if let Err(error) = service.send("timeout", timeout_args.clone()) {
                        log("error", "duel timeout failed", &error);
                    }
                }
            })
            .into_js_value();
            let delay = (num(&deadline) - num(&try_call(&self.clock, "now", &[])?)).max(0.0);
            let handle = try_call(&self.clock, "setTimeout", &[callback.clone(), delay.into()])?;
            self.timers.set(
                &id,
                &object(&[
                    ("handle", handle),
                    ("turn", turn),
                    ("deadline", deadline),
                    ("callback", callback),
                ]),
            );
        }
        Ok(())
    }
    fn emit(&self, event: JsValue) {
        match try_call(&self.opts(), "onEvent", &[event]) {
            Ok(value) => spawn_local(async move {
                if let Err(error) = JsFuture::from(Promise::resolve(&value)).await {
                    log("error", "duel event failed", &error);
                }
            }),
            Err(error) => log("error", "duel event failed", &error),
        }
    }
    fn close(&self) {
        for timer in map_values(&self.timers) {
            let _ = try_call(&self.clock, "clearTimeout", &[get(&timer, "handle")]);
        }
        self.timers.clear();
    }
}

#[wasm_bindgen]
pub struct RustDuelService {
    inner: Rc<Duels>,
}
#[wasm_bindgen]
impl RustDuelService {
    #[wasm_bindgen(constructor)]
    pub fn new(opts: JsValue, error: JsValue, host: JsValue) -> Result<RustDuelService, JsValue> {
        let clock = get(&opts, "clock");
        let seed = get(&opts, "seed");
        Ok(Self {
            inner: Rc::new(Duels {
                state: object(&[("value", engine("server.duels.new", &[])?)]),
                references: Map::new(),
                timers: Map::new(),
                clock: if present(&clock) {
                    clock
                } else {
                    get(&host, "clock")
                },
                seed: if present(&seed) {
                    seed
                } else {
                    get(&host, "seed")
                },
                opts: construct(&get(&global(), "WeakRef"), &[opts]),
                error,
            }),
        })
    }
    pub fn send(&self, method: &str, args: JsValue) -> Result<JsValue, JsValue> {
        self.inner.send(method, args)
    }
    pub fn close(&self) {
        self.inner.close();
    }
}
impl Drop for RustDuelService {
    fn drop(&mut self) {
        self.inner.close();
    }
}

#[wasm_bindgen]
pub struct RustOperatorRegistry {
    operators: Map,
    opts: JsValue,
    host: JsValue,
}
#[wasm_bindgen]
impl RustOperatorRegistry {
    #[wasm_bindgen(constructor)]
    pub fn new(opts: JsValue, host: JsValue) -> Result<RustOperatorRegistry, JsValue> {
        let registry = Self {
            operators: Map::new(),
            opts,
            host,
        };
        let file = get(&registry.opts, "file");
        if present(&file) && !string(&file).is_empty() {
            match try_call(&registry.host, "readFileSync", &[file, "utf8".into()]) {
                Ok(text) => {
                    let saved = js_sys::JSON::parse(&string(&text))?;
                    for op in values(&saved) {
                        registry.operators.set(&get(&op, "telegramId"), &op);
                    }
                }
                Err(error) if string(&get(&error, "code")) == "ENOENT" => (),
                Err(error) => return Err(error),
            }
        }
        Ok(registry)
    }
    #[wasm_bindgen(js_name = byTelegram)]
    pub fn by_telegram(&self, telegram_id: &str) -> JsValue {
        self.operators.get(&telegram_id.into())
    }
    pub fn sign(&self, telegram_id: &str, name: &str) -> Result<JsValue, JsValue> {
        let known = self.by_telegram(telegram_id);
        if present(&known) {
            return Ok(known);
        }
        let custom_now = get(&self.opts, "now");
        let now = if present(&custom_now) {
            apply(&custom_now, &JsValue::UNDEFINED, &[])?
        } else {
            js_sys::Date::now().into()
        };
        let list = array(&map_values(&self.operators));
        let result = engine(
            "server.operators.sign",
            &[list.into(), telegram_id.into(), name.into(), now],
        )?;
        let operator = get(&result, "operator");
        self.operators.set(&get(&operator, "telegramId"), &operator);
        self.save()?;
        Ok(operator)
    }
}
impl RustOperatorRegistry {
    fn save(&self) -> Result<(), JsValue> {
        let file = string(&get(&self.opts, "file"));
        if file.is_empty() {
            return Ok(());
        }
        let directory = try_call(&self.host, "dirname", &[file.clone().into()])?;
        try_call(
            &self.host,
            "mkdirSync",
            &[directory, object(&[("recursive", true.into())])],
        )?;
        let temporary: JsValue = format!("{file}.tmp").into();
        let serialized = try_call(
            &get(&global(), "JSON"),
            "stringify",
            &[
                array(&map_values(&self.operators)).into(),
                JsValue::NULL,
                2.into(),
            ],
        )?;
        try_call(
            &self.host,
            "writeFileSync",
            &[temporary.clone(), serialized],
        )?;
        try_call(&self.host, "renameSync", &[temporary, file.into()])?;
        Ok(())
    }
}
