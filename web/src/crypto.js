// WebCrypto tabanli yardimcilar (Workers ve Node'da ortak).
const enc = new TextEncoder();
export const toHex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');
export const fromHex = (h) => new Uint8Array((h.match(/../g) || []).map((x) => parseInt(x, 16)));

export function randomHex(bytes = 32) {
  const a = new Uint8Array(bytes);
  crypto.getRandomValues(a);
  return toHex(a);
}
export async function sha256Hex(s) {
  return toHex(await crypto.subtle.digest('SHA-256', enc.encode(s)));
}
export async function hmacHex(key, msg) {
  const k = await crypto.subtle.importKey('raw', enc.encode(key), { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
  return toHex(await crypto.subtle.sign('HMAC', k, enc.encode(msg)));
}
// Sabit zamanli karsilastirma (uzunluk farki da gizlenir: iki tarafin HMAC'i karsilastirilir).
export async function safeEqual(a, b) {
  const key = randomHex(16);
  const [x, y] = await Promise.all([hmacHex(key, String(a ?? '')), hmacHex(key, String(b ?? ''))]);
  let d = 0;
  for (let i = 0; i < x.length; i++) d |= x.charCodeAt(i) ^ y.charCodeAt(i);
  return d === 0;
}

const ITER = 100000; // Cloudflare Workers ust siniri
async function pbkdf2(password, salt, iter) {
  const k = await crypto.subtle.importKey('raw', enc.encode(password), 'PBKDF2', false, ['deriveBits']);
  return toHex(await crypto.subtle.deriveBits({ name: 'PBKDF2', hash: 'SHA-256', salt, iterations: iter }, k, 256));
}
export async function hashPassword(password) {
  const salt = new Uint8Array(16);
  crypto.getRandomValues(salt);
  return `pbkdf2$${ITER}$${toHex(salt)}$${await pbkdf2(password, salt, ITER)}`;
}
export async function verifyPassword(password, stored) {
  const p = String(stored || '').split('$');
  if (p.length !== 4 || p[0] !== 'pbkdf2') { await pbkdf2(password, new Uint8Array(16), ITER); return false; }
  const h = await pbkdf2(password, fromHex(p[2]), Math.min(parseInt(p[1], 10) || ITER, ITER));
  return safeEqual(h, p[3]);
}
