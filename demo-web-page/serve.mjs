// Serveur statique minimal, sans dépendance : sert la racine du dépôt
// (pour que /docs/… et /demo-web-page/… soient accessibles) et ouvre la page carte.
//
//   node demo-web-page/serve.mjs            → http://localhost:8080/demo-web-page/
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const PORT = Number(process.env.PORT ?? 8080);
const TYPES = { ".html": "text/html; charset=utf-8", ".css": "text/css", ".js": "text/javascript", ".mjs": "text/javascript",
  ".json": "application/json", ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".png": "image/png", ".svg": "image/svg+xml" };

createServer(async (req, res) => {
  try {
    let p = decodeURIComponent(new URL(req.url, "http://x").pathname);
    if (p.endsWith("/")) p += "index.html";
    const file = path.join(ROOT, p);
    if (!file.startsWith(ROOT)) throw Object.assign(new Error("forbidden"), { code: "EACCES" });
    if ((await stat(file)).isDirectory()) { res.writeHead(302, { Location: p + "/" }); return res.end(); }
    res.writeHead(200, { "content-type": TYPES[path.extname(file)] ?? "application/octet-stream", "cache-control": "no-store" });
    res.end(await readFile(file));
  } catch (err) {
    res.writeHead(err.code === "EACCES" ? 403 : 404); res.end("not found");
  }
}).listen(PORT, () => console.log(`Card Place web → http://localhost:${PORT}/demo-web-page/`));
