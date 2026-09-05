//   node serve.mjs                       -> http://localhost:8099 (auto-opens browser)
//   node serve.mjs 5050                  -> custom port
//   node serve.mjs --no-open             -> do not open a browser
//   node serve.mjs --root target/site    -> serve a rendered site instead of this directory

import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { watch } from 'node:fs';
import { extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';

const args = process.argv.slice(2);
const rootArg = args.indexOf('--root');
const ROOT = rootArg >= 0 && args[rootArg + 1]
  ? resolve(args[rootArg + 1]) + sep
  : fileURLToPath(new URL('.', import.meta.url));
const PORT = Number(args.find((a) => /^\d+$/.test(a))) || 8099;
const OPEN = !args.includes('--no-open');

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
};

// Live-reload: HTML pages subscribe to this SSE stream; a file change pings it.
const clients = new Set();
const RELOAD_SNIPPET = `<script>
  (function(){
    var es = new EventSource('/__reload');
    es.onmessage = function(){ location.reload(); };
    es.onerror = function(){ es.close(); setTimeout(function(){ location.reload(); }, 1000); };
  })();
</script>`;

let debounce;
const fire = () =>
{
  clearTimeout(debounce);
  debounce = setTimeout(() => { for (const res of clients) res.write('data: reload\n\n'); }, 80);
};
watch(ROOT, { recursive: true }, fire);

const server = createServer(async (req, res) =>
{
  if (req.url === '/__reload')
  {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    });
    res.write('retry: 1000\n\n');
    clients.add(res);
    req.on('close', () => clients.delete(res));
    return;
  }
  try
  {
    let path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (path === '/') path = '/index.html';
    const file = normalize(join(ROOT, path));
    if (!file.startsWith(ROOT)) { res.writeHead(403).end('forbidden'); return; }
    const type = TYPES[extname(file)] || 'application/octet-stream';
    if (type.startsWith('text/html'))
    {
      let html = await readFile(file, 'utf8');
      html = html.replace('</body>', RELOAD_SNIPPET + '</body>');
      res.writeHead(200, { 'Content-Type': type, 'Cache-Control': 'no-store' });
      res.end(html);
    } else
    {
      const body = await readFile(file);
      res.writeHead(200, { 'Content-Type': type, 'Cache-Control': 'no-store' });
      res.end(body);
    }
  } catch
  {
    res.writeHead(404, { 'Content-Type': 'text/plain' }).end('not found');
  }
});

server.listen(PORT, () =>
{
  const url = `http://localhost:${PORT}`;
  console.log(`\n  pamoja showcase -> ${url}\n  live reload on, editing files refreshes the tab\n  (Ctrl+C to stop)\n`);
  if (OPEN)
  {
    const cmd = process.platform === 'win32' ? 'cmd' : process.platform === 'darwin' ? 'open' : 'xdg-open';
    const a = process.platform === 'win32' ? ['/c', 'start', '', url] : [url];
    try { spawn(cmd, a, { stdio: 'ignore', detached: true }).unref(); } catch { }
  }
});
