// Cloudflare Worker girisi: /api/sync (program), /media /img (R2), /admin (yonetim), geri kalani genel site.
import { jsonResponse, textResponse } from './util.js';
import { handleSync } from './sync.js';
import { handleMedia } from './media.js';
import { handleAdmin } from './admin.js';
import { handlePublic } from './public.js';
import { ensureSeed } from './seed.js';

export async function route(request, env, ctx) {
  const url = new URL(request.url);
  const p = url.pathname;
  try {
    if (p.startsWith('/api/sync/')) return await handleSync(request, env, url);
    if (p.startsWith('/media/') || p.startsWith('/img/')) return await handleMedia(request, env, url);
    // statik varliklar (logo, css, js) ASSETS baglamasindan
    if (/^\/(logo\.png|favicon\.png|favicon\.ico|assets\/|static\/)/.test(p) || /\.(css|js|map|woff2?|svg|webmanifest)$/.test(p)) {
      if (env.ASSETS) return await env.ASSETS.fetch(request);
    }
    await ensureSeed(env); // ilk calismada varsayilan menu/sayfa/kategori/etiket/yazilar
    if (p === '/admin' || p.startsWith('/admin/')) return await handleAdmin(request, env, url);
    return await handlePublic(request, env, url, ctx);
  } catch (e) {
    console.error('HATA', p, e && e.stack || e);
    if (p.startsWith('/api/')) return jsonResponse({ ok: false, error: 'Sunucu hatası' }, 500);
    return textResponse('Beklenmeyen bir hata oluştu. Lütfen daha sonra tekrar deneyin.', 500);
  }
}
export default { fetch: (request, env, ctx) => route(request, env, ctx) };
