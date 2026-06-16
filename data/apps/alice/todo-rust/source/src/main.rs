use rusqlite::{params, Connection};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

const PORT: &str = "PORT";
const DB_PATH: &str = "data/todos.db";

fn init_db() -> Connection {
    std::fs::create_dir_all("data").ok();
    let conn = Connection::open(DB_PATH).expect("Failed to open DB");
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").ok();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS todos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            done INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now'))
        )",
    ).expect("Failed to create table");
    conn
}

fn json_response(status: &str, body: &str) -> Vec<u8> {
    format!("HTTP/1.1 {}\r\nContent-Type: application/json; charset=utf-8\r\nConnection: close\r\n\r\n{}", status, body).into_bytes()
}

fn html_response(status: &str, body: &str) -> Vec<u8> {
    format!("HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n{}", status, body).into_bytes()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn handle_request(conn: &Connection, method: &str, path: &str, body: &str) -> Vec<u8> {
    match (method, path) {
        ("GET", "/") => html_response("200 OK", HTML),
        ("GET", "/api/todos") => {
            let mut stmt = conn.prepare("SELECT id, title, done, created_at FROM todos ORDER BY created_at DESC").unwrap();
            let items: Vec<String> = stmt.query_map([], |row| {
                Ok(format!(
                    r#"{{"id":{},"title":"{}","done":{},"created_at":"{}"}}"#,
                    row.get::<_, i64>(0)?, esc(&row.get::<_, String>(1)?),
                    row.get::<_, i32>(2)?, row.get::<_, String>(3)?
                ))
            }).unwrap().filter_map(|r| r.ok()).collect();
            json_response("200 OK", &format!("[{}]", items.join(",")))
        }
        ("POST", "/api/todos") => {
            let title = serde_json::from_str::<serde_json::Value>(body).ok()
                .and_then(|v| v.get("title").and_then(|t| t.as_str()).map(|s| s.trim().to_string()))
                .unwrap_or_default();
            if title.is_empty() {
                return json_response("400 Bad Request", r#"{"error":"Title required"}"#);
            }
            conn.execute("INSERT INTO todos (title) VALUES (?1)", params![title]).unwrap();
            let id = conn.last_insert_rowid();
            json_response("201 Created", &format!(r#"{{"id":{},"title":"{}","done":0}}"#, id, esc(&title)))
        }
        ("PUT", "/api/todos") => json_response("404 Not Found", r#"{"error":"Not found"}"#),
        (m, _) if m == "PUT" && path.starts_with("/api/todos/") => {
            let id: i64 = match path.split('/').nth(3).and_then(|s| s.parse().ok()) {
                Some(v) => v, None => return json_response("400 Bad Request", r#"{"error":"Invalid id"}"#),
            };
            let done: bool = serde_json::from_str::<serde_json::Value>(body).ok()
                .and_then(|v| v.get("done").and_then(|d| d.as_bool())).unwrap_or(false);
            conn.execute("UPDATE todos SET done=?1 WHERE id=?2", params![done as i32, id]).ok();
            match conn.query_row("SELECT id, title, done, created_at FROM todos WHERE id=?1", params![id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)?, row.get::<_, String>(3)?))
            ) {
                Ok((id, title, done, created_at)) => json_response("200 OK", &format!(
                    r#"{{"id":{},"title":"{}","done":{},"created_at":"{}"}}"#, id, esc(&title), done, created_at
                )),
                Err(_) => json_response("404 Not Found", r#"{"error":"Not found"}"#),
            }
        }
        (m, _) if m == "DELETE" && path.starts_with("/api/todos/") => {
            let id: i64 = match path.split('/').nth(3).and_then(|s| s.parse().ok()) {
                Some(v) => v, None => return json_response("400 Bad Request", r#"{"error":"Invalid id"}"#),
            };
            conn.execute("DELETE FROM todos WHERE id=?1", params![id]).ok();
            json_response("200 OK", r#"{"ok":true}"#)
        }
        _ => json_response("404 Not Found", r#"{"error":"Not found"}"#),
    }
}

fn handle_client(mut stream: TcpStream, conn: &Connection) {
    let mut buf = [0; 8192];
    let n = match stream.read(&mut buf) { Ok(n) if n > 0 => n, _ => return };
    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    let mut lines = request.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/").split('?').next().unwrap_or("/");
    let body = request.find("\r\n\r\n").map(|pos| request[pos + 4..].trim().to_string()).unwrap_or_default();
    let response = handle_request(conn, method, path, &body);
    let _ = stream.write_all(&response);
    let _ = stream.flush();
}

fn main() {
    let port = std::env::var(PORT).unwrap_or_else(|_| "3000".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let conn = init_db();
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("Todo app (Rust) listening on {}", addr);
    for stream in listener.incoming() {
        match stream { Ok(stream) => handle_client(stream, &conn), Err(e) => eprintln!("Connection error: {}", e) }
    }
}


const HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Todo App (Rust)</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:system-ui,-apple-system,sans-serif;background:#f4f4f5;color:#18181b;max-width:640px;margin:0 auto;padding:40px 20px}
h1{font-size:1.8rem;margin-bottom:4px}
.sub{color:#71717a;font-size:14px;margin-bottom:24px}
.tab{display:flex;gap:8px;margin-bottom:16px}
.tab button{padding:6px 16px;border:1px solid #d4d4d8;border-radius:6px;background:#fff;cursor:pointer;font-size:13px;color:#52525b}
.tab button.active{background:#d97706;color:#fff;border-color:#d97706}
form{display:flex;gap:8px;margin-bottom:20px}
form input{flex:1;padding:10px 14px;border:1px solid #d4d4d8;border-radius:8px;font-size:14px;outline:none}
form input:focus{border-color:#d97706;box-shadow:0 0 0 3px rgba(217,119,6,.1)}
form button{padding:10px 20px;background:#d97706;color:#fff;border:none;border-radius:8px;cursor:pointer;font-size:14px;font-weight:500}
.todo{display:flex;align-items:center;gap:12px;padding:12px 16px;background:#fff;border:1px solid #e4e4e7;border-radius:8px;margin-bottom:8px;cursor:pointer}
.todo:hover{box-shadow:0 1px 4px rgba(0,0,0,.06)}
.todo.done .title{text-decoration:line-through;color:#a1a1aa}
.todo .title{flex:1;font-size:14px}
.todo .meta{font-size:11px;color:#a1a1aa}
.todo button{background:none;border:none;color:#dc2626;cursor:pointer;font-size:16px;padding:4px;opacity:0}
.todo:hover button{opacity:1}
.empty{text-align:center;color:#a1a1aa;padding:40px 0;font-size:14px}
.badge{display:inline-block;background:#d97706;color:#fff;font-size:11px;padding:2px 10px;border-radius:999px;margin-left:8px;vertical-align:middle}
</style>
</head>
<body>
<h1>Todos <span class="badge">Rust + SQLite</span></h1>
<p class="sub">A full-stack CRUD app with rusqlite</p>
<div class="tab"><button class="active" data-filter="all">All</button><button data-filter="active">Active</button><button data-filter="done">Done</button></div>
<form id="form"><input id="input" placeholder="Add a new todo..." required autofocus><button type="submit">Add</button></form>
<div id="list"></div>
<script>
let todos=[],filter="all",api=window.location.pathname.replace(/\/?$/,"/")+"api/todos";
async function load(){const r=await fetch(api);todos=await r.json();render()}
async function add(title){const r=await fetch(api,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({title})});const t=await r.json();todos.push(t);render()}
async function toggle(id){const t=todos.find(x=>x.id===id);if(!t)return;t.done=t.done?0:1;await fetch(api+"/"+id,{method:"PUT",headers:{"Content-Type":"application/json"},body:JSON.stringify({done:t.done})});render()}
async function remove(id){todos=todos.filter(x=>x.id!==id);await fetch(api+"/"+id,{method:"DELETE"});render()}
function render(){const f=todos.filter(t=>filter==="all"?true:filter==="active"?!t.done:t.done);
document.querySelector("#list").innerHTML=f.length?f.map(t=>'<div class="todo'+(t.done?" done":"")+'" onclick="toggle('+t.id+')"><span class="title">'+esc(t.title)+'</span><span class="meta">#'+t.id+'</span><button onclick="event.stopPropagation();remove('+t.id+')">x</button></div>').join(""):'<div class="empty">No todos yet</div>'}
function esc(s){const d=document.createElement("div");d.textContent=s;return d.innerHTML}
document.querySelector("#form").addEventListener("submit",async e=>{e.preventDefault();const i=document.querySelector("#input");const v=i.value.trim();if(v){await add(v);i.value="";i.focus()}});
document.querySelectorAll(".tab button").forEach(b=>b.addEventListener("click",()=>{document.querySelectorAll(".tab button").forEach(x=>x.classList.remove("active"));b.classList.add("active");filter=b.dataset.filter;render()}));
load();
</script>
</body>
</html>"##;
