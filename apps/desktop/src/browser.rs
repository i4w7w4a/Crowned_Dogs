mod server;

fn main() {
    match server::game_dir().and_then(server::start) {
        Ok(url) => {
            println!(
                "Crowned Dogs — {url}\nИгра работает локально. Закройте это окно, чтобы остановить её."
            );
            if !std::env::args().any(|a| a == "--no-open") {
                // Start the user's browser through the OS URL handler, with no shell interpolation.
                let _ = std::process::Command::new("rundll32.exe")
                    .args(["url.dll,FileProtocolHandler", &url])
                    .spawn();
            }
            loop {
                std::thread::park();
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
