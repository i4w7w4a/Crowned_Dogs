//! Settings panel state and interaction live beside the game controllers.
use super::dom_support::*;
use js_sys::Array;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;

struct Settings {
    host: JsValue,
    dialog: JsValue,
    listeners: Vec<Listener>,
}
thread_local! { static SETTINGS:RefCell<Option<Rc<RefCell<Settings>>>>=const{RefCell::new(None)}; }
thread_local! { static GPU: RefCell<Option<String>> = const { RefCell::new(None) }; }
/// Whether this browser hands out a WebGPU adapter, and whose. WebGPU lives only on secure
/// pages, so a phone on the home-network address (plain http) never sees it.
async fn webgpu() -> String {
    let gpu = get(&get(&global(), "navigator"), "gpu");
    if !exists(&gpu) {
        return if boolean(&global(), "isSecureContext") { "нет" } else { "нет на http" }.to_owned();
    }
    let Ok(asked) = invoke(&gpu, "requestAdapter", &[]) else {
        return "нет".to_owned();
    };
    match wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(asked)).await {
        Ok(adapter) if exists(&adapter) => {
            let info = get(&adapter, "info");
            let about: Vec<String> = ["vendor", "architecture"]
                .iter()
                .map(|key| text(&get(&info, key)))
                .filter(|part| !part.is_empty())
                .collect();
            if about.is_empty() {
                "есть".to_owned()
            } else {
                format!("есть ({})", about.join(" "))
            }
        }
        _ => "нет".to_owned(),
    }
}
impl Settings {
    fn draw(&self) {
        let audio = get(&self.host, "gameAudio");
        let on = boolean(&audio, "on");
        let toggle = query(&self.dialog, ".cd-sound-toggle");
        content(&query(&toggle, "b"), if on { "Вкл" } else { "Выкл" });
        attr(&toggle, "aria-pressed", &on.to_string());
        for input in all(&self.dialog, "input[data-k]") {
            let volume = num(&get(&audio, "volume"), &data(&input, "k")) * 100.0;
            let v = volume.round().to_string();
            set(&input, "value", &js(&v));
            style(&input, "--v", &format!("{v}%"));
            content(&get(&input, "nextElementSibling"), &v);
        }
        let meter = super::fps_meter::on();
        let fps = query(&self.dialog, ".cd-fps-toggle");
        content(&query(&fps, "b"), if meter { "Вкл" } else { "Выкл" });
        attr(&fps, "aria-pressed", &meter.to_string());
        self.gpu();
        let quality = get(&self.host, "QUALITY");
        let ids = Array::from(&get(&self.host, "QUALITY_IDS"));
        let choices = ids
            .iter()
            .map(|id| {
                let id = text(&id);
                let name = text(&get(&get(&quality, &id), "name"));
                (id, name)
            })
            .collect();
        self.segments(
            "gfx",
            choices,
            &text(&call(&get(&self.host, "store"), "get", &[js("cd-quality")])),
        );
        let restart = query(&self.dialog, ".cd-restart");
        set(&get(&restart, "dataset"), "armed", &js(""));
        content(&query(&restart, "b"), "");
    }
    /// The first press arms the button; the second wipes progress (Persona, kits, gun mods)
    /// and opens the game from the start. Sound, camera and graphics settings stay.
    fn restart(&self) {
        let button = query(&self.dialog, ".cd-restart");
        if data(&button, "armed") != "1" {
            set(&get(&button, "dataset"), "armed", &js("1"));
            content(&query(&button, "b"), "Нажми ещё раз");
            return;
        }
        let storage = get(&global(), "localStorage");
        if exists(&storage) {
            for key in js_sys::Object::keys(&storage.clone().into()).iter() {
                let key = text(&key);
                if key.starts_with("crowned-dogs:") || key == "cd-weapon" {
                    call(&storage, "removeItem", &[js(&key)]);
                }
            }
        }
        let location = get(&global(), "location");
        set(&location, "hash", &js(""));
        call(&location, "reload", &[]);
    }
    /// Under the frame counter: the graphics card and WebGPU, so a phone's numbers come with
    /// its hardware. Asked once per page.
    fn gpu(&self) {
        let line = query(&self.dialog, ".cd-gpu");
        if let Some(known) = GPU.with(|g| g.borrow().clone()) {
            content(&line, &known);
            return;
        }
        let name = super::platform::gpu_name();
        let first = format!("Видеокарта: {name}. WebGPU: проверяю…");
        GPU.with(|g| *g.borrow_mut() = Some(first.clone()));
        content(&line, &first);
        wasm_bindgen_futures::spawn_local(async move {
            let known = format!("Видеокарта: {name}. WebGPU: {}", webgpu().await);
            GPU.with(|g| *g.borrow_mut() = Some(known.clone()));
            content(&line, &known);
        });
    }
    fn segments(&self, group: &str, list: Vec<(String, String)>, current: &str) {
        let element = query(&self.dialog, &format!("[data-g={group}]"));
        call(&element, "replaceChildren", &[]);
        for (id, name) in list {
            let button = create("button", "glass");
            set(&button, "type", &js("button"));
            content(&button, &name);
            attr(&button, "aria-pressed", &(id == current).to_string());
            set(&get(&button, "dataset"), "choice", &js(&id));
            call(&element, "append", &[button]);
        }
    }
}
#[wasm_bindgen]
pub fn mount_sound_settings(host: JsValue) {
    SETTINGS.with(|holder|{if holder.borrow().is_some(){return;}
        let button=create("button","glass cd-settings");set(&button,"type",&js("button"));attr(&button,"aria-label","Настройки звука");html(&button,"<svg class=\"cd-set-ico\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M4 7h16M4 17h16\"/><circle fill=\"currentColor\" stroke=\"none\" cx=\"15.5\" cy=\"7\" r=\"2.6\"/><circle fill=\"currentColor\" stroke=\"none\" cx=\"8.5\" cy=\"17\" r=\"2.6\"/></svg>");
        let dialog=create("dialog","cd-dialog sheet glass cd-sound");html(&dialog,include_str!("sound-settings.html"));call(&get(&document(),"body"),"append",&[button.clone(),dialog.clone()]);
        let state=Rc::new(RefCell::new(Settings{host,dialog:dialog.clone(),listeners:vec![]}));
        for (target,event,action) in [(button,"click","open"),(query(&dialog,".cd-sound-toggle:not(.cd-restart)"),"click","toggle"),(query(&dialog,".cd-restart"),"click","restart"),(query(&dialog,".cd-fps-toggle"),"click","fps"),(query(&dialog,".cd-dialog-close"),"click","close"),(dialog.clone(),"click","click"),(dialog,"input","volume")] {
            let weak=Rc::downgrade(&state);let listener=Listener::new(&target,event,move |event|{let Some(state)=weak.upgrade()else{return;};let state=state.borrow();let audio=get(&state.host,"gameAudio");
                match action{
                    "open"=>{call(&audio,"unlock",&[]);state.draw();call(&state.dialog,"showModal",&[]);call(&state.host,"liquidGlass",&[state.dialog.clone()]);},
                    "toggle"=>{call(&audio,"toggle",&[]);state.draw();},"restart"=>state.restart(),"fps"=>{super::fps_meter::set_on(!super::fps_meter::on());state.draw();},
                    "close"=>{call(&state.dialog,"close",&[]);},
                    "volume"=>{let input=closest(&event,"input[data-k]");if exists(&input){call(&audio,"unlock",&[]);call(&audio,"setVolume",&[js(&data(&input,"k")),number(text(&get(&input,"value")).parse::<f64>().unwrap_or(0.0)/100.0)]);state.draw();}},
                    "click"=>{if get(&event,"target")==state.dialog{call(&state.dialog,"close",&[]);return;}let choice=closest(&event,"[data-choice]");if !exists(&choice){return;}let id=js(&data(&choice,"choice"));call(&state.host,"keepQuality",&[id]);state.draw();},_=>{}
                }
            });state.borrow_mut().listeners.push(listener);
        }
        *holder.borrow_mut()=Some(state);
    });
}
#[wasm_bindgen]
pub fn show_sound_settings(on: bool) {
    let button = query(&document(), ".cd-settings");
    if !exists(&button) {
        return;
    }
    hidden(&button, !on);
    let home = query(&document(), ".cd-home");
    // first show only: later the base moves it between home and creation (base.rs show)
    if on && exists(&home) && get(&button, "parentElement") == get(&document(), "body") {
        call(&home, "append", &[button]);
    }
}
