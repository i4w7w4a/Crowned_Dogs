//! Page audio policy: persistence, screen ambience, radio placement and silent QA mode.
use super::dom_support::*;
use js_sys::{Array, Object, Promise};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
fn quiet(promise: JsValue) {
    if exists(&promise) {
        spawn_local(async move {
            let _ = JsFuture::from(Promise::resolve(&promise)).await;
        });
    }
}
fn ui() -> JsValue {
    args(
        &[
            "ui.tap",
            "ui.guard",
            "ui.step",
            "ui.confirm",
            "ui.tick",
            "ui.tick.urgent",
            "ui.turn",
            "ui.victory",
            "ui.defeat",
        ]
        .map(js),
    )
    .into()
}
fn volume_key(kind: &str) -> &str {
    match kind {
        "music" => "cd-vol-music",
        "fight" => "cd-vol-fight",
        _ => "cd-vol-fx",
    }
}
#[wasm_bindgen]
pub struct BrowserArenaAudio {
    host: JsValue,
    audio: JsValue,
    enabled: bool,
    picked: JsValue,
    preloads: JsValue,
    mode: String,
    list: JsValue,
    spot: (f64, f64),
    ambience: JsValue,
    /// Reverb preset of the current fight map.
    room: String,
    levels: JsValue,
    silent: bool,
    /// An instructor line asked for before the first tap built the audio; it plays on unlock.
    voice: Option<String>,
}
#[wasm_bindgen]
impl BrowserArenaAudio {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue) -> Self {
        let store = get(&host, "store");
        let enabled = text(&call(&store, "get", &[js("cd-music")])) != "off";
        let radio = get(&host, "RADIO_PLAYLIST");
        let saved = call(&store, "get", &[js("cd-station")]);
        let picked = if Array::from(&radio).iter().any(|id| id == saved) {
            saved
        } else {
            Array::from(&radio).get(0)
        };
        let levels = Object::new();
        for kind in ["fx", "music", "fight"] {
            let saved = call(&store, "get", &[js(volume_key(kind))]);
            let value = if exists(&saved) {
                let raw = text(&saved);
                if raw.trim().is_empty() {
                    0.0
                } else {
                    raw.parse::<f64>().unwrap_or(1.0)
                }
            } else {
                1.0
            };
            set(
                &levels,
                kind,
                &number(if value.is_finite() {
                    value.clamp(0.0, 1.0)
                } else {
                    1.0
                }),
            );
        }
        let list = get(&host, "BASE_PLAYLIST");
        let silent = boolean(&host, "silent");
        Self {
            host,
            audio: JsValue::NULL,
            enabled,
            picked,
            preloads: ui(),
            mode: "menu".into(),
            list,
            spot: (-0.55, 0.0),
            ambience: args(&[js("amb.base")]).into(),
            room: "hall".into(),
            levels: levels.into(),
            silent,
            voice: None,
        }
    }
    #[wasm_bindgen(getter)]
    pub fn on(&self) -> bool {
        self.enabled
    }
    #[wasm_bindgen(getter)]
    pub fn volume(&self) -> JsValue {
        self.levels.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn station(&self) -> JsValue {
        let station = get(&get(&get(&self.audio, "music"), "state"), "radioTrack");
        if exists(&station) {
            station
        } else {
            self.picked.clone()
        }
    }
    pub fn unlock(&mut self) {
        // Return before constructing either Web Audio or a media element in silent QA mode.
        if self.silent {
            return;
        }
        if exists(&self.audio) {
            quiet(call(&self.audio, "unlock", &[]));
            return;
        }
        let options = Object::new();
        set(&options, "baseUrl", &js("./"));
        let Ok(audio) = construct(&self.host, "GameAudio", &[options.into()]) else {
            return;
        };
        self.audio = audio;
        quiet(call(&self.audio, "unlock", &[]));
        call(
            &self.audio,
            "setVolume",
            &[js("master"), number(if self.enabled { 1.0 } else { 0.0 })],
        );
        self.apply_levels();
        let music = get(&self.audio, "music");
        call(&music, "tune", &[self.picked.clone()]);
        call(&music, "playlist", &[self.list.clone()]);
        call(
            &self.audio,
            "placeRadio",
            &[number(self.spot.0), number(self.spot.1)],
        );
        call(&self.audio, "setAmbience", &[self.ambience.clone()]);
        call(&self.audio, "setRoom", &[js(&self.room)]);
        quiet(call(&self.audio, "preload", &[self.preloads.clone()]));
        if let Some(id) = self.voice.take() {
            call(&self.audio, "play", &[js(&id), JsValue::UNDEFINED]);
        }
        if self.mode == "menu" {
            call(&music, "enterMenu", &[]);
        } else {
            call(&music, "enterFight", &[]);
            if self.mode == "paused" {
                call(&music, "openMenu", &[]);
            }
        }
    }
    #[wasm_bindgen(js_name=setPlace)]
    pub fn set_place(&mut self, m: JsValue) {
        let config = get(&m, "config");
        // Open-air maps share the courtyard's short outdoor tail and ambience.
        let blocks = matches!(
            text(&get(&config, "mapId")).as_str(),
            "blocks" | "dam" | "dryBed"
        );
        let bed = if blocks { "amb.blocks" } else { "amb.hall" };
        self.room = if blocks { "blocks" } else { "hall" }.into();
        call_optional(&self.audio, "setRoom", &[js(&self.room)]);
        let ids = Array::new();
        ids.push(&js(bed));
        let layer = match text(&get(&config, "weather")).as_str() {
            "dust" => Some("amb.dust"),
            "storm" => Some("amb.storm"),
            "fog" => Some("amb.fog"),
            _ => None,
        };
        if let Some(layer) = layer {
            ids.push(&js(layer));
        }
        self.ambience = ids.into();
        call_optional(&self.audio, "setAmbience", &[self.ambience.clone()]);
    }
    pub fn screen(&mut self, view: &str) {
        self.mode = "menu".into();
        let base = view == "base";
        self.list = get(
            &self.host,
            if base {
                "BASE_PLAYLIST"
            } else {
                "FITTING_PLAYLIST"
            },
        );
        self.ambience = args(&[js(if base { "amb.base" } else { "amb.hall" })]).into();
        self.spot = if base { (-0.55, 0.0) } else { (0.6, 1.0) };
        if !exists(&self.audio) {
            return;
        }
        call(
            &self.audio,
            "placeRadio",
            &[number(self.spot.0), number(self.spot.1)],
        );
        call(&self.audio, "setAmbience", &[self.ambience.clone()]);
        self.amb_level();
        let music = get(&self.audio, "music");
        call(&music, "enterMenu", &[]);
        call(&music, "playlist", &[self.list.clone()]);
    }
    #[wasm_bindgen(js_name=newFight)]
    pub fn new_fight(&mut self, m: JsValue) {
        let preloads = Array::from(&ui());
        let sounds = call(&self.host, "fightSounds", &[guns(&m, 0), guns(&m, 1)]);
        for sound in Array::from(&sounds).iter() {
            preloads.push(&sound);
        }
        self.preloads = preloads.into();
        self.set_place(m);
        self.mode = "fight".into();
        if !exists(&self.audio) {
            return;
        }
        quiet(call(&self.audio, "preload", &[self.preloads.clone()]));
        call(&get(&self.audio, "music"), "enterFight", &[]);
        self.amb_level();
    }
    pub fn sheet(&mut self, open: bool) {
        if self.mode == "menu" {
            return;
        }
        self.mode = if open { "paused" } else { "fight" }.into();
        self.radio_list();
        call_optional(
            &get(&self.audio, "music"),
            if open { "openMenu" } else { "closeMenu" },
            &[],
        );
        self.amb_level();
    }
    pub fn finish(&mut self, won: Option<bool>) {
        if let Some(won) = won {
            let options = Object::new();
            set(&options, "delay", &number(0.9));
            self.play(if won { "ui.victory" } else { "ui.defeat" }, options.into());
        }
        self.mode = "menu".into();
        self.radio_list();
        call_optional(&get(&self.audio, "music"), "enterMenu", &[]);
        self.amb_level();
    }
    /// `ears` is the arena's [our pan, enemy pan] seen from the camera, or undefined before it loads.
    pub fn turn(&self, report: JsValue, m: JsValue, ears: JsValue) {
        if !exists(&self.audio) {
            return;
        }
        let fighters = Array::from(&get(&m, "fighters"));
        let a = fighters.get(0);
        let b = fighters.get(1);
        let options = Object::new();
        set(&options, "viewer", &number(0.0));
        set(
            &options,
            "armor",
            &args(&[get(&a, "armor"), get(&b, "armor")]),
        );
        set(
            &options,
            "weapons",
            &args(&[get(&a, "weapon"), get(&b, "weapon")]),
        );
        if ears.is_object() {
            set(&options, "pan", &ears);
        }
        let cues = call(&self.host, "cuesForTurn", &[report, options.into()]);
        call(&self.audio, "playCues", &[cues, number(0.0)]);
    }
    pub fn play(&self, id: &str, options: JsValue) {
        call_optional(&self.audio, "play", &[js(id), options]);
    }
    /// The instructor speaks; a newer line or stopVoice cuts the one on air.
    pub fn voice(&mut self, id: &str) {
        if exists(&self.audio) {
            self.play(id, JsValue::UNDEFINED);
        } else {
            self.voice = Some(id.into());
        }
    }
    #[wasm_bindgen(js_name=stopVoice)]
    pub fn stop_voice(&mut self) {
        self.voice = None;
        call_optional(&self.audio, "stopVoice", &[]);
    }
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        call(
            &get(&self.host, "store"),
            "set",
            &[js("cd-music"), js(if self.enabled { "on" } else { "off" })],
        );
        self.unlock();
        call_optional(
            &self.audio,
            "setVolume",
            &[js("master"), number(if self.enabled { 1.0 } else { 0.0 })],
        );
    }
    #[wasm_bindgen(js_name=setVolume)]
    pub fn set_volume(&mut self, kind: &str, value: f64) {
        let value = value.clamp(0.0, 1.0);
        set(&self.levels, kind, &number(value));
        call(
            &get(&self.host, "store"),
            "set",
            &[js(volume_key(kind)), js(&format!("{value:.2}"))],
        );
        self.apply_levels();
    }
    pub fn tune(&mut self, station: JsValue) {
        self.picked = station.clone();
        call(
            &get(&self.host, "store"),
            "set",
            &[js("cd-station"), station.clone()],
        );
        if !self.enabled {
            self.toggle();
        }
        self.unlock();
        call_optional(&get(&self.audio, "music"), "tune", &[station]);
    }
}
impl BrowserArenaAudio {
    fn amb_level(&self) {
        call_optional(
            &self.audio,
            "setVolume",
            &[
                js("amb"),
                number(num(&self.levels, "fx") * if self.mode == "fight" { 1.0 } else { 0.62 }),
            ],
        );
    }
    fn apply_levels(&self) {
        if !exists(&self.audio) {
            return;
        }
        for bus in ["sfx", "ui"] {
            call(
                &self.audio,
                "setVolume",
                &[js(bus), get(&self.levels, "fx")],
            );
        }
        call(
            &self.audio,
            "setVolume",
            &[js("music"), get(&self.levels, "music")],
        );
        call(&self.audio, "setFightMusic", &[get(&self.levels, "fight")]);
        self.amb_level();
    }
    fn radio_list(&mut self) {
        self.list = get(&self.host, "RADIO_PLAYLIST");
        self.spot = (0.0, 0.4);
        call_optional(&self.audio, "placeRadio", &[number(0.0), number(0.4)]);
        call_optional(&get(&self.audio, "music"), "playlist", &[self.list.clone()]);
    }
}
fn guns(m: &JsValue, side: u32) -> JsValue {
    let kit = get(
        &Array::from(&get(&get(m, "config"), "fighters")).get(side),
        "kit",
    );
    if exists(&get(&kit, "sidearm")) {
        args(&[
            get(&get(&kit, "weapon"), "id"),
            get(&get(&kit, "sidearm"), "id"),
        ])
        .into()
    } else {
        args(&[get(&Array::from(&get(m, "fighters")).get(side), "weapon")]).into()
    }
}
