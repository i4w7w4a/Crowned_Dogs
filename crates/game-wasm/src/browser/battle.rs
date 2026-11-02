//! Battle screen controller. DOM and Three.js are foreign handles; decisions,
//! event routing, timing, loading, and screen state are owned here.
use super::bridge::*;
use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{prelude::*, JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::{future_to_promise, JsFuture};

const ZONES: [&str; 4] = ["head", "body", "groin", "legs"];
const QUALITY_IDS: [&str; 5] = ["minimal", "low", "medium", "high", "ultra"];

fn id(name: &str) -> JsValue {
    call(&document(), "getElementById", &[name.into()])
}
fn query(el: &JsValue, selector: &str) -> JsValue {
    call(el, "querySelector", &[selector.into()])
}
fn all(selector: &str) -> Vec<JsValue> {
    values(&call(&document(), "querySelectorAll", &[selector.into()]))
}
fn style(el: &JsValue, key: &str, value: &str) {
    set(&get(el, "style"), key, &value.into());
}
fn text(el: &JsValue, value: &str) {
    set(el, "textContent", &value.into());
}
fn html(el: &JsValue, value: &str) {
    set(el, "innerHTML", &value.into());
}
fn attr(el: &JsValue, key: &str, value: &str) {
    call(el, "setAttribute", &[key.into(), value.into()]);
}
fn toggle(el: &JsValue, class: &str, on: bool) {
    call(&get(el, "classList"), "toggle", &[class.into(), on.into()]);
}
fn hide(el: &JsValue, on: bool) {
    set(el, "hidden", &on.into());
}
fn flag(s: &JsValue, name: &str) -> bool {
    boolean(&get(s, name))
}
fn number(s: &JsValue, name: &str) -> f64 {
    num(&get(s, name))
}
fn word(s: &JsValue, name: &str) -> String {
    string(&get(s, name))
}
fn host(s: &JsValue, key: &str, args: &[JsValue]) -> JsValue {
    call(&get(s, "host"), key, args)
}
fn audio(s: &JsValue, method: &str, args: &[JsValue]) {
    call(&get(s, "audio"), method, args);
}
fn arena(s: &JsValue, method: &str, args: &[JsValue]) -> JsValue {
    let a = get(s, "arena");
    if present(&a) {
        call(&a, method, args)
    } else {
        JsValue::UNDEFINED
    }
}
fn core(method: &str, args: &[JsValue]) -> JsValue {
    let input = js_sys::JSON::stringify(&array(args))
        .unwrap_throw()
        .as_string()
        .unwrap();
    js_sys::JSON::parse(&crate::dispatch(method, &input).unwrap_throw()).unwrap_throw()
}
fn listen(target: &JsValue, event: &str, action: impl FnMut(JsValue) + 'static) {
    let callback = Closure::<dyn FnMut(JsValue)>::new(action).into_js_value();
    call(target, "addEventListener", &[event.into(), callback]);
}
fn button(s: &JsValue, name: &str, action: fn(&JsValue)) {
    let state = s.clone();
    listen(&id(name), "click", move |_| action(&state));
}
fn once(ms: f64, action: impl FnOnce() + 'static) -> JsValue {
    // A cancelled timeout must be reclaimable without invoking its callback.
    let mut action = Some(action);
    let callback = Closure::<dyn FnMut()>::new(move || {
        if let Some(action) = action.take() {
            action();
        }
    })
    .into_js_value();
    call(&global(), "setTimeout", &[callback, ms.into()])
}
fn after(s: &JsValue, seconds: f64, action: impl FnOnce(&JsValue) + 'static) {
    let state = s.clone();
    let generation = get(s, "fightNo");
    once(seconds * 1000., move || {
        if get(&state, "fightNo") == generation {
            action(&state);
        }
    });
}
fn duel(s: &JsValue) -> JsValue {
    get(s, "duel")
}
fn match_(s: &JsValue) -> JsValue {
    get(&duel(s), "match")
}
fn draft(s: &JsValue) -> JsValue {
    get(&duel(s), "draft")
}
fn over(s: &JsValue) -> bool {
    flag(&duel(s), "over")
}
fn store_get(s: &JsValue, name: &str) -> JsValue {
    call(&get(&get(s, "host"), "store"), "get", &[name.into()])
}
fn store_set(s: &JsValue, name: &str, value: &str) {
    call(
        &get(&get(s, "host"), "store"),
        "set",
        &[name.into(), value.into()],
    );
}
fn pct(value: &JsValue) -> String {
    if !present(value) {
        "—".into()
    } else {
        format!("{}%", (num(value) * 100.).round())
    }
}
fn zone_name(zone: &str) -> String {
    string(&core("words.zoneName", &[zone.into()]))
}
fn capital(value: &str) -> String {
    let mut c = value.chars();
    c.next()
        .map(|first| first.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}
fn steps(n: f64) -> String {
    string(&core("words.steps", &[n.into()]))
}

#[wasm_bindgen]
pub struct RustBattle {
    state: JsValue,
}

#[wasm_bindgen]
impl RustBattle {
    #[wasm_bindgen(constructor)]
    pub fn new(bindings: JsValue) -> Self {
        console_error_panic_hook::set_once();
        let words = core("client.data", &[]);
        let state = object(&[
            ("host", bindings.clone()),
            ("audio", get(&bindings, "gameAudio")),
            ("frames", get(&bindings, "frames")),
            ("frame", at(&get(&bindings, "frames"), 0)),
            ("words", words),
            ("weapon", "rifle".into()),
            ("veteran", false.into()),
            ("fitted", JsValue::NULL),
            ("active", false.into()),
            ("busy", false.into()),
            ("fightNo", 0.into()),
            ("seconds", 0.into()),
            ("quality", "low".into()),
            ("byHand", false.into()),
            ("hp", array(&[0.into(), 0.into()]).into()),
            ("turn", 1.into()),
            ("ringR", 7.2.into()),
            ("ctlTop", 1e9.into()),
            (
                "geo",
                object(&[
                    ("x", 0.into()),
                    ("y", 0.into()),
                    ("w", 1.into()),
                    ("h", 1.into()),
                ]),
            ),
            ("ringEls", object(&[])),
            ("ringAt", object(&[])),
        ]);
        let saved = store_get(&state, "cd-weapon");
        if values(&get(&bindings, "weaponIds")).contains(&saved) {
            set(&state, "weapon", &saved);
        }
        let reduced = try_call(
            &global(),
            "matchMedia",
            &["(prefers-reduced-motion: reduce)".into()],
        )
        .ok()
        .map(|v| flag(&v, "matches"))
        .unwrap_or(false);
        set(&state, "reduceMotion", &reduced.into());
        let selected = live_frame(&state, None);
        if present(&selected) {
            set(&state, "frame", &selected);
        }
        set(&state, "duel", &new_duel(&state));
        let log = construct(
            &get(&bindings, "FightLog"),
            &[object(&[
                ("list", id("logList")),
                ("ticker", id("ticker")),
                ("button", id("logBtn")),
                ("sheet", id("logSheet")),
            ])],
        );
        set(&state, "log", &log);
        host(&state, "liquidGlass", &[]);
        create_rings(&state);
        wire(&state);
        let camera = if string(&store_get(&state, "cd-camera")) == "back"
            && word(&get(&global(), "location"), "hash") != "#fp"
        {
            "back"
        } else {
            "fp"
        };
        set(&state, "camera", &camera.into());
        let ctor = get(&bindings, "Arena");
        if let Ok(a) = Reflect::construct(ctor.unchecked_ref::<Function>(), &array(&[id("gl")])) {
            let a: JsValue = a.into();
            set(&state, "arena", &a);
            let s = state.clone();
            set(
                &a,
                "onFrame",
                &Closure::<dyn FnMut()>::new(move || {
                    place_rings(&s);
                    thunder(&s);
                })
                .into_js_value(),
            );
            arena(&state, "setFirstPerson", &[(camera == "fp").into()]);
            let q = host(&state, "startQuality", &[get(&a, "gl")]);
            set(&state, "quality", &get(&q, "id"));
            set(&state, "byHand", &get(&q, "picked"));
            arena(
                &state,
                "setQuality",
                &[quality(&state, &word(&state, "quality"))],
            );
        } else {
            text(&id("status"), "3D не запустилось на этом устройстве");
        }
        set_frame(&state, &get(&state, "frame"));
        install_clock(&state);
        install_speed_watch(&state);
        let diag = object(&[
            ("audio", get(&state, "audio")),
            ("arena", get(&state, "arena")),
        ]);
        let s = state.clone();
        let getter = Closure::<dyn FnMut() -> JsValue>::new(move || duel(&s)).into_js_value();
        Reflect::define_property(
            diag.unchecked_ref::<js_sys::Object>(),
            &"duel".into(),
            object(&[("get", getter)]).unchecked_ref::<js_sys::Object>(),
        )
        .unwrap_throw();
        set(&global(), "__cd", &diag);
        Self { state }
    }

    pub fn start(
        &self,
        kit: JsValue,
        on_exit: JsValue,
        before_enter: JsValue,
        signal: JsValue,
    ) -> Promise {
        let s = self.state.clone();
        future_to_promise(async move {
            let selected = call(&global(), "structuredClone", &[kit]);
            let camera = if string(&store_get(&s, "cd-camera")) == "back" {
                "back"
            } else {
                "fp"
            };
            set_camera(&s, camera);
            let q = string(&store_get(&s, "cd-quality"));
            if present(&get(&s, "arena"))
                && QUALITY_IDS.contains(&q.as_str())
                && q != word(&s, "quality")
            {
                set_quality(&s, &q, true);
            }
            if present(&core("validateLoadout", &[selected.clone()])) {
                return Err(js_sys::Error::new("Некорректный комплект").into());
            }
            abort_check(&signal)?;
            JsFuture::from(prepare_arena(&s)).await?;
            abort_check(&signal)?;
            // The fight is drawn under the loader first; the loader lifts and the clock starts after that.
            set(&s, "fitted", &selected);
            set(&s, "exit", &on_exit);
            set(&s, "active", &true.into());
            hide(&id("app"), false);
            restart(&s, false);
            stop_timer(&s);
            layout(&s);
            arena(&s, "resume", &[]);
            let fight = number(&s, "fightNo");
            settle(&get(&s, "arena"), "drawn", 5000.).await;
            settle(&get(&s, "arena"), "drawn", 5000.).await;
            if present(&before_enter) {
                JsFuture::from(Promise::resolve(&apply(
                    &before_enter,
                    &JsValue::UNDEFINED,
                    &[],
                )?))
                .await
                .inspect_err(|_| undo_enter(&s))?;
            }
            abort_check(&signal).inspect_err(|_| undo_enter(&s))?;
            if flag(&s, "active") && number(&s, "fightNo") == fight {
                start_timer(&s);
            }
            audio(&s, "unlock", &[]);
            let state = s.clone();
            let timeout = once(1500., move || watch_speed(&state));
            set(&s, "speedDelay", &timeout);
            Ok(JsValue::UNDEFINED)
        })
    }
}

/// Rolls back a fight that was drawn under the loader but never shown.
fn undo_enter(s: &JsValue) {
    set(s, "active", &false.into());
    stop_timer(s);
    arena(s, "pause", &[]);
    audio(s, "finish", &[JsValue::NULL]);
    hide(&id("app"), true);
}
/// Awaits an optional arena promise (`warm`, `drawn`), giving up after `ms`.
async fn settle(arena: &JsValue, method: &str, ms: f64) {
    if !get(arena, method).is_function() {
        return;
    }
    let Ok(p) = try_call(arena, method, &[]) else {
        return;
    };
    let timeout = Promise::new(&mut |resolve, _| {
        call(&global(), "setTimeout", &[resolve.into(), ms.into()]);
    });
    let _ = JsFuture::from(Promise::race(&array(&[
        Promise::resolve(&p).into(),
        timeout.into(),
    ])))
    .await;
}
fn abort_check(signal: &JsValue) -> Result<(), JsValue> {
    if present(signal) {
        try_call(signal, "throwIfAborted", &[])?;
    }
    Ok(())
}
/// The operator's synchronisation (level, stats, mastery) as stored on this device.
fn sync_profile(s: &JsValue) -> JsValue {
    let saved = store_get(s, "cd-sync")
        .as_string()
        .and_then(|raw| js_sys::JSON::parse(&raw).ok())
        .unwrap_or(JsValue::NULL);
    core("syncClean", &[saved])
}
fn new_duel(s: &JsValue) -> JsValue {
    let ids = get(&get(s, "host"), "weaponIds");
    let foe = at(
        &ids,
        (js_sys::Math::random() * number(&ids, "length")) as usize,
    );
    // Nothing spent yet plays the plain rules; the bot mirrors what the operator has.
    let profile = sync_profile(s);
    let skills = if core("syncPlays", &[profile.clone()]).as_bool() == Some(true) {
        profile.clone()
    } else {
        JsValue::NULL
    };
    let bot_skills = core("syncBot", &[profile, foe.clone()]);
    let frame = get(s, "frame");
    let fitted = get(s, "fitted");
    let kit = if present(&fitted) {
        fitted
    } else if flag(s, "veteran") {
        get(&get(&get(s, "host"), "veteranKits"), &word(s, "weapon"))
    } else {
        JsValue::UNDEFINED
    };
    // The guns working on this map, or ranging on it for the next shift, bring far more shells.
    let clock = get(&frame, "clock");
    let map = word(&frame, "mapId");
    let shelled = word(&clock, "shelled") == map || word(&clock, "nextShelled") == map;
    construct(
        &get(&get(s, "host"), "LocalDuel"),
        &[object(&[
            ("mapId", get(&frame, "mapId")),
            ("weather", get(&frame, "weather")),
            ("time", get(&frame, "time")),
            ("events", true.into()),
            ("shelled", shelled.into()),
            (
                "player",
                object(&[
                    ("name", "Тень".into()),
                    ("tag", "K-417-093".into()),
                    ("weapon", get(s, "weapon")),
                    ("kit", kit),
                    ("skills", skills),
                ]),
            ),
            (
                "bot",
                object(&[
                    ("name", "Искра".into()),
                    ("weapon", foe),
                    ("skills", bot_skills),
                ]),
            ),
        ])],
    )
}
fn create_rings(s: &JsValue) {
    for zone in ZONES {
        let el = call(&document(), "createElement", &["button".into()]);
        set(&el, "type", &"button".into());
        set(&el, "className", &"ring hint".into());
        attr(&el, "aria-label", &format!("Атака: {}", zone_name(zone)));
        html(
            &el,
            "<svg viewBox=\"0 0 40 40\" aria-hidden=\"true\"><circle class=\"rc\" cx=\"20\" cy=\"20\" r=\"7.2\"/></svg>",
        );
        let state = s.clone();
        listen(&el, "click", move |_| press(&state, "zone", Some(zone)));
        call(&id("rings"), "appendChild", &[el.clone()]);
        set(&get(s, "ringEls"), zone, &el);
    }
    let tag = call(&document(), "createElement", &["div".into()]);
    set(&tag, "className", &"tag".into());
    hide(&tag, true);
    call(&id("rings"), "appendChild", &[tag.clone()]);
    set(s, "tag", &tag);
    let state = s.clone();
    let cb = Closure::<dyn FnMut()>::new(move || measure_ctl(&state)).into_js_value();
    let observer = construct(&get(&global(), "ResizeObserver"), &[cb]);
    call(
        &observer,
        "observe",
        &[query(&query(&document(), ".panel"), ".ctl")],
    );
    set(s, "observer", &observer);
}
fn measure_ctl(s: &JsValue) {
    let panel = query(&document(), ".panel");
    let y = number(&panel, "offsetTop") + number(&query(&panel, ".ctl"), "offsetTop")
        - number(&id("scene"), "offsetTop");
    set(s, "ctlTop", &y.into());
}
fn point(s: &JsValue, zone: &str) -> (f64, f64) {
    let i = ZONES.iter().position(|z| *z == zone).unwrap_or(1);
    let (u, v) = if present(&get(s, "arena")) {
        let p = arena(s, "zoneOnScreen", &[zone.into()]);
        (num(&at(&p, 0)), num(&at(&p, 1)))
    } else {
        (0.645, [0.26, 0.34, 0.42, 0.49][i])
    };
    let geo = get(s, "geo");
    (
        number(&geo, "x") + u * number(&geo, "w"),
        number(&geo, "y") + v * number(&geo, "h") + [-0.5, -1.4, 0., 1.4][i] * number(s, "ringR"),
    )
}
fn place_rings(s: &JsValue) {
    let points: Vec<JsValue> = ZONES
        .iter()
        .map(|z| {
            let (x, y) = point(s, z);
            array(&[x.into(), y.into()]).into()
        })
        .collect();
    let placed = core(
        "ui.reticles",
        &[array(&points).into(), get(s, "ringR"), get(s, "ctlTop")],
    );
    for (i, zone) in ZONES.iter().enumerate() {
        let p = at(&placed, i);
        let x = num(&at(&p, 0));
        let y = num(&at(&p, 1));
        let height = format!("{}px", num(&at(&p, 2)));
        set(
            &get(s, "ringAt"),
            zone,
            &array(&[x.into(), y.into()]).into(),
        );
        let el = get(&get(s, "ringEls"), zone);
        if word(&get(&el, "style"), "height") != height {
            style(&el, "height", &height);
            style(&el, "marginTop", &format!("calc({height} / -2)"));
        }
        style(&el, "transform", &format!("translate({x:.1}px, {y:.1}px)"));
    }
    let tag = get(s, "tag");
    let zone = word(&draft(s), "zone");
    if !flag(&tag, "hidden") && !zone.is_empty() {
        let p = get(&get(s, "ringAt"), &zone);
        let x = num(&at(&p, 0));
        let y = num(&at(&p, 1));
        let w = number(&tag, "offsetWidth");
        let gap = number(s, "ringR") + 16.;
        let sc = call(&id("scene"), "getBoundingClientRect", &[]);
        let col = call(&id("moves"), "getBoundingClientRect", &[]);
        let beside = y + 14. > number(&col, "top") - number(&sc, "top")
            && y - 14. < number(&col, "bottom") - number(&sc, "top");
        let edge = if beside {
            number(&col, "left") - number(&sc, "left") - 6.
        } else {
            number(&id("scene"), "clientWidth") - 8.
        };
        let right = x + gap + w <= edge;
        toggle(&tag, "flip", !right);
        style(
            &tag,
            "transform",
            &format!(
                "translate({:.1}px, {y:.1}px) translateY(-50%)",
                if right { x + gap } else { x - gap - w }
            ),
        );
    }
}
fn layout(s: &JsValue) {
    let w = number(&id("scene"), "clientWidth");
    if !flag(s, "active") || w == 0. {
        return;
    }
    let view = core(
        "ui.layout",
        &[w.into(), get(&id("scene"), "clientHeight"), get(s, "frame")],
    );
    set(s, "geo", &view);
    for el in [id("bg"), id("gl")] {
        for (css, key) in [("width", "w"), ("height", "h"), ("left", "x"), ("top", "y")] {
            style(&el, css, &format!("{}px", number(&view, key)));
        }
    }
    arena(
        s,
        "resize",
        &[get(&view, "w"), get(&view, "h"), get(&view, "viewport")],
    );
    set(s, "ringR", &get(&view, "radius"));
    for z in ZONES {
        attr(
            &query(&get(&get(s, "ringEls"), z), "circle"),
            "r",
            &format!("{:.2}", number(s, "ringR")),
        );
    }
    measure_ctl(s);
    place_rings(s);
}
fn can_tap(s: &JsValue) -> bool {
    flag(s, "active") && !flag(s, "busy") && !over(s)
}
fn press(s: &JsValue, kind: &str, zone: Option<&str>) {
    if !can_tap(s) {
        return;
    }
    let command = object(&[("type", kind.into())]);
    if let Some(z) = zone {
        set(&command, "zone", &z.into());
    }
    let err = call(&duel(s), "press", &[command]);
    tap_result(s, &err);
}
fn tap_result(s: &JsValue, error: &JsValue) {
    if present(error) {
        call(&get(s, "log"), "note", &[error.clone()]);
    }
    render(s);
}
fn move_to(s: &JsValue, distance: f64) {
    if !can_tap(s) {
        return;
    }
    let mut err = call(&duel(s), "setMove", &[distance.into()]);
    if present(&err) && word(&draft(s), "kind") == "knife" {
        call(&duel(s), "press", &[object(&[("type", "knife".into())])]);
        err = call(&duel(s), "setMove", &[distance.into()]);
    }
    tap_result(s, &err);
}
fn pressed(el: &JsValue, on: bool) {
    toggle(el, "on", on);
    attr(el, "aria-pressed", if on { "true" } else { "false" });
}
fn render(s: &JsValue) {
    arena(s, "setConditions", &[match_(s)]);
    if !flag(s, "busy") && get(&get(s, "arena"), "hazardMark").is_function() {
        arena(s, "hazardMark", &[match_(s)]);
    }
    let m = match_(s);
    let d = draft(s);
    let me = at(&get(&m, "fighters"), 0);
    let controls = core("ui.battleControls", &[m.clone(), d.clone(), get(s, "busy")]);
    let lock = flag(&controls, "lock");
    let aimed = flag(&controls, "aimed");
    let busy = flag(s, "busy");
    toggle(&id("rings"), "off", busy);
    toggle(&query(&document(), ".panel"), "playing", busy);
    toggle(&id("rings"), "idle", !aimed);
    toggle(
        &id("rings"),
        "gone",
        over(s) || (present(&get(s, "arena")) && !flag(&get(s, "arena"), "ready")),
    );
    let zone = word(&d, "zone");
    let kind = word(&d, "kind");
    for z in ZONES {
        let el = get(&get(s, "ringEls"), z);
        toggle(&el, "sel", aimed && zone == z);
        toggle(
            &el,
            "hint",
            aimed && zone.is_empty() && number(&m, "turn") == 1.,
        );
    }
    let tag = get(s, "tag");
    let chance = get(&controls, "chance");
    hide(&tag, !aimed || zone.is_empty() || !present(&chance) || lock);
    if !flag(&tag, "hidden") {
        text(
            &tag,
            &format!(
                "{}{} · {}",
                if kind == "knife" { "Нож · " } else { "" },
                capital(&zone_name(&zone)),
                pct(&chance)
            ),
        );
    }
    place_rings(s);
    let defend = values(&get(&d, "defend"));
    for b in all(".dz") {
        pressed(&b, defend.contains(&get(&get(&b, "dataset"), "z")));
        set(&b, "disabled", &lock.into());
    }
    let need = number(&controls, "need");
    let n = defend.len();
    text(&id("pips"), &format!("{n}/{need}"));
    toggle(&id("pips"), "done", n as f64 >= need);
    attr(&id("pips"), "aria-label", &format!("Выбрано {n} из {need}"));
    for b in all(".mv") {
        let mv = word(&get(&b, "dataset"), "m").parse::<f64>().unwrap_or(0.);
        pressed(&b, mv == number(&d, "move") && kind != "flee");
        set(
            &b,
            "disabled",
            &(lock || number(&m, "distance") - mv < 1.).into(),
        );
    }
    // A hazard warned for this turn: the chip under the health bars and an amber mark on
    // every move that ends inside it.
    let warning = core("words.hazardWarning", &[m.clone()]);
    let risky = if present(&warning) {
        values(&get(&warning, "risky"))
    } else {
        vec![]
    };
    for b in all(".mv") {
        let mv = word(&get(&b, "dataset"), "m").parse::<f64>().unwrap_or(0.);
        toggle(&b, "risk", !lock && risky.iter().any(|r| r.as_f64() == Some(mv)));
    }
    // While the turn plays out the match already holds the next warning: it waits for the next turn.
    hide(&id("hazardNote"), !present(&warning) || over(s) || flag(s, "busy"));
    if present(&warning) {
        let icon = match word(&warning, "kind").as_str() {
            "drone" => "i-drone",
            "shell" | "collapse" => "i-blast",
            "toxic" => "i-dust",
            _ => "i-storm",
        };
        html(&id("hazardIcon"), &string(&host(s, "icon", &[icon.into()])));
        text(&id("hazardText"), &word(&warning, "text"));
    }
    let stunned = flag(&warning, "stunned") && !lock;
    let dist = number(&controls, "dist");
    text(&id("distLabel"), &format!("{dist} {}", steps(dist)));
    hide(&id("distNote"), !flag(&controls, "noShot"));
    text(
        &id("distNote"),
        if kind == "swap" {
            "без выстрела"
        } else {
            "без атаки"
        },
    );
    let knife_ok = flag(&controls, "knifeOk");
    let nade_ok = flag(&controls, "nadeOk");
    for (name, k, disabled) in [
        ("knifeBtn", "knife", !knife_ok || lock),
        ("nadeBtn", "grenade", !nade_ok || lock),
        ("swapBtn", "swap", lock),
    ] {
        let b = id(name);
        pressed(&b, kind == k);
        set(&b, "disabled", &disabled.into());
    }
    let step_icon = string(&host(s, "icon", &["i-steps".into()]));
    html(
        &id("knifeSub"),
        &if knife_ok {
            pct(&get(&controls, "knifeChance"))
        } else {
            format!("≤2{step_icon}")
        },
    );
    let grenades = number(&me, "grenades");
    let g = get(&controls, "grenade");
    html(
        &id("nadeSub"),
        &if grenades == 0. {
            "×0".into()
        } else if nade_ok {
            format!("×{grenades}")
        } else {
            format!(
                "{}–{}{step_icon}",
                number(&g, "minDistance"),
                number(&g, "maxDistance")
            )
        },
    );
    let spare = get(&controls, "spare");
    hide(&id("swapBtn"), !present(&spare));
    if present(&spare) {
        let name = word(&get(&get(s, "words"), "GUN_ACC"), &string(&spare));
        text(&id("swapText"), &format!("Взять {name}"));
        attr(
            &id("swapBtn"),
            "aria-label",
            &format!("Взять {name}, в этот ход без выстрела"),
        );
    }
    let message = get(&controls, "message");
    set(&id("goBtn"), "disabled", &(present(&message) && !stunned).into());
    text(
        &id("goText"),
        &if stunned {
            "Оглушён · ход без тебя".into()
        } else if present(&message) {
            string(&message)
        } else if kind == "flee" {
            "Уйти из боя".into()
        } else {
            "Подтвердить ход".into()
        },
    );
    draw_hp(s, 0);
    draw_hp(s, 1);
    text(
        &id("turn"),
        &format!(
            "Ход {} / {}",
            number(s, "turn"),
            num(&core("turnLimit", &[m]))
        ),
    );
    draw_timer(s);
}
fn draw_hp(s: &JsValue, side: usize) {
    let suffix = if side == 0 { "Me" } else { "En" };
    let fighter = at(&get(&match_(s), "fighters"), side);
    let view = core(
        "ui.health",
        &[at(&get(s, "hp"), side), get(&fighter, "maxHp")],
    );
    text(&id(&format!("hp{suffix}")), &word(&view, "text"));
    for prefix in ["bar", "ghost"] {
        style(
            &id(&format!("{prefix}{suffix}")),
            "width",
            &word(&view, "width"),
        );
    }
    toggle(&id(&format!("side{suffix}")), "low", flag(&view, "low"));
}
fn draw_header(s: &JsValue) {
    let c = get(&match_(s), "config");
    let words = get(s, "words");
    // Each map names its own weather: smoke in the hall, mist off the water, fumes over the ponds.
    let weather = string(&core(
        "client.world.weatherName",
        &[get(&c, "mapId"), get(&c, "weather")],
    ));
    let mut chars = weather.chars();
    let weather = chars
        .next()
        .map(|f| f.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_else(|| word(&get(&words, "WEATHER_WORD"), &word(&c, "weather")));
    text(
        &id("condText"),
        &format!(
            "{} · {weather}",
            word(&get(&words, "TIME_WORD"), &word(&c, "time")),
        ),
    );
    text(&id("mapTitle"), &word(&get(s, "frame"), "name"));
    let fighters = get(&duel(s), "fighters");
    text(&id("nameMe"), &word(&at(&fighters, 0), "name"));
    text(&id("nameEn"), &word(&at(&fighters, 1), "name"));
    text(&id("recTag"), &word(&at(&fighters, 0), "tag"));
}
fn draw_timer(s: &JsValue) {
    let t = number(s, "seconds").max(0.) as u32;
    text(&id("timer"), &format!("{:02}:{:02}", t / 60, t % 60));
    toggle(&id("timer"), "low", t <= 10 && !flag(s, "busy") && !over(s));
    toggle(&id("timer"), "idle", flag(s, "busy") || over(s));
}
fn install_clock(s: &JsValue) {
    let state = s.clone();
    let callback = Closure::<dyn FnMut()>::new(move || {
        let s = &state;
        if flag(s, "busy") || over(s) || !flag(&id("infoVeil"), "hidden") {
            return;
        }
        let t = number(s, "seconds") - 1.;
        set(s, "seconds", &t.into());
        draw_timer(s);
        if t > 0. && t <= 5. {
            audio(
                s,
                "play",
                &[if t <= 3. { "ui.tick.urgent" } else { "ui.tick" }.into()],
            );
        }
        if t <= 0. {
            play_turn(s, call(&duel(s), "timeout", &[]));
        }
    })
    .into_js_value();
    set(s, "clockCallback", &callback);
}
fn stop_timer(s: &JsValue) {
    call(&global(), "clearInterval", &[get(s, "timer")]);
    set(s, "timer", &JsValue::NULL);
}
fn start_timer(s: &JsValue) {
    stop_timer(s);
    if !flag(s, "active") {
        return;
    }
    set(s, "seconds", &core("turnSeconds", &[match_(s)]));
    draw_timer(s);
    set(
        s,
        "timer",
        &call(
            &global(),
            "setInterval",
            &[get(s, "clockCallback"), 1000.into()],
        ),
    );
}
fn end_turn(s: &JsValue) {
    if !can_tap(s) {
        return;
    }
    // Stunned by lightning: the turn plays without an order, which is not going missing.
    if num(&get(&at(&get(&match_(s), "fighters"), 0), "stunned")) > 0. {
        play_turn(s, call(&duel(s), "timeout", &[]));
        return;
    }
    let r = call(&duel(s), "submit", &[]);
    if r.is_string() {
        call(&get(s, "log"), "note", &[r]);
    } else {
        play_turn(s, r);
    }
}
fn play_turn(s: &JsValue, r: JsValue) {
    stop_timer(s);
    set(s, "busy", &true.into());
    audio(s, "play", &["ui.confirm".into()]);
    // Where each fighter stands for the ear; an arena without it keeps the fixed stereo places.
    let ears = if get(&get(s, "arena"), "ears").is_function() {
        arena(s, "ears", &[])
    } else {
        JsValue::UNDEFINED
    };
    audio(s, "turn", &[r.clone(), match_(s), ears]);
    arm_both(s);
    let moves = get(&r, "moves");
    arena(
        s,
        "move",
        &[get(&r, "distanceAfter"), at(&moves, 0), at(&moves, 1)],
    );
    let timing = get(&get(s, "host"), "timing");
    for side in 0..2 {
        let a = at(&get(&r, "attacks"), side);
        if !present(&a) {
            continue;
        }
        let t = number(
            &timing,
            if side == 0 {
                "ourAttack"
            } else {
                "enemyAttack"
            },
        );
        arena(
            s,
            "attack",
            &[
                if side == 0 { "me" } else { "enemy" }.into(),
                get(&a, "weapon"),
                (t + if word(&a, "weapon") == "knife" {
                    number(&timing, "knifeLands")
                } else {
                    0.
                })
                .into(),
            ],
        );
    }
    if get(&get(s, "arena"), "hazardMark").is_function() {
        arena(s, "hazardMark", &[JsValue::NULL]);
    }
    render(s);
    let report = r.clone();
    after(s, number(&timing, "ourAttack"), move |s| {
        strike(s, &report, 0)
    });
    let hazard = get(&r, "hazard");
    let kind = word(&hazard, "kind");
    // The living world strikes after both shots: the line goes into the log once it has landed.
    let lands = present(&hazard) && (kind != "blackout" || present(&get(&hazard, "toxic")));
    let report = r.clone();
    after(s, number(&timing, "enemyAttack"), move |s| {
        strike(s, &report, 1);
        if !lands {
            log_turn(s, report);
        }
    });
    if !lands {
        after(s, 2.5, next_turn);
        return;
    }
    if kind == "shell" {
        // the far guns, then the shell's own whine comes down and ends in the blast
        sound(s, "artillery.far", 0.4, -2.);
        let whine = HAZARD_AT + SHELL_FALL - SHELL_WHINE;
        sound(s, "artillery.incoming", whine, -2.);
    }
    after(s, HAZARD_AT, move |s| {
        let impact = if get(&get(s, "arena"), "hazardStrike").is_function() {
            num(&arena(s, "hazardStrike", &[r.clone()])).max(0.)
        } else {
            0.
        };
        match word(&get(&r, "hazard"), "kind").as_str() {
            "lightning" => sound(s, "thunder", impact, 3.),
            "shell" => sound(s, "grenade.blast", impact, 2.),
            "drone" => {
                sound(s, "dash.whoosh", (impact - 0.3).max(0.), 0.);
                sound(s, "grenade.blast", impact, 0.);
            }
            "collapse" => {
                sound(s, "body.fall", impact, 2.);
                sound(s, "grenade.blast", impact, -8.);
            }
            _ => {}
        }
        after(s, impact, move |s| {
            hazard_lands(s, &r);
            log_turn(s, r);
            after(s, 0.9_f64.max(2.5 - HAZARD_AT - impact), next_turn);
        });
    });
}
/// When the warned hazard comes down, after both shots.
const HAZARD_AT: f64 = 1.75;
/// How long a shell falls once it is launched (`blast_in(0.5, ..)` in arena.rs `strike`).
const SHELL_FALL: f64 = 0.5;
/// Length of the artillery-incoming files: each ends where the shell lands.
const SHELL_WHINE: f64 = 1.8;
fn sound(s: &JsValue, id: &str, delay: f64, gain_db: f64) {
    audio(
        s,
        "play",
        &[
            id.into(),
            object(&[("delay", delay.into()), ("gainDb", gain_db.into())]),
        ],
    );
}
/// The hazard's damage, the stun and the poison show where they land, after the shots' numbers.
fn hazard_lands(s: &JsValue, r: &JsValue) {
    let h = get(r, "hazard");
    let stage = id("stage");
    let mine = (
        number(&stage, "clientWidth") * 0.3,
        number(&stage, "clientHeight") * 0.6,
    );
    let toxic = number(&h, "toxic");
    for side in 0..2usize {
        let hp = at(&get(r, "hp"), side);
        Reflect::set(&get(s, "hp"), &(side as u32).into(), &hp).unwrap_throw();
        let (x, y) = if side == 1 { point(s, "body") } else { mine };
        let hit = values(&get(&h, "hits"))
            .into_iter()
            .find(|hh| number(hh, "side") as usize == side);
        if let Some(hh) = &hit {
            let stun = if flag(hh, "stun") { " · оглушён" } else { "" };
            // above the shots' numbers, which are still floating up
            floating(
                x + if side == 1 { 36. } else { 0. },
                y - 90.,
                "dmg",
                &format!("−{}", number(hh, "damage")),
                &format!("{}{stun}", word(&h, "name")),
            );
            let blast = word(&h, "kind") != "lightning";
            arena(
                s,
                "hit",
                &[
                    if side == 1 { "enemy" } else { "me" }.into(),
                    "body".into(),
                    if blast { "grenade" } else { "hit" }.into(),
                ],
            );
            if side == 0 {
                fire(&id("flashHurt"), "on");
            }
        }
        if toxic > 0. {
            floating(
                x + if side == 1 { 36. } else { 0. },
                y + if hit.is_some() { -40. } else { -90. },
                "dmg",
                &format!("−{toxic}"),
                "отрава",
            );
        }
        if num(&hp) <= 0. {
            arena(s, "fall", &[if side == 1 { "enemy" } else { "me" }.into()]);
            if side == 0 && word(s, "camera") == "fp" && !flag(s, "reduceMotion") {
                toggle(&id("stage"), "down", true);
            }
        }
    }
    if !values(&get(&h, "hits")).is_empty() || word(&h, "kind") == "shell" {
        shake(s);
    }
    render(s);
}
fn next_turn(s: &JsValue) {
    set(s, "busy", &false.into());
    set(s, "turn", &get(&match_(s), "turn"));
    if over(s) {
        finish(s);
    } else {
        audio(s, "play", &["ui.turn".into()]);
        render(s);
        start_timer(s);
    }
}
fn log_turn(s: &JsValue, r: JsValue) {
    let fighters = get(&match_(s), "fighters");
    call(
        &get(s, "log"),
        "add",
        &[
            r,
            array(&[
                get(&at(&fighters, 0), "name"),
                get(&at(&fighters, 1), "name"),
            ])
            .into(),
            array(&[
                get(&at(&fighters, 0), "weapon"),
                get(&at(&fighters, 1), "weapon"),
            ])
            .into(),
        ],
    );
}
fn arm_both(s: &JsValue) {
    for side in 0..2 {
        let f = at(&get(&match_(s), "fighters"), side);
        let kit = get(&at(&get(&duel(s), "fighters"), side), "kit");
        let who: JsValue = if side == 0 { "me" } else { "enemy" }.into();
        if get(&get(s, "arena"), "wear").is_function() {
            arena(s, "wear", &[who.clone(), kit.clone()]);
        }
        arena(
            s,
            "arm",
            &[who, get(&f, "weapon"), get(&get(&kit, "weapon1"), "mods")],
        );
    }
}
fn fire(el: &JsValue, class: &str) {
    toggle(el, class, false);
    get(el, "offsetWidth");
    toggle(el, class, true);
}
fn shake(s: &JsValue) {
    if !flag(s, "reduceMotion") {
        fire(&id("stage"), "shake");
    }
}
fn floating(x: f64, y: f64, class: &str, big: &str, small: &str) {
    let el = call(&document(), "createElement", &["div".into()]);
    set(&el, "className", &format!("float {class}").into());
    style(&el, "left", &format!("{x}px"));
    style(&el, "top", &format!("{y}px"));
    html(
        &el,
        &format!(
            "{big}{}",
            if small.is_empty() {
                String::new()
            } else {
                format!("<small>{small}</small>")
            }
        ),
    );
    call(&id("rings"), "appendChild", &[el.clone()]);
    once(1700., move || {
        call(&el, "remove", &[]);
    });
}
fn strike(s: &JsValue, r: &JsValue, side: usize) {
    let a = at(&get(r, "attacks"), side);
    let target = 1 - side;
    // A hazard after the shots takes its own share later: the shots show the health before it.
    let before = get(&get(r, "hazard"), "hpBefore");
    let hp = at(
        &if present(&before) { before } else { get(r, "hp") },
        target,
    );
    Reflect::set(&get(s, "hp"), &(target as u32).into(), &hp).unwrap_throw();
    if present(&a) {
        let ours = side == 0;
        let hit = flag(&a, "hit");
        let blocked = flag(&a, "blocked");
        let zone = if present(&get(&a, "zone")) {
            word(&a, "zone")
        } else {
            "body".into()
        };
        if ours {
            fire(&id("flashOurs"), "on");
            shake(s);
        } else if hit && !blocked {
            fire(&id("flashHurt"), "on");
            if word(s, "camera") == "fp" {
                shake(s);
            }
        }
        let (x, y) = if ours {
            point(s, &zone)
        } else {
            (
                number(&id("stage"), "clientWidth") * 0.3,
                number(&id("stage"), "clientHeight") * 0.6,
            )
        };
        let label = if word(&a, "weapon") == "grenade" {
            "граната".into()
        } else if flag(&a, "crit") {
            "крит".into()
        } else {
            zone_name(&zone)
        };
        if !hit {
            floating(
                x + if ours { 30. } else { 0. },
                y - if ours { 10. } else { 0. },
                "miss",
                if ours { "Промах" } else { "мимо" },
                "",
            );
        } else {
            // The 3D dome shows a blocked hit; the flat ring stays for a fight without the arena.
            if ours && blocked && !present(&get(s, "arena")) {
                let el = call(&document(), "createElement", &["div".into()]);
                set(&el, "className", &"ripple".into());
                style(&el, "left", &format!("{x}px"));
                style(&el, "top", &format!("{y}px"));
                call(&id("rings"), "appendChild", &[el.clone()]);
                once(900., move || {
                    call(&el, "remove", &[]);
                });
            }
            // Shotgun pellets that caught a neighbouring zone get their own small number there.
            let splash = get(&a, "splash");
            let pellets = if present(&splash) {
                number(&splash, "length") as u32
            } else {
                0
            };
            let mut main = number(&a, "damage");
            for i in 0..pellets {
                let h = at(&splash, i as usize);
                main -= number(&h, "damage");
                if ours {
                    let (px, py) = point(s, &word(&h, "zone"));
                    floating(
                        px - 30.,
                        py + 8.,
                        if flag(&h, "blocked") {
                            "blocked"
                        } else {
                            "dmg"
                        },
                        &format!("−{}", number(&h, "damage")),
                        "дробь",
                    );
                }
            }
            floating(
                x + if ours { 36. } else { 0. },
                y - if ours { 14. } else { 0. },
                if blocked { "blocked" } else { "dmg" },
                &format!("−{}", if ours { main } else { number(&a, "damage") }),
                if blocked { "Зонт" } else { &label },
            );
            let kind = if word(&a, "weapon") == "grenade" {
                "grenade"
            } else if blocked {
                "blocked"
            } else {
                "hit"
            };
            arena(
                s,
                "hit",
                &[
                    if ours { "enemy" } else { "me" }.into(),
                    zone.into(),
                    kind.into(),
                    word(&a, "weapon").into(),
                    flag(&a, "shieldBroken").into(),
                ],
            );
        }
    }
    if num(&hp) <= 0. {
        arena(s, "fall", &[if side == 0 { "enemy" } else { "me" }.into()]);
        if side == 1 && word(s, "camera") == "fp" && !flag(s, "reduceMotion") {
            toggle(&id("stage"), "down", true);
        }
    }
    render(s);
}

fn finish(s: &JsValue) {
    stop_timer(s);
    render(s);
    let m = match_(s);
    let result = get(&m, "result");
    let history = get(&m, "history");
    let words = core(
        "words.endWords",
        &[
            result.clone(),
            0.into(),
            get(&at(&get(&duel(s), "fighters"), 1), "name"),
            get(&history, "length"),
        ],
    );
    let lost = flag(&words, "lost");
    toggle(&id("endVeil"), "lost", lost);
    for (el, key) in [
        ("endTitle", "title"),
        ("endText", "text"),
        ("againBtn", "again"),
    ] {
        text(&id(el), &word(&words, key));
    }
    html(
        &id("endStats"),
        &string(&core("ui.finishStats", &[history])),
    );
    // Experience and mastery go to the capsule's synchronisation.
    let gained = core("syncGain", &[sync_profile(s), m.clone(), 0.into()]);
    if let Some(raw) = js_sys::JSON::stringify(&get(&gained, "profile"))
        .ok()
        .and_then(|v| v.as_string())
    {
        store_set(s, "cd-sync", &raw);
    }
    html(
        &id("endSync"),
        &values(&get(&gained, "lines"))
            .iter()
            .map(|l| format!("<span>{}</span>", string(l)))
            .collect::<String>(),
    );
    html(
        &id("endBadge"),
        &string(&host(
            s,
            "icon",
            &[if lost { "i-signal" } else { "i-shot" }.into()],
        )),
    );
    hide(&id("baseEndBtn"), lost);
    set(s, "againHome", &lost.into());
    hide(&id("endVeil"), false);
    call(&id("againBtn"), "focus", &[]);
    let winner = get(&result, "winner");
    audio(
        s,
        "finish",
        &[if present(&winner) {
            (num(&winner) == 0.).into()
        } else {
            JsValue::NULL
        }],
    );
}
fn restart(s: &JsValue, keep_sheet: bool) {
    set(s, "fightNo", &(number(s, "fightNo") + 1.).into());
    // A new fight starts at the map's current hour and weather.
    let live = live_frame(s, Some(&word(&get(s, "frame"), "mapId")));
    if present(&live) {
        set(s, "frame", &live);
    }
    hide(&id("endVeil"), true);
    toggle(&id("stage"), "down", false);
    set(s, "duel", &new_duel(s));
    set(s, "busy", &false.into());
    let m = match_(s);
    let canonical = core(
        "client.conditions.frame",
        &[
            get(s, "frames"),
            get(&m, "config"),
            get(&get(s, "frame"), "id"),
        ],
    );
    if present(&canonical) {
        set_frame(s, &canonical);
    }
    let fighters = get(&m, "fighters");
    set(
        s,
        "hp",
        &array(&[get(&at(&fighters, 0), "hp"), get(&at(&fighters, 1), "hp")]).into(),
    );
    set(s, "turn", &get(&m, "turn"));
    arm_both(s);
    arena(s, "reset", &[get(&m, "distance")]);
    let distance = number(&m, "distance");
    let weapon = word(&at(&get(&duel(s), "fighters"), 1), "weapon");
    let with = word(&get(&get(s, "words"), "GUN_WITH"), &weapon);
    call(
        &get(s, "log"),
        "reset",
        &[format!("Запись идёт · Искра {with}, {distance} {}", steps(distance)).into()],
    );
    draw_header(s);
    render(s);
    start_timer(s);
    audio(s, "newFight", &[m]);
    set_sheet(s, keep_sheet);
}
fn set_sheet(s: &JsValue, open: bool) {
    hide(&id("infoVeil"), !open);
    if open {
        render_menu(s);
        call(
            &id("closeInfo"),
            "focus",
            &[object(&[("preventScroll", true.into())])],
        );
    }
    audio(s, "sheet", &[open.into()]);
}
fn segment(name: &str, count: usize) -> JsValue {
    let el = id(name);
    html(&el, "");
    call(
        &get(&el, "style"),
        "setProperty",
        &["--n".into(), count.to_string().into()],
    );
    el
}
fn option(parent: &JsValue, content: &str, selected: bool, action: impl FnMut(JsValue) + 'static) {
    let b = call(&document(), "createElement", &["button".into()]);
    set(&b, "type", &"button".into());
    set(&b, "className", &"glass".into());
    html(&b, content);
    attr(&b, "aria-pressed", if selected { "true" } else { "false" });
    listen(&b, "click", action);
    call(parent, "appendChild", &[b]);
}
/// The game clock picks the map's hour and weather. Address keys (`time`,
/// `weather`, `frame`, `map`, `cal`) pin them for checks and screenshots; the
/// menu's test pickers pin time and weather until "По часам" is chosen again.
fn live_frame(s: &JsValue, map: Option<&str>) -> JsValue {
    let params = construct(
        &get(&global(), "URLSearchParams"),
        &[get(&get(&global(), "location"), "search")],
    );
    let param = |key: &str| call(&params, "get", &[key.into()]);
    let pinned = |key: &str, stored: &str| {
        let value = param(key);
        if value.is_null() {
            store_get(s, stored)
        } else {
            value
        }
    };
    core(
        "client.conditions.live",
        &[
            get(s, "frames"),
            object(&[
                ("time", pinned("time", "cd-test-time")),
                (
                    "frame",
                    if map.is_some() {
                        JsValue::NULL
                    } else {
                        param("frame")
                    },
                ),
                ("cal", call(&params, "has", &["cal".into()])),
                ("map", map.map_or_else(|| param("map"), JsValue::from)),
                ("savedMap", store_get(s, "cd-map")),
                ("weather", pinned("weather", "cd-test-weather")),
                ("now", js_sys::Date::now().into()),
            ]),
        ],
    )
}
/// Thunder follows each lightning flash after a second or two, like a strike
/// a few hundred metres away.
fn thunder(s: &JsValue) {
    let strikes = number(&get(s, "arena"), "strikes");
    if strikes <= number(s, "strikesHeard") {
        return;
    }
    set(s, "strikesHeard", &strikes.into());
    let state = s.clone();
    let roll = Closure::once_into_js(move || audio(&state, "play", &["thunder".into()]));
    call(
        &global(),
        "setTimeout",
        &[roll, (900. + js_sys::Math::random() * 1600.).into()],
    );
}
fn choose_map(s: &JsValue, map: &str) {
    let frame = live_frame(s, Some(map));
    if !present(&frame) {
        return;
    }
    store_set(s, "cd-map", &word(&frame, "mapId"));
    set_frame(s, &frame);
    restart(s, false);
}
fn minutes(ms: f64) -> String {
    let m = (ms / 60_000.).ceil().max(1.) as u32;
    let unit = match (m % 10, m % 100) {
        (1, x) if x != 11 => "минуту",
        (2..=4, x) if !(12..=14).contains(&x) => "минуты",
        _ => "минут",
    };
    format!("{m} {unit}")
}
fn render_menu(s: &JsValue) {
    let bindings = get(s, "host");
    let held = word(&at(&get(&duel(s), "fighters"), 0), "weapon");
    let ids = values(&get(&bindings, "weaponIds"));
    let weapons = segment("weaponSeg", ids.len());
    for w in ids {
        let weapon = string(&w);
        let same = weapon == held;
        let state = s.clone();
        option(
            &weapons,
            &word(&get(&get(&bindings, "weapons"), &weapon), "name"),
            same,
            move |_| {
                if same {
                    return;
                }
                set(&state, "weapon", &w);
                store_set(&state, "cd-weapon", &weapon);
                set(&state, "fitted", &JsValue::NULL);
                restart(&state, true);
            },
        );
    }
    let vet = flag(s, "veteran") && !present(&get(s, "fitted"));
    toggle(&id("vetBtn"), "off", !vet);
    attr(
        &id("vetBtn"),
        "aria-pressed",
        if vet { "true" } else { "false" },
    );
    text(&id("vetState"), if vet { "Вкл" } else { "Выкл" });
    let on = flag(&get(s, "audio"), "on");
    toggle(&id("soundBtn"), "off", !on);
    attr(
        &id("soundBtn"),
        "aria-pressed",
        if on { "true" } else { "false" },
    );
    text(&id("soundState"), if on { "Вкл" } else { "Выкл" });
    for kind in ["fx", "music", "fight"] {
        draw_volume(s, kind);
    }
    let tracks = id("trackList");
    html(&tracks, "");
    let station = get(&get(s, "audio"), "station");
    for song in values(&get(&bindings, "playlist")) {
        let now = song == station;
        let title = word(&get(&get(&bindings, "tracks"), &string(&song)), "title");
        let state = s.clone();
        option(
            &tracks,
            &format!(
                "{title}<small>{}</small>",
                if now { "в эфире" } else { "&nbsp;" }
            ),
            now,
            move |_| {
                audio(&state, "tune", &[song.clone()]);
                render_menu(&state);
            },
        );
    }
    let cameras = segment("camSeg", 2);
    for (view, name) in [("back", "Из-за плеча"), ("fp", "От первого лица")] {
        let state = s.clone();
        option(&cameras, name, word(s, "camera") == view, move |_| {
            if word(&state, "camera") != view {
                set_camera(&state, view);
                render_menu(&state);
            }
        });
    }
    let gfx = segment("gfxSeg", QUALITY_IDS.len());
    for q in QUALITY_IDS {
        let state = s.clone();
        option(
            &gfx,
            &word(&quality(s, q), "name"),
            word(s, "quality") == q,
            move |_| {
                if word(&state, "quality") != q {
                    set_quality(&state, q, true);
                    render_menu(&state);
                }
            },
        );
    }
    let note = match word(s, "quality").as_str() {
        "ultra" => {
            "Для мощного ПК: всё на максимум, самые мягкие тени, кадр рисуется крупнее экрана и сглаживается."
        }
        "high" => "Для компьютера: дымка с тенями бойцов, текстуры 2К, чёткий фон.",
        "medium" => "Для телефона: мягкие тени, лучи света из окон, пыль в воздухе.",
        "minimal" => "Для самых слабых телефонов: кадр мельче, до 30 кадров в секунду, без теней, дымки и пыли.",
        _ => "Для слабых телефонов: без теней, дымки и пыли, картинка попроще.",
    };
    text(
        &id("gfxNote"),
        &format!(
            "{note}{}",
            if flag(s, "byHand") {
                ""
            } else {
                " Подобрано под это устройство."
            }
        ),
    );
    let current_map = word(&get(&match_(s), "config"), "mapId");
    let all_maps = values(&core("client.conditions.maps", &[get(s, "frames")]));
    let clocks = core("client.world.maps", &[js_sys::Date::now().into()]);
    let clock_of = |map: &str| {
        values(&clocks)
            .into_iter()
            .find(|c| word(c, "mapId") == map)
            .unwrap_or(JsValue::NULL)
    };
    let current_region = all_maps
        .iter()
        .find(|m| word(m, "id") == current_map)
        .map(|m| word(m, "region"))
        .unwrap_or_default();
    let regions = values(&core("client.conditions.regions", &[get(s, "frames")]));
    let rs = segment("regionSeg", regions.len());
    for region in regions {
        let wanted = word(&region, "id");
        let current = wanted == current_region;
        // A region opens on its first drawn map, at that map's own hour.
        let first = all_maps
            .iter()
            .find(|m| word(m, "region") == wanted)
            .map(|m| word(m, "id"))
            .unwrap_or_default();
        let state = s.clone();
        option(&rs, &word(&region, "name"), current, move |_| {
            if !current {
                choose_map(&state, &first);
            }
        });
    }
    let maps: Vec<JsValue> = all_maps
        .into_iter()
        .filter(|m| word(m, "region") == current_region)
        .collect();
    let ms = segment("mapSeg", maps.len());
    for choice in maps {
        let wanted = word(&choice, "id");
        let current = wanted == current_map;
        let c = clock_of(&wanted);
        let left = number(&c, "changeInMs");
        let shut = get(&c, "open") == JsValue::FALSE;
        // The map buttons are a debug tool like the weapon picker (Art), so a
        // shut map still opens; it says why and for how long, and an open one
        // warns five minutes before the clock shuts it.
        let note = if shut {
            format!(
                "<small class=\"shut\">{} · ещё {}</small>",
                word(&c, "closedName"),
                minutes(left)
            )
        } else if left > 0. && left <= 300_000. && present(&get(&c, "nextClosed")) {
            format!(
                "<small class=\"shut\">{} через {}</small>",
                word(&c, "nextClosedName"),
                minutes(left)
            )
        } else {
            format!(
                "<small>{} · {}, {}</small>",
                word(&c, "clock"),
                word(&c, "timeName"),
                word(&c, "weatherName")
            )
        };
        let state = s.clone();
        option(
            &ms,
            &format!("{}{note}", word(&choice, "name")),
            current,
            move |_| {
                if !current {
                    choose_map(&state, &wanted);
                }
            },
        );
    }
    let c = clock_of(&current_map);
    hide(&id("frameLabel"), false);
    // Test pickers: a choice holds until "По часам" and starts a new fight with the same kit.
    let picks = core(
        "client.conditions.picks",
        &[get(s, "frames"), current_map.as_str().into()],
    );
    let mut manual = false;
    for (seg, key, list) in [
        ("timeSeg", "cd-test-time", "times"),
        ("weatherSeg", "cd-test-weather", "weathers"),
    ] {
        let chosen = store_get(s, key).as_string().unwrap_or_default();
        let options = values(&get(&picks, list));
        let known = options.iter().any(|o| word(o, "id") == chosen);
        manual |= known && !chosen.is_empty();
        let row = segment(seg, options.len());
        for choice in options {
            let wanted = word(&choice, "id");
            let current = wanted == chosen || (!known && wanted.is_empty());
            let state = s.clone();
            let map = current_map.clone();
            option(&row, &word(&choice, "name"), current, move |_| {
                if !current {
                    store_set(&state, key, &wanted);
                    choose_map(&state, &map);
                }
            });
        }
    }
    let weather_next = if word(&c, "nextWeather") == word(&c, "weather") {
        format!(
            "{} ещё {}",
            word(&c, "weatherName"),
            minutes(number(&c, "nextWeatherInMs"))
        )
    } else {
        format!(
            "через {} {}",
            minutes(number(&c, "nextWeatherInMs")),
            word(&c, "nextWeatherName")
        )
    };
    let note = if manual {
        format!(
            "Часы на этой карте отключены вашим выбором. По часам сейчас было бы {}, {}, {}.",
            word(&c, "clock"),
            word(&c, "timeName"),
            word(&c, "weatherName")
        )
    } else {
        format!(
            "Сейчас {}, {}, {}. Через {} {}, {}. В регионе: {}.",
            word(&c, "clock"),
            word(&c, "timeName"),
            word(&c, "weatherName"),
            minutes(number(&c, "nextTimeInMs")),
            word(&c, "nextTimeName"),
            weather_next,
            word(&c, "eventName").to_lowercase()
        )
    };
    text(&id("clockNote"), &note);
    text(
        &id("conditionNote"),
        &string(&core("client.conditions.description", &[match_(s)])),
    );
    host(s, "liquidGlass", &[id("infoVeil")]);
}
fn volume_id(kind: &str) -> &'static str {
    match kind {
        "music" => "volMusic",
        "fight" => "volFight",
        _ => "volFx",
    }
}
fn draw_volume(s: &JsValue, kind: &str) {
    let v = (number(&get(&get(s, "audio"), "volume"), kind) * 100.).round();
    let name = volume_id(kind);
    let el = id(name);
    set(&el, "value", &v.to_string().into());
    call(
        &get(&el, "style"),
        "setProperty",
        &["--v".into(), format!("{v}%").into()],
    );
    text(&id(&format!("{name}Out")), &v.to_string());
}
fn quality(s: &JsValue, q: &str) -> JsValue {
    get(&get(&get(s, "words"), "QUALITY"), q)
}
fn set_quality(s: &JsValue, q: &str, picked: bool) {
    set(s, "quality", &q.into());
    if picked {
        set(s, "byHand", &true.into());
        store_set(s, "cd-quality", q);
    }
    arena(s, "setQuality", &[quality(s, q)]);
    layout(s);
}
fn request_speed(s: &JsValue) {
    set(
        s,
        "speedFrame",
        &call(
            &global(),
            "requestAnimationFrame",
            &[get(s, "speedCallback")],
        ),
    );
}
fn install_speed_watch(s: &JsValue) {
    let state = s.clone();
    let cb = Closure::<dyn FnMut(f64)>::new(move |t: f64| {
        let s = &state;
        if flag(s, "byHand") || !flag(s, "active") {
            return;
        }
        if flag(&document(), "hidden") {
            set(s, "speedStart", &0.into());
            set(s, "speedCount", &0.into());
            request_speed(s);
            return;
        }
        if number(s, "speedStart") == 0. {
            set(s, "speedStart", &t.into());
        }
        let count = number(s, "speedCount") + 1.;
        set(s, "speedCount", &count.into());
        let duration = t - number(s, "speedStart");
        if duration < 4000. {
            request_speed(s);
            return;
        }
        let i = QUALITY_IDS
            .iter()
            .position(|q| *q == word(s, "quality"))
            .unwrap_or(0);
        if count * 1000. / duration < 24. && i > 0 {
            set_quality(s, QUALITY_IDS[i - 1], false);
            let state = s.clone();
            let timeout = once(1500., move || watch_speed(&state));
            set(s, "speedDelay", &timeout);
        }
    })
    .into_js_value();
    set(s, "speedCallback", &cb);
}
fn watch_speed(s: &JsValue) {
    call(&global(), "cancelAnimationFrame", &[get(s, "speedFrame")]);
    set(s, "speedStart", &0.into());
    set(s, "speedCount", &0.into());
    request_speed(s);
}
fn set_frame(s: &JsValue, frame: &JsValue) {
    set(s, "frame", frame);
    set(&id("bg"), "src", &get(frame, "src"));
    set(
        &id("bg"),
        "alt",
        &format!("Арена: {}", word(frame, "name")).into(),
    );
    arena(s, "setFrame", &[frame.clone()]);
    layout(s);
}
fn set_camera(s: &JsValue, view: &str) {
    set(s, "camera", &view.into());
    store_set(s, "cd-camera", view);
    arena(s, "setFirstPerson", &[(view == "fp").into()]);
}
fn wire(s: &JsValue) {
    for b in all(".dz") {
        let state = s.clone();
        let zone = word(&get(&b, "dataset"), "z");
        listen(&b, "click", move |_| press(&state, "guard", Some(&zone)));
    }
    for b in all(".mv") {
        let state = s.clone();
        let mv = word(&get(&b, "dataset"), "m").parse().unwrap_or(0.);
        listen(&b, "click", move |_| move_to(&state, mv));
    }
    for (name, kind) in [
        ("knifeBtn", "knife"),
        ("nadeBtn", "grenade"),
        ("swapBtn", "swap"),
    ] {
        let state = s.clone();
        listen(&id(name), "click", move |_| press(&state, kind, None));
    }
    button(s, "goBtn", end_turn);
    button(s, "againBtn", |s| {
        if flag(s, "againHome") {
            leave(s)
        } else {
            restart(s, false)
        }
    });
    button(s, "restartBtn", |s| restart(s, false));
    button(s, "moreBtn", |s| set_sheet(s, true));
    button(s, "closeInfo", |s| set_sheet(s, false));
    button(s, "closeX", |s| set_sheet(s, false));
    let state = s.clone();
    listen(&id("infoVeil"), "click", move |e| {
        if get(&e, "target") == id("infoVeil") {
            set_sheet(&state, false);
        }
    });
    button(s, "soundBtn", |s| {
        audio(s, "toggle", &[]);
        render_menu(s);
    });
    button(s, "vetBtn", |s| {
        let on = present(&get(s, "fitted")) || !flag(s, "veteran");
        set(s, "veteran", &on.into());
        store_set(s, "cd-veteran", if on { "on" } else { "off" });
        set(s, "fitted", &JsValue::NULL);
        restart(s, true);
    });
    button(s, "logBtn", |s| {
        let log = get(s, "log");
        call(&log, "setOpen", &[(!flag(&log, "open")).into()]);
    });
    button(s, "logClose", |s| {
        call(&get(s, "log"), "setOpen", &[false.into()]);
    });
    let state = s.clone();
    listen(&document(), "keydown", move |e| {
        if word(&e, "key") != "Escape" {
            return;
        }
        if !flag(&id("infoVeil"), "hidden") {
            set_sheet(&state, false);
        } else if flag(&get(&state, "log"), "open") {
            call(&get(&state, "log"), "setOpen", &[false.into()]);
        }
    });
    button(s, "baseInfoBtn", leave);
    button(s, "baseEndBtn", leave);
    let state = s.clone();
    listen(&document(), "click", move |e| {
        let target = get(&e, "target");
        if !flag(&state, "active") || !present(&target) {
            return;
        }
        let b = try_call(&target, "closest", &["button".into()]).unwrap_or(JsValue::NULL);
        if !present(&b)
            || !boolean(&call(&id("app"), "contains", &[b.clone()]))
            || ["goBtn", "soundBtn"].contains(&word(&b, "id").as_str())
        {
            return;
        }
        let classes = get(&b, "classList");
        let sound = if boolean(&call(&classes, "contains", &["dz".into()])) {
            "ui.guard"
        } else if boolean(&call(&classes, "contains", &["mv".into()])) {
            "ui.step"
        } else {
            "ui.tap"
        };
        audio(&state, "play", &[sound.into()]);
    });
    for event in ["pointerdown", "keydown"] {
        let state = s.clone();
        let cb = Closure::<dyn FnMut()>::new(move || audio(&state, "unlock", &[])).into_js_value();
        call(
            &document(),
            "addEventListener",
            &[event.into(), cb, object(&[("once", true.into())])],
        );
    }
    let state = s.clone();
    listen(&global(), "resize", move |_| layout(&state));
    let state = s.clone();
    listen(&global(), "hashchange", move |_| {
        if word(&get(&global(), "location"), "hash") == "#fp" {
            set_camera(&state, "fp");
        }
    });
    for kind in ["fx", "music", "fight"] {
        let state = s.clone();
        listen(&id(volume_id(kind)), "input", move |_| {
            let value = word(&id(volume_id(kind)), "value")
                .parse::<f64>()
                .unwrap_or(0.)
                / 100.;
            audio(&state, "setVolume", &[kind.into(), value.into()]);
            draw_volume(&state, kind);
        });
    }
    let state = s.clone();
    listen(&id("volFx"), "change", move |_| {
        audio(&state, "play", &["ui.tap".into()])
    });
}
fn prepare_arena(s: &JsValue) -> Promise {
    let a = get(s, "arena");
    if !present(&a) {
        return Promise::reject(&js_sys::Error::new("3D не запустилось на этом устройстве"));
    }
    let sex = string(&get(&super::platform::current_look(), "sex"));
    if flag(&a, "ready") {
        let loaded = string(&get(s, "arenaSex"));
        if !loaded.is_empty() && loaded != sex {
            // The loaded figures belong to the other sex; a fresh page loads the right pair.
            call(&get(&global(), "location"), "reload", &[]);
            return Promise::new(&mut |_, _| {});
        }
        if get(&a, "applyLook").is_function() {
            call(&a, "applyLook", &[]);
        }
        return Promise::resolve(&JsValue::UNDEFINED);
    }
    set(s, "arenaSex", &sex.clone().into());
    let existing = get(s, "arenaLoading");
    if present(&existing) {
        return existing.unchecked_into();
    }
    let state = s.clone();
    let promise = future_to_promise(async move {
        let s = &state;
        let bindings = get(s, "host");
        // The foe is always the other sex, so the two figures never share materials.
        let (mine, theirs) = if sex == "female" {
            (get(&bindings, "foeModel"), get(&bindings, "myModel"))
        } else {
            (get(&bindings, "myModel"), get(&bindings, "foeModel"))
        };
        let result = async {
            let loading = try_call(&a, "load", &[mine, theirs, get(&match_(s), "distance")])?;
            JsFuture::from(Promise::resolve(&loading)).await?;
            if !flag(&a, "ready") {
                return Err(js_sys::Error::new("Арена ещё не готова").into());
            }
            arm_both(s);
            arena(s, "reset", &[get(&match_(s), "distance")]);
            let decode = try_call(&id("bg"), "decode", &[]).unwrap_or(JsValue::UNDEFINED);
            let decoded = future_to_promise(async move {
                let _ = JsFuture::from(Promise::resolve(&decode)).await;
                Ok(JsValue::UNDEFINED)
            });
            let all = Promise::all(&array(&[get(&a, "texturesDone"), decoded.into()]));
            let timeout = Promise::new(&mut |resolve, _| {
                call(&global(), "setTimeout", &[resolve.into(), 25000.into()]);
            });
            JsFuture::from(Promise::race(&array(&[all.into(), timeout.into()]))).await?;
            settle(&a, "warm", 15000.).await;
            hide(&id("status"), true);
            layout(s);
            render(s);
            Ok(JsValue::UNDEFINED)
        }
        .await;
        if result.is_err() {
            set(s, "arenaLoading", &JsValue::NULL);
        }
        result
    });
    set(s, "arenaLoading", promise.as_ref());
    promise
}
fn leave(s: &JsValue) {
    set(s, "active", &false.into());
    set(s, "fightNo", &(number(s, "fightNo") + 1.).into());
    stop_timer(s);
    set(s, "busy", &false.into());
    call(&global(), "cancelAnimationFrame", &[get(s, "speedFrame")]);
    call(&global(), "clearTimeout", &[get(s, "speedDelay")]);
    set(s, "speedDelay", &JsValue::NULL);
    arena(s, "pause", &[]);
    hide(&id("infoVeil"), true);
    hide(&id("endVeil"), true);
    call(&get(s, "log"), "setOpen", &[false.into()]);
    audio(s, "finish", &[JsValue::NULL]);
    hide(&id("app"), true);
    let exit = get(s, "exit");
    if present(&exit) {
        invoke(&exit, &[]);
    }
}
