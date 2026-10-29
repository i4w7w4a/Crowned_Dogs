use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

pub const PORT: u16 = 5197;

pub fn game_dir() -> Result<PathBuf, String> {
    if let Some(path) = std::env::args().skip_while(|a| a != "--assets").nth(1) {
        return checked_root(Path::new(&path));
    }
    let beside = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .unwrap()
        .join("game");
    if beside.join("index.html").is_file() {
        return checked_root(&beside);
    }
    checked_root(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/dist"))
}

fn checked_root(path: &Path) -> Result<PathBuf, String> {
    if !path.join("index.html").is_file() {
        return Err(format!("Не найдена папка game с игрой: {}", path.display()));
    }
    path.canonicalize().map_err(|e| e.to_string())
}

pub fn start(root: PathBuf) -> Result<String, String> {
    let listener = TcpListener::bind(("127.0.0.1", PORT))
        .map_err(|e| format!("Порт {PORT} занят. Закройте другую локальную сборку. {e}"))?;
    let active = Arc::new(AtomicUsize::new(0));
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if active.fetch_add(1, Ordering::Relaxed) >= 32 {
                active.fetch_sub(1, Ordering::Relaxed);
                continue;
            }
            let root = root.clone();
            let active = active.clone();
            thread::spawn(move || {
                let _ = handle(stream, &root);
                active.fetch_sub(1, Ordering::Relaxed);
            });
        }
    });
    Ok(format!("http://127.0.0.1:{PORT}/"))
}

fn decoded_path(url: &str) -> Option<PathBuf> {
    let raw = url.split('?').next()?.as_bytes();
    let mut bytes = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%' {
            let hex = std::str::from_utf8(raw.get(i + 1..i + 3)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            bytes.push(raw[i]);
            i += 1;
        }
    }
    let path = String::from_utf8(bytes).ok()?;
    if !path.starts_with('/') || path.contains(['\\', '\0', ':']) {
        return None;
    }
    let relative = path.trim_start_matches('/');
    let path = PathBuf::from(if relative.is_empty() {
        "index.html"
    } else {
        relative
    });
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    Some(path)
}

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "woff2" => "font/woff2",
        "glb" => "model/gltf-binary",
        _ => "application/octet-stream",
    }
}

fn range(value: &str, size: u64) -> Option<(u64, u64)> {
    if size == 0 {
        return None;
    }
    let (start, end) = value.strip_prefix("bytes=")?.split_once('-')?;
    if start.is_empty() {
        let suffix: u64 = end.parse().ok()?;
        return (suffix > 0).then_some((size.saturating_sub(suffix), size - 1));
    }
    let start: u64 = start.parse().ok()?;
    let end: u64 = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>().ok()?.min(size - 1)
    };
    (start <= end && start < size).then_some((start, end))
}

fn error(stream: &mut TcpStream, status: &str, extra: &str) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n{extra}\r\n"
    )
}

fn handle(mut stream: TcpStream, root: &Path) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(15)))?;
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buffer)?;
        if n == 0 {
            return Ok(());
        }
        request.extend_from_slice(&buffer[..n]);
        if request.len() > 8192 {
            return error(&mut stream, "431 Request Header Fields Too Large", "");
        }
    }
    let text = match std::str::from_utf8(&request) {
        Ok(s) => s,
        Err(_) => return error(&mut stream, "400 Bad Request", ""),
    };
    let mut lines = text.split("\r\n");
    let mut parts = lines.next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let url = parts.next().unwrap_or("");
    if !matches!(method, "GET" | "HEAD") {
        return error(
            &mut stream,
            "405 Method Not Allowed",
            "Allow: GET, HEAD\r\n",
        );
    }
    let headers: Vec<_> = lines.filter_map(|s| s.split_once(':')).collect();
    // Bind to loopback and reject alternate Host headers (including DNS rebinding).
    let host = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("host"))
        .map(|(_, v)| v.trim())
        .unwrap_or("");
    if host != format!("127.0.0.1:{PORT}") && host != format!("localhost:{PORT}") {
        return error(&mut stream, "403 Forbidden", "");
    }
    if url == "/__cd_health" {
        let body = concat!(
            "{\"game\":\"Crowned Dogs\",\"version\":\"",
            env!("CARGO_PKG_VERSION"),
            "\",\"engine\":\"Rust/WASM\"}"
        );
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
            body.len()
        )?;
        if method != "HEAD" {
            stream.write_all(body.as_bytes())?;
        }
        return Ok(());
    }
    let Some(path) = decoded_path(url) else {
        return error(&mut stream, "403 Forbidden", "");
    };
    let path = match root.join(path).canonicalize() {
        Ok(p) if p.starts_with(root) && p.is_file() => p,
        _ => return error(&mut stream, "404 Not Found", ""),
    };
    let mut file = File::open(&path)?;
    let size = file.metadata()?.len();
    let wanted = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("range"))
        .map(|(_, v)| v.trim());
    let (status, start, length, extra) = if let Some(wanted) = wanted {
        let Some((start, end)) = range(wanted, size) else {
            return error(
                &mut stream,
                "416 Range Not Satisfiable",
                &format!("Content-Range: bytes */{size}\r\n"),
            );
        };
        (
            "206 Partial Content",
            start,
            end - start + 1,
            format!("Content-Range: bytes {start}-{end}/{size}\r\n"),
        )
    } else {
        ("200 OK", 0, size, String::new())
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {length}\r\nAccept-Ranges: bytes\r\nCache-Control: no-cache\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n{extra}\r\n",
        mime(&path)
    )?;
    if method != "HEAD" {
        file.seek(SeekFrom::Start(start))?;
        std::io::copy(&mut file.take(length), &mut stream)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_paths_outside_game() {
        for p in [
            "/../secret",
            "/%2e%2e/secret",
            "/a%5c..%5csecret",
            "/C:/secret",
            "/a%00x",
            "http://evil/a",
        ] {
            assert!(decoded_path(p).is_none(), "{p}");
        }
        assert_eq!(
            decoded_path("/assets/a.js?v=3"),
            Some(PathBuf::from("assets/a.js"))
        );
        assert_eq!(decoded_path("/"), Some(PathBuf::from("index.html")));
    }
    #[test]
    fn audio_byte_ranges() {
        assert_eq!(range("bytes=2-5", 20), Some((2, 5)));
        assert_eq!(range("bytes=-5", 20), Some((15, 19)));
        assert_eq!(range("bytes=10-", 20), Some((10, 19)));
        assert_eq!(range("bytes=21-", 20), None);
        assert_eq!(range("bytes=0-2,5-8", 20), None);
    }
}
