const http = require('http');
const port = process.env.PORT || 3000;
const server = http.createServer((req, res) => {
  const msg = `<html><head><meta charset="utf-8"><title>Node.js App</title>
<style>*{margin:0;padding:0;box-sizing:border-box}body{font-family:system-ui,sans-serif;display:flex;align-items:center;justify-content:center;min-height:100vh;background:#f4f4f5;color:#18181b}h1{font-size:2rem;margin-bottom:8px}p{color:#52525b}.card{background:#fff;border:1px solid #e4e4e7;border-radius:12px;padding:40px;text-align:center;max-width:400px}.badge{display:inline-block;background:#2563eb;color:#fff;font-size:12px;padding:4px 12px;border-radius:999px;margin-top:16px}</style></head>
<body><div class="card"><h1>Hello from Node.js!</h1><p>This app is running on gitpage App Hosting.</p><div class="badge">Node.js</div></div></body></html>`;
  res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
  res.end(msg);
});
server.listen(port, () => console.log(`Listening on ${port}`));
