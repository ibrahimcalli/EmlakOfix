#!/usr/bin/env bash
# Emlak Ofis kaynak kodunu GitHub'a yükler (https://github.com/ibrahimcalli/EmlakOfix).
# Gerekenler: git kurulu olmalı; GitHub'da "Personal access token" (parola yerine) veya `gh auth login`.
set -e
cd "$(dirname "$0")"
git config user.name  >/dev/null 2>&1 || read -p "Adınız (commit için): " n && git config --global user.name  "${n:-$(git config user.name)}" 2>/dev/null || true
git config user.email >/dev/null 2>&1 || { read -p "E-posta (commit için): " m; git config --global user.email "$m"; }
[ -d .git ] || git init -b main
git add -A
echo
echo "--- Yüklenecek dosyalar (.db, .zip, .env, .gizli, ayarlar.json GÖRMEMELİSİNİZ) ---"
git status --short
if git status --short | grep -E '\.(db|zip|env|gizli|key|pem)$|ayarlar\.json|yedekler/'; then
  echo "UYARI: gizli olabilecek dosya listede! Durduruldu."; exit 2
fi
read -p "Devam edilsin mi? (e/h) " c; [ "$c" = "e" ] || exit 1
git commit -m "Emlak Ofis v$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)" || true
git remote get-url origin >/dev/null 2>&1 || git remote add origin https://github.com/ibrahimcalli/EmlakOfix.git
git branch -M main
git push -u origin main
echo "Tamam: https://github.com/ibrahimcalli/EmlakOfix"
