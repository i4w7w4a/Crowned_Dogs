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
        let store = get(&self.host, "store");
        let camera = call(&store, "get", &[js("cd-camera")]);
        let camera = if text(&camera) == "back" {
            "back"
        } else {
            "fp"
        };
        let cameras = [("fp", "От первого лица"), ("back", "Из-за плеча")];
        self.segments(
            "cam",
            cameras
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
            camera,
        );
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
            &text(&call(&store, "get", &[js("cd-quality")])),
        );
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
        let button=create("button","glass cd-settings");set(&button,"type",&js("button"));attr(&button,"aria-label","Настройки звука");html(&button,"<svg class=\"cd-set-ico\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M4 7h10M18 7h2M4 17h4M12 17h8\"/><circle cx=\"16\" cy=\"7\" r=\"2.2\"/><circle cx=\"10\" cy=\"17\" r=\"2.2\"/></svg>");
        let dialog=create("dialog","cd-dialog sheet glass cd-sound");html(&dialog,include_str!("sound-settings.html"));call(&get(&document(),"body"),"append",&[button.clone(),dialog.clone()]);
        let state=Rc::new(RefCell::new(Settings{host,dialog:dialog.clone(),listeners:vec![]}));
        for (target,event,action) in [(button,"click","open"),(query(&dialog,".cd-sound-toggle"),"click","toggle"),(query(&dialog,".cd-dialog-close"),"click","close"),(dialog.clone(),"click","click"),(dialog,"input","volume")] {
            let weak=Rc::downgrade(&state);let listener=Listener::new(&target,event,move |event|{let Some(state)=weak.upgrade()else{return;};let state=state.borrow();let audio=get(&state.host,"gameAudio");
                match action{
                    "open"=>{call(&audio,"unlock",&[]);state.draw();call(&state.dialog,"showModal",&[]);call(&state.host,"liquidGlass",&[state.dialog.clone()]);},
                    "toggle"=>{call(&audio,"toggle",&[]);state.draw();},
                    "close"=>{call(&state.dialog,"close",&[]);},
                    "volume"=>{let input=closest(&event,"input[data-k]");if exists(&input){call(&audio,"unlock",&[]);call(&audio,"setVolume",&[js(&data(&input,"k")),number(text(&get(&input,"value")).parse::<f64>().unwrap_or(0.0)/100.0)]);state.draw();}},
                    "click"=>{if get(&event,"target")==state.dialog{call(&state.dialog,"close",&[]);return;}let choice=closest(&event,"[data-choice]");if !exists(&choice){return;}let group=call(&choice,"closest",&[js("[data-g]")]);let id=js(&data(&choice,"choice"));if data(&group,"g")=="cam"{call(&get(&state.host,"store"),"set",&[js("cd-camera"),id]);}else{call(&state.host,"keepQuality",&[id]);}state.draw();},_=>{}
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
    if on && exists(&home) && get(&button, "parentElement") != home {
        call(&home, "append", &[button]);
    }
}
