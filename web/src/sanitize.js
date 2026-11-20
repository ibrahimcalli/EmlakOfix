// Beyaz liste tabanli HTML temizleyici (blog/sayfa govdesi). script/style/iframe ve olay ozellikleri atilir.
import { esc, safeUrl } from './util.js';
const ALLOWED = {
  p: [], br: [], b: [], strong: [], i: [], em: [], u: [], s: [], h2: [], h3: [], h4: [], ul: [], ol: [], li: [], blockquote: [], hr: [],
  a: ['href', 'title', 'target', 'rel'], img: ['src', 'alt', 'width', 'height', 'loading', 'title'],
  table: [], thead: [], tbody: [], tr: [], th: ['colspan', 'rowspan'], td: ['colspan', 'rowspan'],
  figure: [], figcaption: [], span: [], div: [], code: [], pre: [], small: [], sup: [], sub: [],
};
const DROP_WITH_CONTENT = new Set(['script', 'style', 'iframe', 'object', 'embed', 'noscript', 'template', 'svg', 'math', 'form', 'textarea', 'select', 'button', 'title', 'head']);
const VOID = new Set(['br', 'hr', 'img']);
const ALIAS = { h1: 'h2', h5: 'h4', h6: 'h4', strike: 's', del: 's', center: 'div', font: 'span' };

export function sanitizeHtml(input) {
  let src = String(input ?? '').replace(/<!--[\s\S]*?-->/g, '');
  const out = [];
  const stack = [];
  let skip = null; // icerigiyle atilan etiket
  const re = /<\/?([a-zA-Z][a-zA-Z0-9]*)((?:"[^"]*"|'[^']*'|[^'">])*)>|([^<]+)|</g;
  let m;
  while ((m = re.exec(src))) {
    if (m[3] !== undefined) { if (!skip) out.push(esc(m[3].replace(/&(?!(?:[a-z]+|#\d+|#x[0-9a-f]+);)/gi, '&amp;')).replace(/&amp;amp;/g, '&amp;').replace(/&amp;(#?\w+);/g, '&$1;')); continue; }
    if (m[1] === undefined) { if (!skip) out.push('&lt;'); continue; }
    const closing = m[0][1] === '/';
    let tag = m[1].toLowerCase();
    if (skip) { if (closing && tag === skip) skip = null; continue; }
    if (DROP_WITH_CONTENT.has(tag)) { if (!closing && !/\/\s*>$/.test(m[0])) skip = tag; continue; }
    tag = ALIAS[tag] || tag;
    if (!(tag in ALLOWED)) continue; // etiket atilir, icerik kalir
    if (closing) {
      const i = stack.lastIndexOf(tag);
      if (i >= 0) { while (stack.length > i) out.push(`</${stack.pop()}>`); }
      continue;
    }
    const attrs = [];
    const are = /([a-zA-Z_:][-a-zA-Z0-9_:.]*)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;
    let a;
    while ((a = are.exec(m[2] || ''))) {
      const name = a[1].toLowerCase();
      if (!ALLOWED[tag].includes(name)) continue;
      let val = a[2] ?? a[3] ?? a[4] ?? '';
      val = val.replace(/&amp;/g, '&');
      if (name === 'href' || name === 'src') { val = safeUrl(val); if (!val) continue; }
      if (name === 'target') val = val === '_blank' ? '_blank' : '';
      if (name === 'colspan' || name === 'rowspan' || name === 'width' || name === 'height') { if (!/^\d{1,4}$/.test(val)) continue; }
      if (!val && name !== 'alt') continue;
      attrs.push(` ${name}="${esc(val)}"`);
    }
    if (tag === 'a' && attrs.some((x) => x.includes('target="_blank"'))) attrs.push(' rel="noopener noreferrer"');
    if (tag === 'img' && !attrs.some((x) => x.startsWith(' alt='))) attrs.push(' alt=""');
    if (tag === 'img' && !attrs.some((x) => x.startsWith(' loading='))) attrs.push(' loading="lazy"');
    if (tag === 'img' && !attrs.some((x) => x.startsWith(' src='))) continue;
    out.push(`<${tag}${attrs.join('')}>`);
    if (!VOID.has(tag)) stack.push(tag);
  }
  while (stack.length) out.push(`</${stack.pop()}>`);
  return out.join('');
}
