// Yerel testler icin D1 (node:sqlite) ve R2 (bellek) taklidi. Cloudflare'de bu dosya KULLANILMAZ.
import { DatabaseSync } from 'node:sqlite';
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, extname } from 'node:path';
const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');

class Stmt {
  constructor(db, sql) { this.db = db; this.sql = sql; this.args = []; }
  bind(...a) { this.args = a.map((x) => (x === undefined ? null : typeof x === 'boolean' ? (x ? 1 : 0) : x)); return this; }
  async all() { const st = this.db.prepare(this.sql); return { results: st.all(...this.args).map((r) => ({ ...r })), success: true, meta: {} }; }
  async first() { const r = this.db.prepare(this.sql).get(...this.args); return r ? { ...r } : null; }
  async run() { const r = this.db.prepare(this.sql).run(...this.args); return { success: true, meta: { changes: Number(r.changes), last_row_id: Number(r.lastInsertRowid) } }; }
  _runSync() { return this.db.prepare(this.sql).run(...this.args); }
}
export class FakeD1 {
  constructor() { this.db = new DatabaseSync(':memory:'); }
  prepare(sql) { return new Stmt(this.db, sql); }
  exec(sql) { this.db.exec(sql); }
  async batch(stmts) {
    const out = [];
    this.db.exec('BEGIN');
    try { for (const s of stmts) { const r = s._runSync(); out.push({ success: true, meta: { changes: Number(r.changes), last_row_id: Number(r.lastInsertRowid) } }); } this.db.exec('COMMIT'); }
    catch (e) { this.db.exec('ROLLBACK'); throw e; }
    return out;
  }
}
export class FakeR2 {
  constructor() { this.m = new Map(); }
  async put(key, body, opts = {}) {
    let buf;
    if (body instanceof ArrayBuffer) buf = new Uint8Array(body);
    else if (ArrayBuffer.isView(body)) buf = new Uint8Array(body.buffer, body.byteOffset, body.byteLength);
    else if (typeof body === 'string') buf = new TextEncoder().encode(body);
    else buf = new Uint8Array(await new Response(body).arrayBuffer());
    const o = { key, buf: new Uint8Array(buf), size: buf.byteLength, httpMetadata: opts.httpMetadata || {}, uploaded: new Date(), etag: 'e' + buf.byteLength + '-' + (this.m.size + 1) };
    this.m.set(key, o);
    return { key, size: o.size, etag: o.etag };
  }
  async get(key) {
    const o = this.m.get(key);
    if (!o) return null;
    return { key, size: o.size, etag: o.etag, httpEtag: `"${o.etag}"`, httpMetadata: o.httpMetadata, uploaded: o.uploaded, body: new Response(o.buf).body, arrayBuffer: async () => o.buf.slice().buffer, text: async () => new TextDecoder().decode(o.buf) };
  }
  async head(key) { const o = this.m.get(key); return o ? { key, size: o.size, etag: o.etag, httpEtag: `"${o.etag}"`, httpMetadata: o.httpMetadata, uploaded: o.uploaded } : null; }
  async delete(keys) { for (const k of Array.isArray(keys) ? keys : [keys]) this.m.delete(k); }
  async list({ prefix = '', cursor, limit = 1000 } = {}) {
    const all = [...this.m.keys()].filter((k) => k.startsWith(prefix)).sort();
    const start = cursor ? parseInt(cursor, 10) : 0;
    const slice = all.slice(start, start + limit);
    return { objects: slice.map((k) => ({ key: k, size: this.m.get(k).size, uploaded: this.m.get(k).uploaded, httpMetadata: this.m.get(k).httpMetadata })), truncated: start + limit < all.length, cursor: String(start + limit) };
  }
}
const MIME = { '.png': 'image/png', '.css': 'text/css', '.js': 'text/javascript', '.svg': 'image/svg+xml', '.ico': 'image/x-icon' };
// ASSETS baglamasi: web/public klasorunu sunar
const fakeAssets = { async fetch(req) {
  const p = join(ROOT, 'public', new URL(req.url).pathname.replace(/^\/+/, ''));
  if (p.startsWith(join(ROOT, 'public')) && existsSync(p)) return new Response(readFileSync(p), { headers: { 'content-type': MIME[extname(p)] || 'application/octet-stream' } });
  return new Response('yok', { status: 404 });
} };

export function makeEnv(extra = {}) {
  const DB = new FakeD1();
  DB.exec(readFileSync(join(ROOT, 'schema.sql'), 'utf8'));
  return { DB, MEDIA: new FakeR2(), ASSETS: fakeAssets, SITE_TOKEN: 'test-token', SITE_URL: 'https://test.example', ...extra };
}
export const req = (path, init = {}) => new Request('https://test.example' + path, init);
export const authHeaders = (extra = {}) => ({ authorization: 'Bearer test-token', ...extra });
