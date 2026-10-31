//! Web Audio graph, cached decoding, loop crossfades and cancellable music decks.
use super::dom_support::*;
use js_sys::{Array, Object, Promise};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::{closure::Closure, prelude::*};
use wasm_bindgen_futures::{JsFuture, future_to_promise, spawn_local};

const MUSIC_FADE: f64 = 0.4;
struct Timer {
    id: JsValue,
    _callback: Closure<dyn FnMut()>,
}
impl Timer {
    fn new(delay: f64, callback: impl FnMut() + 'static) -> Self {
        let callback = Closure::wrap(Box::new(callback) as Box<dyn FnMut()>);
        let id = call(
            &global(),
            "setTimeout",
            &[callback.as_ref().clone(), number(delay)],
        );
        Self {
            id,
            _callback: callback,
        }
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        call(&global(), "clearTimeout", &[self.id.clone()]);
    }
}
fn gain(ctx: &JsValue, db: f64, to: &JsValue) -> JsValue {
    let g = call(ctx, "createGain", &[]);
    set(
        &get(&g, "gain"),
        "value",
        &rust("audio.dsp.dbToGain", &[number(db)]),
    );
    call(&g, "connect", &[to.clone()]);
    g
}
fn time(ctx: &JsValue) -> f64 {
    num(ctx, "currentTime")
}
fn url(base: &str, file: &str) -> String {
    format!("{}/{file}", base.strip_suffix('/').unwrap_or(base))
}
fn ignore(promise: JsValue) {
    spawn_local(async move {
        let _ = JsFuture::from(Promise::resolve(&promise)).await;
    });
}
fn decode(ctx: JsValue, base: String, file: String) -> Promise {
    future_to_promise(async move {
        let response = JsFuture::from(Promise::resolve(&invoke(
            &global(),
            "fetch",
            &[js(&url(&base, &file))],
        )?))
        .await?;
        if !boolean(&response, "ok") {
            return Err(
                js_sys::Error::new(&format!("{file}: HTTP {}", num(&response, "status"))).into(),
            );
        }
        let bytes =
            JsFuture::from(Promise::resolve(&invoke(&response, "arrayBuffer", &[])?)).await?;
        JsFuture::from(Promise::resolve(&invoke(
            &ctx,
            "decodeAudioData",
            &[bytes],
        )?))
        .await
    })
}

