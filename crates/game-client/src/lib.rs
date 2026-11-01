//! Platform-independent client policy. The browser is an I/O and rendering adapter.
use serde_json::{json, Value};
use std::sync::OnceLock;

mod arena;
mod audio;
mod conditions;
mod dsp;
mod duel;
mod lens;
mod loadout;
mod look;
mod server;
mod server_view;
mod words;
mod workshop;
pub use arena::compute as arena_compute;
pub use dsp::audio_samples;

pub fn data() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../fixtures/client.json")).expect("client data")
    })
}
pub(crate) fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}
pub(crate) fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}
pub(crate) fn truth(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}
pub(crate) fn core(method: &str, args: Value) -> Result<Value, String> {
    game_core::dispatch(method, args)
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    match method {
        "client.data" => Ok(data().clone()),
        "quality.guess" => Ok(json!(guess_quality(&args[0]))),
        "quality.start" => {
            let supplied = text(&args[0]);
            let stored = text(&args[1]);
            let valid = |s: &str| data()["QUALITY"].get(s).is_some();
            Ok(if valid(supplied) {
                json!({"id": supplied, "picked": true})
            } else if valid(stored) {
                json!({"id": stored, "picked": true})
            } else {
                json!({"id":guess_quality(&args[2]),"picked":false})
            })
        }
        _ if method.starts_with("server.view.") => server_view::dispatch(method, args),
        _ if method.starts_with("server.") => server::dispatch(method, args),
        _ if method.starts_with("client.arena.") => arena::dispatch(method, args),
        _ if method.starts_with("client.conditions.") => conditions::dispatch(method, args),
        _ if method.starts_with("client.workshop.") || method.starts_with("client.models.") => {
            workshop::dispatch(method, args)
        }
        _ if method.starts_with("audio.dsp.") => dsp::dispatch(method, args),
        _ if method.starts_with("client.duel.") || method == "client.moveCommand" => {
            duel::dispatch(method, args)
        }
        _ if method.starts_with("client.look.") => look::dispatch(method, args),
        _ if method.starts_with("client.") => loadout::dispatch(method, args),
        _ if method.starts_with("audio.") => audio::dispatch(method, args),
        _ if method.starts_with("words.") => words::dispatch(method, args),
        _ => Err(format!("Unknown client operation: {method}")),
    }
}

fn guess_quality(device: &Value) -> &'static str {
    if !truth(&device["webgl"]) {
        return "low";
    }
    let gpu = text(&device["gpu"]).to_lowercase();
    if ["swiftshader", "llvmpipe", "software", "basic render"]
        .iter()
        .any(|x| gpu.contains(x))
    {
        return "low";
    }
    let memory = device["memory"].as_f64().unwrap_or(4.0);
    let cores = device["cores"].as_f64().unwrap_or(4.0);
    if truth(&device["phone"]) {
        let mali = gpu.contains("mali-t")
            || gpu.contains("mali-4")
            || (50..=59)
                .chain(70..=72)
                .any(|n| gpu.contains(&format!("mali-g{n}")));
        let adreno = (300..=599).any(|n| gpu.contains(&format!("adreno (tm) {n}")));
        return if memory <= 3.0 || cores <= 4.0 || mali || adreno {
            "low"
        } else {
            "medium"
        };
    }
    if ["intel", "uhd", "hd graphics", "iris"]
        .iter()
        .any(|x| gpu.contains(x))
        && memory <= 8.0
    {
        "medium"
    } else {
        "high"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quality_does_not_select_ultra_automatically() {
        for device in [
            json!({}),
            json!({"webgl":true,"gpu":"RTX 5090","memory":32,"cores":24}),
            json!({"webgl":true,"gpu":"Adreno (TM) 740","phone":true,"memory":8,"cores":8}),
        ] {
            assert_ne!(guess_quality(&device), "ultra");
        }
    }
    #[test]
    fn software_and_old_mobile_renderers_are_restricted() {
        assert_eq!(
            guess_quality(&json!({"webgl":true,"gpu":"Google SwiftShader"})),
            "low"
        );
        assert_eq!(
            guess_quality(
                &json!({"webgl":true,"gpu":"Adreno (TM) 540","phone":true,"memory":8,"cores":8})
            ),
            "low"
        );
    }
}
