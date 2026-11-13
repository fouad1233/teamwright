/**
 * Serves the website on this computer, the way GitHub Pages serves it.
 *
 *   node website/tools/serve.mjs [port]
 *
 * Then open http://localhost:4321/teamwright/. The site lives under
 * /teamwright/, as it will on GitHub Pages, so that the page that answers
 * an unknown address (404.html) can be tried with its real links.
 *
 * Nothing is needed but Node.js. It answers range requests, which a
 * browser uses to play a video, and sends no headers that keep a file in a
 * cache, so that a change shows at the next reload.
 */

import { createReadStream, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { dirname, extname, join, normalize, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const WEBSITE = join(dirname(fileURLToPath(import.meta.url)), '..');
const BASE = '/teamwright/';
const PORT = Number(process.argv[2] || process.env.PORT || 4321);

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json; charset=utf-8',
  '.xml': 'application/xml; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.webp': 'image/webp',
  '.ico': 'image/x-icon',
  '.woff2': 'font/woff2',
  '.mp4': 'video/mp4',
  '.bin': 'application/octet-stream',
};

/** The file under website/ that a path names, or null. */
function fileFor(path) {
  let relative = decodeURIComponent(path.slice(BASE.length));
  if (relative === '' || relative.endsWith('/')) relative += 'index.html';
  const file = normalize(join(WEBSITE, relative));
  if (!file.startsWith(WEBSITE + sep)) return null;
  // The tools are not part of the published site.
  if (file.startsWith(join(WEBSITE, 'tools') + sep)) return null;
  try {
    return statSync(file).isFile() ? file : null;
  } catch {
    return null;
  }
}

function send(response, status, file, request) {
  const size = statSync(file).size;
  const headers = {
    'Content-Type': TYPES[extname(file)] || 'application/octet-stream',
    'Accept-Ranges': 'bytes',
    'Cache-Control': 'no-store',
  };
  const range = request.headers.range && /^bytes=(\d*)-(\d*)$/.exec(request.headers.range);
  if (status === 200 && range) {
    let start = range[1] === '' ? size - Number(range[2]) : Number(range[1]);
    let end = range[1] === '' || range[2] === '' ? size - 1 : Number(range[2]);
    start = Math.max(0, start);
    end = Math.min(size - 1, end);
    if (start > end) {
      response.writeHead(416, { 'Content-Range': `bytes */${size}` });
      response.end();
      return;
    }
    response.writeHead(206, { ...headers, 'Content-Range': `bytes ${start}-${end}/${size}`, 'Content-Length': end - start + 1 });
    if (request.method === 'HEAD') response.end();
    else createReadStream(file, { start, end }).pipe(response);
    return;
  }
  response.writeHead(status, { ...headers, 'Content-Length': size });
  if (request.method === 'HEAD') response.end();
  else createReadStream(file).pipe(response);
}

const server = createServer((request, response) => {
  const path = new URL(request.url, 'http://localhost').pathname;
  if (path === '/' || path === '/teamwright') {
    response.writeHead(302, { Location: BASE });
    response.end();
    return;
  }
  const file = path.startsWith(BASE) ? fileFor(path) : null;
  if (file) send(response, 200, file, request);
  else send(response, 404, join(WEBSITE, '404.html'), request);
});

server.listen(PORT, () => {
  console.log(`The website is at http://localhost:${PORT}${BASE}`);
});