struct Channel {
    fade: JsValue,
    level: JsValue,
    base: f64,
    source: JsValue,
    buffer: JsValue,
    track: Option<String>,
    at: f64,
    offset: f64,
    playing: bool,
    serial: u32,
    pending: Option<Timer>,
    ended: Option<Closure<dyn FnMut()>>,
}
impl Channel {
    fn new(ctx: &JsValue, base: f64, to: &JsValue) -> Self {
        let fade = call(ctx, "createGain", &[]);
        set(&get(&fade, "gain"), "value", &number(0.0));
        let level = gain(ctx, base, to);
        call(&fade, "connect", &[level.clone()]);
        Self {
            fade,
            level,
            base,
            source: JsValue::NULL,
            buffer: JsValue::NULL,
            track: None,
            at: 0.0,
            offset: 0.0,
            playing: false,
            serial: 0,
            pending: None,
            ended: None,
        }
    }
    fn halt(&mut self) {
        let source = std::mem::replace(&mut self.source, JsValue::NULL);
        if exists(&source) {
            set(&source, "onended", &JsValue::NULL);
            let _ = invoke(&source, "stop", &[]);
            call(&source, "disconnect", &[]);
        }
        self.ended = None;
    }
    fn fade_to(&self, ctx: &JsValue, value: f64) {
        let gain = get(&self.fade, "gain");
        let now = time(ctx);
        call(&gain, "cancelScheduledValues", &[number(now)]);
        call(&gain, "setValueAtTime", &[get(&gain, "value"), number(now)]);
        call(
            &gain,
            "linearRampToValueAtTime",
            &[number(value), number(now + MUSIC_FADE)],
        );
    }
    fn position(&self, ctx: &JsValue) -> f64 {
        if exists(&self.source) {
            time(ctx) - self.at
        } else {
            self.offset
        }
    }
}
struct Loop {
    ctx: JsValue,
    level: JsValue,
    stopped: bool,
    timer: Option<Timer>,
}
impl Loop {
    fn new(ctx: JsValue, out: JsValue) -> Rc<RefCell<Self>> {
        let level = call(&ctx, "createGain", &[]);
        set(&get(&level, "gain"), "value", &number(0.0));
        call(&level, "connect", &[out]);
        Rc::new(RefCell::new(Self {
            ctx,
            level,
            stopped: false,
            timer: None,
        }))
    }
    fn start(state: &Rc<RefCell<Self>>, buffer: JsValue, fade: f64) {
        let s = state.borrow();
        if s.stopped {
            return;
        }
        let now = time(&s.ctx);
        call(
            &get(&s.level, "gain"),
            "setTargetAtTime",
            &[number(1.0), number(now), number(fade / 3.0)],
        );
        drop(s);
        Self::pass(state, buffer, now, true);
    }
    fn stop(&mut self, fade: f64) {
        self.stopped = true;
        self.timer = None;
        let now = time(&self.ctx);
        let parameter = get(&self.level, "gain");
        call(&parameter, "cancelScheduledValues", &[number(now)]);
        call(
            &parameter,
            "setTargetAtTime",
            &[number(0.0), number(now), number(fade / 3.0)],
        );
        let level = self.level.clone();
        let callback = Closure::once_into_js(move || {
            call(&level, "disconnect", &[]);
        });
        call(&global(), "setTimeout", &[callback, number(fade * 1500.0)]);
    }
    fn pass(state: &Rc<RefCell<Self>>, buffer: JsValue, at: f64, first: bool) {
        let mut s = state.borrow_mut();
        if s.stopped {
            return;
        }
        let source = call(&s.ctx, "createBufferSource", &[]);
        set(&source, "buffer", &buffer);
        let g = call(&s.ctx, "createGain", &[]);
        call(&source, "connect", &[g.clone()]);
        call(&g, "connect", &[s.level.clone()]);
        let schedule = rust(
            "audio.dsp.loop",
            &[number(at), get(&buffer, "duration"), number(time(&s.ctx))],
        );
        let parameter = get(&g, "gain");
        call(
            &parameter,
            "setValueAtTime",
            &[number(if first { 1.0 } else { 0.0 }), number(at)],
        );
        if !first {
            call(
                &parameter,
                "linearRampToValueAtTime",
                &[number(1.0), get(&schedule, "fadeInEnd")],
            );
        }
        call(
            &parameter,
            "setValueAtTime",
            &[number(1.0), get(&schedule, "fadeOutStart")],
        );
        call(
            &parameter,
            "linearRampToValueAtTime",
            &[number(0.0), get(&schedule, "end")],
        );
        call(&source, "start", &[number(at)]);
        call(&source, "stop", &[get(&schedule, "stopAt")]);
        let next = num(&schedule, "next");
        let weak = Rc::downgrade(state);
        s.timer = Some(Timer::new(num(&schedule, "delay"), move || {
            if let Some(state) = weak.upgrade() {
                Self::pass(&state, buffer.clone(), next, false);
            }
        }));
    }
}
struct Audio {
    ctx: JsValue,
    mix: JsValue,
    sounds: JsValue,
    files: JsValue,
    tracks: JsValue,
    base: String,
    random: JsValue,
    buses: HashMap<String, JsValue>,
    duck: JsValue,
    buffers: HashMap<String, Promise>,
    last: HashMap<String, u32>,
    loops: HashMap<String, Rc<RefCell<Loop>>>,
    music: JsValue,
    radio: JsValue,
    channels: HashMap<String, Channel>,
    /// Shared room reverb: cues send into `hall`, its tail returns through `hall_return` to the sfx bus.
    hall: JsValue,
    hall_return: JsValue,
    callbacks: Vec<Closure<dyn FnMut(JsValue, JsValue)>>,
    keep_alive: JsValue,
    /// The instructor line on air. A newer line or stopVoice cuts it; `voice_turn` drops a
    /// line that is still decoding when a newer one was asked for.
    voice: JsValue,
    voice_turn: u32,
}
impl Audio {
    fn random(&self) -> f64 {
        call(&self.random, "call", &[JsValue::UNDEFINED])
            .as_f64()
            .unwrap_or(0.0)
    }
    fn jitter(&self, spread: JsValue) -> JsValue {
        let random = if spread.as_f64().unwrap_or(0.0) != 0.0 {
            self.random()
        } else {
            0.0
        };
        rust("audio.jitter", &[spread, number(random)])
    }
    fn buffer(state: &Rc<RefCell<Self>>, file: &str) -> Promise {
        let mut s = state.borrow_mut();
        if let Some(promise) = s.buffers.get(file) {
            return promise.clone();
        }
        let promise = decode(s.ctx.clone(), s.base.clone(), file.into());
        s.buffers.insert(file.into(), promise.clone());
        let weak = Rc::downgrade(state);
        let name = file.to_owned();
        let task = promise.clone();
        spawn_local(async move {
            if JsFuture::from(task.clone()).await.is_err() {
                if let Some(state) = weak.upgrade() {
                    let mut s = state.borrow_mut();
                    if s.buffers
                        .get(&name)
                        .is_some_and(|cached| Object::is(cached.as_ref(), task.as_ref()))
                    {
                        s.buffers.remove(&name);
                    }
                }
            }
        });
        promise
    }
    fn play(state: &Rc<RefCell<Self>>, id: &str, options: JsValue) {
        let (file, when, voice, turn) = {
            let mut s = state.borrow_mut();
            let definition = get(&s.sounds, id);
            let voice = boolean(&definition, "voice");
            if voice {
                s.voice_turn += 1;
            }
            let files = Array::from(&get(&definition, "files"));
            let random = if files.length() > 1 { s.random() } else { 0.0 };
            let previous = s
                .last
                .get(id)
                .map(|n| number(*n as f64))
                .unwrap_or(JsValue::UNDEFINED);
            let index = rust(
                "audio.pickOther",
                &[number(files.length() as f64), previous, number(random)],
            )
            .as_f64()
            .unwrap_or(0.0) as u32;
            s.last.insert(id.into(), index);
            (
                text(&files.get(index)),
                time(&s.ctx) + num(&options, "delay"),
                voice,
                s.voice_turn,
            )
        };
        let promise = Self::buffer(state, &file);
        let weak = Rc::downgrade(state);
        let id = id.to_owned();
        spawn_local(async move {
            let Ok(buffer) = JsFuture::from(promise).await else {
                return;
            };
            let Some(state) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if voice && s.voice_turn != turn {
                return;
            }
            let definition = get(&s.sounds, &id);
            let source = call(&s.ctx, "createBufferSource", &[]);
            set(&source, "buffer", &buffer);
            let options = if exists(&options) {
                options
            } else {
                Object::new().into()
            };
            let levels = rust(
                "audio.dsp.play",
                &[
                    js(&id),
                    number(num(&get(&s.files, &file), "trimDb")),
                    options.clone(),
                    s.jitter(get(&definition, "pitchJitter")),
                    s.jitter(get(&definition, "gainJitter")),
                ],
            );
            set(
                &get(&source, "playbackRate"),
                "value",
                &get(&levels, "playbackRate"),
            );
            let out = s.buses.get(&text(&get(&definition, "bus"))).unwrap();
            let g = gain(&s.ctx, num(&levels, "gainDb"), out);
            let wet = get(&options, "wetDb");
            if wet.as_f64().is_some() && exists(&s.hall) {
                let send = gain(&s.ctx, num(&options, "wetDb"), &s.hall);
                call(&g, "connect", &[send]);
            }
            let pan = num(&options, "pan");
            let mut head = if pan != 0.0 {
                let panner = call(&s.ctx, "createStereoPanner", &[]);
                set(&get(&panner, "pan"), "value", &number(pan));
                call(&panner, "connect", &[g]);
                panner
            } else {
                g
            };
            let cut = num(&options, "cutHz");
            let filter = if cut > 0.0 {
                invoke(&s.ctx, "createBiquadFilter", &[]).unwrap_or(JsValue::NULL)
            } else {
                JsValue::NULL
            };
            if exists(&filter) {
                set(&filter, "type", &js("lowpass"));
                set(&get(&filter, "frequency"), "value", &number(cut));
                call(&filter, "connect", &[head]);
                head = filter;
            }
            call(&source, "connect", &[head]);
            let at = when.max(time(&s.ctx));
            call(&source, "start", &[number(at)]);
            if voice {
                s.cut_voice();
                s.voice = source;
            }
            let duck = num(&definition, "duckDb");
            if duck != 0.0 {
                // A voice line keeps the music down until it ends.
                let hold = if voice { num(&buffer, "duration") } else { 0.0 };
                s.duck(duck, at, hold);
            }
        });
    }
    fn cut_voice(&mut self) {
        let voice = std::mem::replace(&mut self.voice, JsValue::NULL);
        if exists(&voice) {
            let _ = invoke(&voice, "stop", &[]);
            let gain = get(&self.duck, "gain");
            let now = time(&self.ctx);
            call(&gain, "cancelScheduledValues", &[number(now)]);
            call(&gain, "setTargetAtTime", &[number(1.0), number(now), number(0.4)]);
        }
    }
    fn duck(&self, db: f64, at: f64, hold: f64) {
        let gain = get(&self.duck, "gain");
        call(&gain, "cancelScheduledValues", &[number(at)]);
        call(
            &gain,
            "setTargetAtTime",
            &[
                rust("audio.dsp.dbToGain", &[number(-db)]),
                number(at),
                number(0.02),
            ],
        );
        call(
            &gain,
            "setTargetAtTime",
            &[number(1.0), number(at + hold.max(0.25)), number(0.4)],
        );
    }
    fn deck_play(state: &Rc<RefCell<Self>>, channel: &str, track: &str) {
        let (ctx, base, file, serial) = {
            let mut s = state.borrow_mut();
            let ctx = s.ctx.clone();
            let base = s.base.clone();
            let file = text(&get(&get(&s.tracks, track), "file"));
            let radio = s.radio.clone();
            let c = s.channels.get_mut(channel).unwrap();
            c.pending = None;
            c.halt();
            c.serial += 1;
            c.track = Some(track.into());
            c.offset = 0.0;
            c.playing = true;
            c.buffer = JsValue::NULL;
            c.fade_to(&ctx, 1.0);
            if channel == "radio" {
                call(&radio, "setHiss", &[flag(true)]);
            }
            (ctx, base, file, c.serial)
        };
        let promise = decode(ctx, base, file);
        let weak = Rc::downgrade(state);
        let channel = channel.to_owned();
        spawn_local(async move {
            let Ok(buffer) = JsFuture::from(promise).await else {
                return;
            };
            let Some(state) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let c = s.channels.get_mut(&channel).unwrap();
            if c.serial != serial || !c.playing {
                if c.serial == serial {
                    c.buffer = buffer;
                }
                return;
            }
            c.buffer = buffer;
            drop(s);
            Self::start(state, &channel);
        });
    }
    fn start(state: Rc<RefCell<Self>>, channel: &str) {
        let mut s = state.borrow_mut();
        let ctx = s.ctx.clone();
        let c = s.channels.get_mut(channel).unwrap();
        if !exists(&c.buffer) {
            return;
        }
        let source = call(&ctx, "createBufferSource", &[]);
        set(&source, "buffer", &c.buffer);
        call(&source, "connect", &[c.fade.clone()]);
        let offset = if c.offset < num(&c.buffer, "duration") {
            c.offset
        } else {
            0.0
        };
        call(&source, "start", &[number(0.0), number(offset)]);
        c.at = time(&ctx) - offset;
        c.source = source.clone();
        let weak = Rc::downgrade(&state);
        let name = channel.to_owned();
        let ended = Closure::wrap(Box::new(move || {
            let Some(state) = weak.upgrade() else {
                return;
            };
            let music = {
                let mut s = state.borrow_mut();
                let c = s.channels.get_mut(&name).unwrap();
                if c.source != source || !c.playing {
                    return;
                }
                c.source = JsValue::NULL;
                s.music.clone()
            };
            call(&music, "ended", &[js(&name)]);
        }) as Box<dyn FnMut()>);
        set(&c.source, "onended", ended.as_ref());
        c.ended = Some(ended);
    }
    fn deck_pause(state: &Rc<RefCell<Self>>, channel: &str, stop: bool) {
        let mut s = state.borrow_mut();
        let ctx = s.ctx.clone();
        let radio = s.radio.clone();
        let c = s.channels.get_mut(channel).unwrap();
        c.fade_to(&ctx, 0.0);
        c.pending = None;
        if stop {
            c.playing = false;
            c.serial += 1;
        } else if c.playing {
            c.offset = c.position(&ctx);
            c.playing = false;
        }
        let weak = Rc::downgrade(state);
        let name = channel.to_owned();
        c.pending = Some(Timer::new(MUSIC_FADE * 1000.0 + 50.0, move || {
            if let Some(state) = weak.upgrade() {
                let mut s = state.borrow_mut();
                let c = s.channels.get_mut(&name).unwrap();
                c.halt();
                if stop {
                    c.buffer = JsValue::NULL;
                    c.track = None;
                    c.offset = 0.0;
                }
            }
        }));
        if channel == "radio" {
            call(&radio, "setHiss", &[flag(false)]);
        }
    }
    fn deck_resume(state: &Rc<RefCell<Self>>, channel: &str) {
        let restart = {
            let mut s = state.borrow_mut();
            let c = s.channels.get_mut(channel).unwrap();
            c.pending = None;
            let Some(track) = c.track.clone() else {
                return;
            };
            if !c.playing {
                c.playing = true;
                c.halt();
                Some((track, exists(&c.buffer)))
            } else {
                None
            }
        };
        if let Some((track, buffered)) = restart {
            if buffered {
                Self::start(state.clone(), channel);
            } else {
                Self::deck_play(state, channel, &track);
            }
        }
        let s = state.borrow();
        s.channels.get(channel).unwrap().fade_to(&s.ctx, 1.0);
        if channel == "radio" {
            call(&s.radio, "setHiss", &[flag(true)]);
        }
    }
}

