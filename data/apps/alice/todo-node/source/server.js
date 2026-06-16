const http = require('http');
const path = require('path');
const Database = require('better-sqlite3');
const url = require('url');

const PORT = process.env.PORT || 3000;
const dbPath = path.join(__dirname, 'data', 'todos.db');
require('fs').mkdirSync(path.join(__dirname, 'data'), { recursive: true });
const db = new Database(dbPath);
db.pragma('journal_mode=WAL');
db.exec(`CREATE TABLE IF NOT EXISTS todos (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  done INTEGER DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now'))
)`);

function json(res, status, data) {
  res.writeHead(status, { 'Content-Type': 'application/json; charset=utf-8' });
  res.end(JSON.stringify(data));
}
function html(res, status, content) {
  res.writeHead(status, { 'Content-Type': 'text/html; charset=utf-8' });
  res.end(content);
}
function parseBody(req) {
  return new Promise((resolve) => {
    let body = '';
    req.on('data', c => body += c);
    req.on('end', () => {
      try { resolve(JSON.parse(body)); } catch { resolve(null); }
    });
  });
}

const HTML = 
'<!DOCTYPE html>' +
'<html lang="en">' +
'<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">' +
'<title>Todo App (Node.js)</title>' +
'<style>' +
'*{margin:0;padding:0;box-sizing:border-box}' +
'body{font-family:system-ui,-apple-system,sans-serif;background:#f4f4f5;color:#18181b;max-width:640px;margin:0 auto;padding:40px 20px}' +
'h1{font-size:1.8rem;margin-bottom:4px}' +
'.sub{color:#71717a;font-size:14px;margin-bottom:24px}' +
'.tab{display:flex;gap:8px;margin-bottom:16px}' +
'.tab button{padding:6px 16px;border:1px solid #d4d4d8;border-radius:6px;background:#fff;cursor:pointer;font-size:13px;color:#52525b}' +
'.tab button.active{background:#18181b;color:#fff;border-color:#18181b}' +
'form{display:flex;gap:8px;margin-bottom:20px}' +
'form input{flex:1;padding:10px 14px;border:1px solid #d4d4d8;border-radius:8px;font-size:14px;outline:none}' +
'form input:focus{border-color:#18181b;box-shadow:0 0 0 3px rgba(0,0,0,.08)}' +
'form button{padding:10px 20px;background:#18181b;color:#fff;border:none;border-radius:8px;cursor:pointer;font-size:14px;font-weight:500}' +
'.todo{display:flex;align-items:center;gap:12px;padding:12px 16px;background:#fff;border:1px solid #e4e4e7;border-radius:8px;margin-bottom:8px;cursor:pointer}' +
'.todo:hover{box-shadow:0 1px 4px rgba(0,0,0,.06)}' +
'.todo.done .title{text-decoration:line-through;color:#a1a1aa}' +
'.todo .title{flex:1;font-size:14px}' +
'.todo .meta{font-size:11px;color:#a1a1aa}' +
'.todo button{background:none;border:none;color:#dc2626;cursor:pointer;font-size:16px;padding:4px;opacity:0}' +
'.todo:hover button{opacity:1}' +
'.empty{text-align:center;color:#a1a1aa;padding:40px 0;font-size:14px}' +
'.badge{display:inline-block;background:#2563eb;color:#fff;font-size:11px;padding:2px 10px;border-radius:999px;margin-left:8px;vertical-align:middle}' +
'</style>' +
'</head>' +
'<body>' +
'<h1>Todos <span class="badge">Node.js + SQLite</span></h1>' +
'<p class="sub">A full-stack CRUD app with better-sqlite3</p>' +
'<div class="tab"><button class="active" data-filter="all">All</button><button data-filter="active">Active</button><button data-filter="done">Done</button></div>' +
'<form id="form"><input id="input" placeholder="Add a new todo..." required autofocus><button type="submit">Add</button></form>' +
'<div id="list"></div>' +
'<script>' +
'let todos=[],filter="all",api=window.location.pathname.replace(/\\/?$/,"/")+"api/todos";' +
'async function load(){const r=await fetch(api);todos=await r.json();render()}' +
'async function add(title){const r=await fetch(api,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({title})});const t=await r.json();todos.push(t);render()}' +
'async function toggle(id){const t=todos.find(x=>x.id===id);if(!t)return;t.done=t.done?0:1;await fetch(api+"/"+id,{method:"PUT",headers:{"Content-Type":"application/json"},body:JSON.stringify({done:t.done})});render()}' +
'async function remove(id){todos=todos.filter(x=>x.id!==id);await fetch(api+"/"+id,{method:"DELETE"});render()}' +
'function render(){const filtered=todos.filter(t=>filter==="all"?true:filter==="active"?!t.done:t.done);' +
'document.getElementById("list").innerHTML=filtered.length?filtered.map(t=>\'<div class="todo\'+(t.done?" done":"")+\'" onclick="toggle(\'+t.id+\')"><span class="title">\'+esc(t.title)+\'</span><span class="meta">#\'+t.id+\'</span><button onclick="event.stopPropagation();remove(\'+t.id+\')">x</button></div>\').join(""):\'<div class="empty">No todos yet</div>\'}' +
'function esc(s){const d=document.createElement("div");d.textContent=s;return d.innerHTML}' +
'document.getElementById("form").addEventListener("submit",async e=>{e.preventDefault();const i=document.getElementById("input");const v=i.value.trim();if(v){await add(v);i.value="";i.focus()}});' +
'document.querySelectorAll(".tab button").forEach(b=>b.addEventListener("click",()=>{document.querySelectorAll(".tab button").forEach(x=>x.classList.remove("active"));b.classList.add("active");filter=b.dataset.filter;render()}));' +
'load();' +
'</script>' +
'</body>' +
'</html>'
;

