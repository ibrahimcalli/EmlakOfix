#!/usr/bin/env python3
"""Güncelleme paketi (.zip) üretir.

Kullanım:
  python3 arac/paketle_guncelleme.py --surum 1.3.0 --linux yol/emlak-ofis [--windows yol/emlak-ofis.exe] \
      [--notlar "Bu sürümde neler değişti"] [--cikti EmlakOfis-Guncelleme-1.3.0.zip]

Paketin içeriği: surum.json + linux/emlak-ofis + windows/emlak-ofis.exe (SHA-256 özetleriyle).
Programda: Ayarlar > Hakkında ve güncelleme > "Güncelleme paketi seç (.zip)".
"""
import argparse, hashlib, json, sys, zipfile

ap = argparse.ArgumentParser()
ap.add_argument("--surum", required=True)
ap.add_argument("--linux")
ap.add_argument("--windows")
ap.add_argument("--notlar", default="")
ap.add_argument("--cikti")
a = ap.parse_args()
if not a.linux and not a.windows:
    sys.exit("En az bir platform dosyası verin (--linux ve/veya --windows).")

paketler, dosyalar = {}, []
for plat, yol, ad in (("linux", a.linux, "linux/emlak-ofis"), ("windows", a.windows, "windows/emlak-ofis.exe")):
    if yol:
        veri = open(yol, "rb").read()
        if len(veri) < 1_000_000:
            sys.exit(f"{yol} çok küçük; doğru program dosyası mı?")
        paketler[plat] = {"dosya": ad, "sha256": hashlib.sha256(veri).hexdigest()}
        dosyalar.append((ad, veri))

cikti = a.cikti or f"EmlakOfis-Guncelleme-{a.surum}.zip"
with zipfile.ZipFile(cikti, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("surum.json", json.dumps({"surum": a.surum, "notlar": a.notlar, "paketler": paketler}, ensure_ascii=False, indent=2))
    for ad, veri in dosyalar:
        z.writestr(ad, veri)
print("Hazır:", cikti, "| platformlar:", ", ".join(paketler))
