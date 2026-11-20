// R2'den medya sunumu: GET|HEAD /media/<ad> (R2 m/<ad>) ve /img/l/<id>/<n>.jpg (R2 l/<id>/<n>.jpg).
import { MIME_BY_EXT } from './sync.js';

const CACHE_MEDIA = 'public, max-age=2592000, immutable'; // medya adları benzersiz
const CACHE_PHOTO = 'public, max-age=86400, stale-while-revalidate=604800'; // aynı adrese yeniden yüklenebilir
const NAME_RE = /^[A-Za-z0-9][A-Za-z0-9._-]{0,150}$/;
const PHOTO_RE = /^l\/\d{1,15}\/\d{1,3}\.jpg$/;

const plain = (msg, status, extra = {}) => new Response(msg, { status, headers: { 'content-type': 'text/plain; charset=utf-8', 'x-content-type-options': 'nosniff', ...extra } });

// İstek yolundan R2 anahtarı üretir; güvenli değilse null.
export function keyFromPath(pathname) {
  let p;
  try { p = decodeURIComponent(pathname); } catch { return null; }
  if (/[\u0000-\u001f\u007f\\]/.test(p)) return null;
  if (p.startsWith('/media/')) {
    const name = p.slice('/media/'.length);
    if (!NAME_RE.test(name) || name.includes('..')) return null;
    return `m/${name}`;
  }
  if (p.startsWith('/img/')) {
    const key = p.slice('/img/'.length);
    return PHOTO_RE.test(key) ? key : null;
  }
  return null;
}
function etagMatches(header, etag) {
  if (!header) return false;
  if (header.trim() === '*') return true;
  const bare = (s) => s.trim().replace(/^W\//, '');
  return header.split(',').some((x) => bare(x) === bare(etag));
}
function contentType(obj, key) {
  const ct = obj.httpMetadata && obj.httpMetadata.contentType;
  if (ct && /^image\/(jpeg|png|webp|gif)$/.test(ct)) return ct;
  const ext = key.split('.').pop().toLowerCase();
  return MIME_BY_EXT[ext === 'jpeg' ? 'jpg' : ext] || 'application/octet-stream';
}

export async function handleMedia(request, env, url) {
  const method = request.method.toUpperCase();
  if (method !== 'GET' && method !== 'HEAD') return plain('Yöntem desteklenmiyor.', 405, { allow: 'GET, HEAD' });
  const key = keyFromPath(url.pathname);
  if (!key) return plain('Bulunamadı.', 404);
  const inm = request.headers.get('if-none-match');
  const obj = method === 'HEAD' || inm ? await env.MEDIA.head(key) : await env.MEDIA.get(key);
  if (!obj) return plain('Bulunamadı.', 404);
  const etag = obj.httpEtag || `"${obj.etag}"`;
  const headers = {
    etag,
    'cache-control': key.startsWith('m/') ? CACHE_MEDIA : CACHE_PHOTO,
    'content-type': contentType(obj, key),
    'x-content-type-options': 'nosniff',
    'content-length': String(obj.size),
  };
  if (obj.uploaded) headers['last-modified'] = new Date(obj.uploaded).toUTCString();
  if (etagMatches(inm, etag)) {
    delete headers['content-length'];
    return new Response(null, { status: 304, headers });
  }
  if (method === 'HEAD') return new Response(null, { status: 200, headers });
  const full = obj.body ? obj : await env.MEDIA.get(key); // If-None-Match eşleşmediyse gövdeyi şimdi al
  if (!full) return plain('Bulunamadı.', 404);
  return new Response(full.body, { status: 200, headers });
}
