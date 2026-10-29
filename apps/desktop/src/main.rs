#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod server;

fn main() {
    if let Err(error) = run() {
        let log = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("CrownedDogs-error.txt")));
        if let Some(log) = log {
            let _ = std::fs::write(log, &error);
        }
        eprintln!("{error}");
    }
}

fn run() -> Result<(), String> {
    let mut url = server::start(server::game_dir()?)?;
    if std::env::args().any(|a| a == "--muted") {
        url.push_str("?muted=1");
    }
    tauri::Builder::default()
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(url.parse()?),
            )
            .title("Crowned Dogs")
            .inner_size(1050.0, 900.0)
            .min_inner_size(360.0, 640.0)
            .background_color(tauri::window::Color(8, 10, 10, 255))
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| e.to_string())
}
