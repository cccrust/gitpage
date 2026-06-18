use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("Listening on {}", addr);

    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            let mut buf = [0; 4096];
            let _ = stream.read(&mut buf);
            let body = r#"<html><head><meta charset="utf-8"><title>Rust App</title>
<style>*{margin:0;padding:0;box-sizing:border-box}body{font-family:system-ui,sans-serif;display:flex;align-items:center;justify-content:center;min-height:100vh;background:#f4f4f5;color:#18181b}h1{font-size:2rem;margin-bottom:8px}p{color:#52525b}.card{background:#fff;border:1px solid #e4e4e7;border-radius:12px;padding:40px;text-align:center;max-width:400px}.badge{display:inline-block;background:#d97706;color:#fff;font-size:12px;padding:4px 12px;border-radius:999px;margin-top:16px}</style></head>
<body><div class="card"><h1>Hello from Rust!</h1><p>This app is running on gitpage App Hosting.</p><div class="badge">Rust</div></div></body></html>"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    }
}