const routes = {
  'GET /api/todos': (req, res) => {
    const list = db.prepare('SELECT * FROM todos ORDER BY created_at DESC').all();
    json(res, 200, list);
  },
  'POST /api/todos': async (req, res) => {
    const body = await parseBody(req);
    if (!body || !body.title || !body.title.trim()) return json(res, 400, { error: 'Title required' });
    const stmt = db.prepare('INSERT INTO todos (title) VALUES (?)');
    const info = stmt.run(body.title.trim());
    const todo = db.prepare('SELECT * FROM todos WHERE id = ?').get(info.lastInsertRowid);
    json(res, 201, todo);
  },
  'PUT /api/todos/:id': async (req, res, id) => {
    const body = await parseBody(req);
    if (!body) return json(res, 400, { error: 'Invalid JSON' });
    const existing = db.prepare('SELECT * FROM todos WHERE id = ?').get(id);
    if (!existing) return json(res, 404, { error: 'Not found' });
    const done = body.done !== undefined ? (body.done ? 1 : 0) : existing.done;
    const title = body.title !== undefined ? body.title.trim() : existing.title;
    db.prepare('UPDATE todos SET title=?, done=? WHERE id=?').run(title, done, id);
    const todo = db.prepare('SELECT * FROM todos WHERE id = ?').get(id);
    json(res, 200, todo);
  },
  'DELETE /api/todos/:id': (req, res, id) => {
    db.prepare('DELETE FROM todos WHERE id = ?').run(id);
    json(res, 200, { ok: true });
  },
  'GET /': (req, res) => html(res, 200, HTML),
};

const server = http.createServer((req, res) => {
  const u = url.parse(req.url, true);
  const method = req.method;
  const p = u.pathname.replace(/\/+$/, '') || '/';
  const key = method + ' ' + p;

  if (routes[key]) return routes[key](req, res);

  if (u.pathname.startsWith('/api/todos/')) {
    const id = u.pathname.split('/')[3];
    if (method === 'PUT' && routes['PUT /api/todos/:id'] && id) return routes['PUT /api/todos/:id'](req, res, parseInt(id));
    if (method === 'DELETE' && routes['DELETE /api/todos/:id'] && id) return routes['DELETE /api/todos/:id'](req, res, parseInt(id));
  }

  json(res, 404, { error: 'Not found' });
});

server.listen(PORT, () => console.log('Todo app running on port', PORT));
