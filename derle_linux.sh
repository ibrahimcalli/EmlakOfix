#!/usr/bin/env bash
# Linux (Mint/Ubuntu/Debian) için derleme betiği
set -e
sudo apt update
sudo apt install -y build-essential pkg-config libxkbcommon-dev libwayland-dev curl \
    xdg-desktop-portal xdg-desktop-portal-gtk zenity
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  . "$HOME/.cargo/env"
fi
cargo build --release
mkdir -p "$HOME/EmlakOfis"
cp target/release/emlak-ofis "$HOME/EmlakOfis/"
cp assets/icon.png "$HOME/EmlakOfis/emlak-ofis.png"
mkdir -p "$HOME/.local/share/applications"
cat > "$HOME/.local/share/applications/emlak-ofis.desktop" <<DESK
[Desktop Entry]
Type=Application
Name=Emlak Ofis
Exec=$HOME/EmlakOfis/emlak-ofis
Icon=$HOME/EmlakOfis/emlak-ofis.png
StartupWMClass=emlak-ofis
Terminal=false
Categories=Office;
DESK
echo "Tamam. Program: $HOME/EmlakOfis/emlak-ofis  (uygulama menüsünde 'Emlak Ofis')"