#[wasm_bindgen]
pub struct BrowserGameAudio {
    state: Rc<RefCell<Audio>>,
}
#[wasm_bindgen]
impl BrowserGameAudio {
    #[wasm_bindgen(constructor)]
    pub fn new(options: JsValue, host: JsValue) -> Result<Self, JsValue> {
        let config = Object::new();
        set(&config, "latencyHint", &js("interactive"));
        let ctx = construct(&global(), "AudioContext", &[config.into()])?;
        let mix = get(&host, "MIX");
        let master = gain(&ctx, num(&mix, "master"), &get(&ctx, "destination"));
        let duck = gain(&ctx, 0.0, &master);
        let mut buses = HashMap::new();
        buses.insert("master".into(), master.clone());
        for bus in ["sfx", "ui", "amb", "music"] {
            buses.insert(
                bus.into(),
                gain(
                    &ctx,
                    num(&mix, bus),
                    if bus == "music" { &duck } else { &master },
                ),
            );
        }
        // Optional: a context without convolution (old engines, test fakes) just plays dry.
        let hall = invoke(&ctx, "createConvolver", &[]).unwrap_or(JsValue::NULL);
        let hall_return = if exists(&hall) {
            let level = gain(&ctx, -60.0, buses.get("sfx").unwrap());
            call(&hall, "connect", &[level.clone()]);
            level
        } else {
            JsValue::NULL
        };
        let radio = call(&host, "createRadio", &[ctx.clone()]);
        let music_bus = buses.get("music").unwrap();
        call(&get(&radio, "output"), "connect", &[music_bus.clone()]);
        let mut channels = HashMap::new();
        channels.insert(
            "fight".into(),
            Channel::new(&ctx, num(&mix, "fightMusic"), music_bus),
        );
        channels.insert(
            "radio".into(),
            Channel::new(&ctx, num(&mix, "radioMusic"), &get(&radio, "input")),
        );
        let random = get(&options, "random");
        let random = if random.is_function() {
            random
        } else {
            get(&get(&global(), "Math"), "random")
        };
        let state = Rc::new(RefCell::new(Audio {
            ctx,
            mix,
            sounds: get(&host, "SOUNDS"),
            files: get(&host, "FILES"),
            tracks: get(&host, "TRACKS"),
            base: text(&get(&options, "baseUrl")),
            random: random.clone(),
            buses,
            duck,
            buffers: HashMap::new(),
            last: HashMap::new(),
            loops: HashMap::new(),
            music: JsValue::NULL,
            radio,
            channels,
            hall,
            hall_return,
            callbacks: vec![],
            keep_alive: JsValue::NULL,
            voice: JsValue::NULL,
            voice_turn: 0,
        }));
        let deck = Object::new();
        for method in ["play", "pause", "resume", "stop", "cue"] {
            let weak = Rc::downgrade(&state);
            let callback = Closure::wrap(Box::new(move |channel: JsValue, track: JsValue| {
                let Some(state) = weak.upgrade() else {
                    return;
                };
                let channel = text(&channel);
                match method {
                    "play" => Audio::deck_play(&state, &channel, &text(&track)),
                    "pause" => Audio::deck_pause(&state, &channel, false),
                    "resume" => Audio::deck_resume(&state, &channel),
                    "stop" => Audio::deck_pause(&state, &channel, true),
                    "cue" => Audio::play(&state, &channel, JsValue::UNDEFINED),
                    _ => {}
                }
            }) as Box<dyn FnMut(JsValue, JsValue)>);
            set(&deck, method, callback.as_ref());
            state.borrow_mut().callbacks.push(callback);
        }
        let music = construct(&host, "MusicDirector", &[deck.into(), random])?;
        state.borrow_mut().music = music;
        Ok(Self { state })
    }
    #[wasm_bindgen(getter)]
    pub fn ctx(&self) -> JsValue {
        self.state.borrow().ctx.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn music(&self) -> JsValue {
        self.state.borrow().music.clone()
    }
    pub fn unlock(&self) -> Promise {
        let mut s = self.state.borrow_mut();
        let session = get(&get(&global(), "navigator"), "audioSession");
        if exists(&session) {
            let _ = js_sys::Reflect::set(&session, &js("type"), &js("playback"));
        }
        if !exists(&s.keep_alive) {
            let silence = silence();
            if let Ok(audio) = construct(&global(), "Audio", &[js(&silence)]) {
                set(&audio, "loop", &flag(true));
                call(&audio, "setAttribute", &[js("playsinline"), js("")]);
                s.keep_alive = audio;
            }
        }
        if exists(&s.keep_alive) {
            if let Ok(promise) = invoke(&s.keep_alive, "play", &[]) {
                ignore(promise);
            }
        }
        let ctx = s.ctx.clone();
        drop(s);
        future_to_promise(async move {
            if text(&get(&ctx, "state")) != "running" {
                if let Ok(promise) = invoke(&ctx, "resume", &[]) {
                    let _ = JsFuture::from(Promise::resolve(&promise)).await;
                }
            }
            Ok(JsValue::UNDEFINED)
        })
    }
    #[wasm_bindgen(js_name=setVolume)]
    pub fn set_volume(&self, bus: &str, value: f64) {
        let s = self.state.borrow();
        let level = rust("audio.dsp.volume", &[get(&s.mix, bus), number(value)]);
        if let Some(bus) = s.buses.get(bus) {
            call(
                &get(bus, "gain"),
                "setTargetAtTime",
                &[level, number(time(&s.ctx)), number(0.03)],
            );
        }
    }
    pub fn url(&self, file: &str) -> String {
        url(&self.state.borrow().base, file)
    }
    pub fn preload(&self, ids: JsValue) -> Promise {
        let promises = Array::new();
        for id in Array::from(&ids).iter() {
            let files = get(&get(&self.state.borrow().sounds, &text(&id)), "files");
            for file in Array::from(&files).iter() {
                promises.push(&Audio::buffer(&self.state, &text(&file)));
            }
        }
        let promise = Promise::all(&promises);
        future_to_promise(async move {
            JsFuture::from(promise).await?;
            Ok(JsValue::UNDEFINED)
        })
    }
    pub fn play(&self, id: &str, options: JsValue) {
        Audio::play(&self.state, id, options);
    }
    #[wasm_bindgen(js_name=stopVoice)]
    pub fn stop_voice(&self) {
        let mut s = self.state.borrow_mut();
        s.voice_turn += 1;
        s.cut_voice();
    }
    #[wasm_bindgen(js_name=playCues)]
    pub fn play_cues(&self, cues: JsValue, start_in: f64) {
        for cue in Array::from(&cues).iter() {
            let options = Object::new();
            set(&options, "delay", &number(start_in + num(&cue, "at")));
            set(&options, "gainDb", &get(&cue, "gainDb"));
            for key in ["pan", "wetDb", "cutHz"] {
                set(&options, key, &get(&cue, key));
            }
            Audio::play(&self.state, &text(&get(&cue, "sound")), options.into());
        }
    }
    #[wasm_bindgen(js_name=setAmbience)]
    pub fn set_ambience(&self, ids: JsValue, fade: f64) {
        let ids: Vec<_> = Array::from(&ids).iter().map(|id| text(&id)).collect();
        let removed: Vec<_> = self
            .state
            .borrow()
            .loops
            .keys()
            .filter(|id| !ids.contains(id))
            .cloned()
            .collect();
        for id in removed {
            if let Some(loop_state) = self.state.borrow_mut().loops.remove(&id) {
                loop_state.borrow_mut().stop(fade);
            }
        }
        for id in ids {
            if self.state.borrow().loops.contains_key(&id) {
                continue;
            }
            let (file, loop_state) = {
                let mut s = self.state.borrow_mut();
                let definition = get(&s.sounds, &id);
                let file = text(&Array::from(&get(&definition, "files")).get(0));
                let out = gain(
                    &s.ctx,
                    num(&definition, "gainDb"),
                    s.buses.get(&text(&get(&definition, "bus"))).unwrap(),
                );
                let loop_state = Loop::new(s.ctx.clone(), out);
                s.loops.insert(id, loop_state.clone());
                (file, loop_state)
            };
            let promise = Audio::buffer(&self.state, &file);
            spawn_local(async move {
                if let Ok(buffer) = JsFuture::from(promise).await {
                    Loop::start(&loop_state, buffer, fade);
                }
            });
        }
    }
    #[wasm_bindgen(js_name=setFightMusic)]
    pub fn set_fight_music(&self, value: f64) {
        let s = self.state.borrow();
        let channel = s.channels.get("fight").unwrap();
        call(
            &get(&channel.level, "gain"),
            "setTargetAtTime",
            &[
                rust("audio.dsp.volume", &[number(channel.base), number(value)]),
                number(time(&s.ctx)),
                number(0.05),
            ],
        );
    }
    /// Picks the fight room ("hall", "blocks" or "off"): reverb length and how loud its tail returns.
    #[wasm_bindgen(js_name=setRoom)]
    pub fn set_room(&self, room: &str) {
        let s = self.state.borrow();
        if !exists(&s.hall) {
            return;
        }
        let preset = rust("audio.dsp.room", &[js(room)]);
        set(
            &s.hall,
            "buffer",
            &super::radio::buffer(&s.ctx, "room", num(&preset, "seconds")),
        );
        call(
            &get(&s.hall_return, "gain"),
            "setTargetAtTime",
            &[
                rust("audio.dsp.dbToGain", &[get(&preset, "returnDb")]),
                number(time(&s.ctx)),
                number(0.2),
            ],
        );
    }
    #[wasm_bindgen(js_name=placeRadio)]
    pub fn place_radio(&self, pan: f64, far: f64) {
        call(
            &self.state.borrow().radio,
            "place",
            &[number(pan), number(far)],
        );
    }
    #[wasm_bindgen(js_name=musicBus)]
    pub fn music_bus(&self) -> JsValue {
        self.state.borrow().buses.get("music").unwrap().clone()
    }
    #[wasm_bindgen(js_name=gainNode)]
    pub fn gain_node(&self, db: f64, to: JsValue) -> JsValue {
        gain(&self.state.borrow().ctx, db, &to)
    }
    pub fn decode(&self, file: &str) -> Promise {
        let s = self.state.borrow();
        decode(s.ctx.clone(), s.base.clone(), file.into())
    }
}
fn silence() -> String {
    let mut bytes = vec![128u8; 4044];
    bytes[..44].fill(0);
    bytes[0..4].copy_from_slice(b"RIFF");
    bytes[4..8].copy_from_slice(&4036u32.to_le_bytes());
    bytes[8..16].copy_from_slice(b"WAVEfmt ");
    bytes[16..20].copy_from_slice(&16u32.to_le_bytes());
    bytes[20..22].copy_from_slice(&1u16.to_le_bytes());
    bytes[22..24].copy_from_slice(&1u16.to_le_bytes());
    bytes[24..28].copy_from_slice(&8000u32.to_le_bytes());
    bytes[28..32].copy_from_slice(&8000u32.to_le_bytes());
    bytes[32..34].copy_from_slice(&1u16.to_le_bytes());
    bytes[34..36].copy_from_slice(&8u16.to_le_bytes());
    bytes[36..40].copy_from_slice(b"data");
    bytes[40..44].copy_from_slice(&4000u32.to_le_bytes());
    let data: String = bytes.into_iter().map(char::from).collect();
    format!(
        "data:audio/wav;base64,{}",
        text(&call(&global(), "btoa", &[js(&data)]))
    )
}
