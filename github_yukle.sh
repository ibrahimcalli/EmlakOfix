#!/usr/bin/env bash
# Depoyu GitHub'a yükler. Önce: git config --global user.name "Adınız" ; git config --global user.email "mail@adresiniz"
# Kimlik doğrulama: HTTPS için GitHub "Personal access token" (Settings > Developer settings) veya `gh auth login`.
set -e
cd "$(dirname "$0")"
[ -d .git ] || git init -b main
git add -A
echo "--- Eklenecek dosyalar (kontrol edin; .db / .zip / .env görmemelisiniz) ---"
git status --short
read -p "Devam edilsin mi? (e/h) " c; [ "$c" = "e" ] || exit 1
git commit -m "Emlak Ofis v$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)" || true
git remote get-url origin >/dev/null 2>&1 || git remote add origin https://github.com/ibrahimcalli/EmlakOfix.git
git push -u origin main
