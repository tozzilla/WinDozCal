#!/usr/bin/env bash
# WinDozCal: verifica prerequisiti, installa dipendenze, controlla il progetto.
# Uso: bash init.sh [--dev]   (--dev avvia `npm run tauri dev` a fine verifica)
# Pensato per Git Bash su Windows 11.

cd "$(dirname "$0")" || exit 1

DEV=0
[ "${1:-}" = "--dev" ] && DEV=1

# rustup installa in ~/.cargo/bin, che una shell già aperta può non avere nel PATH
export PATH="$HOME/.cargo/bin:$PATH"

MISSING=()
FAILED=()

have() { command -v "$1" >/dev/null 2>&1; }

missing() { # nome, comando di installazione
  echo "MANCA  $1"
  echo "       installa con: $2"
  MISSING+=("$1")
}

step() { # nome, comando...
  local name="$1"; shift
  echo
  echo "==> $name"
  if "$@"; then
    echo "OK     $name"
  else
    echo "FALLITO $name"
    FAILED+=("$name")
  fi
}

echo "== Prerequisiti =="

if have node; then echo "OK     node $(node -v)"; else
  missing "node" "winget install --id OpenJS.NodeJS.LTS -e"
fi
if have npm; then echo "OK     npm $(npm -v)"; else
  missing "npm" "winget install --id OpenJS.NodeJS.LTS -e (include npm)"
fi

HAVE_CARGO=0
if have cargo; then echo "OK     cargo $(cargo -V)"; HAVE_CARGO=1; else
  missing "cargo" "winget install --id Rustlang.Rustup -e  (poi riapri il terminale)"
fi
if have rustc; then echo "OK     rustc $(rustc -V)"; else
  missing "rustc" "winget install --id Rustlang.Rustup -e  (poi riapri il terminale)"
fi

VSWHERE="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
MSVC=""
if [ -x "$VSWHERE" ]; then
  MSVC=$("$VSWHERE" -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>/dev/null | head -n1)
fi
if [ -n "$MSVC" ]; then
  echo "OK     MSVC Build Tools ($MSVC)"
else
  missing "MSVC Build Tools (workload C++)" \
    "winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override \"--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended\""
fi

# WebView2 Runtime: preinstallato su Windows 11
WV2_KEY='HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
if reg query "$WV2_KEY" //v pv >/dev/null 2>&1 || reg query "${WV2_KEY/HKLM/HKCU}" //v pv >/dev/null 2>&1; then
  echo "OK     WebView2 Runtime"
else
  missing "WebView2 Runtime" "winget install --id Microsoft.EdgeWebView2Runtime -e"
fi

if ! have node || ! have npm; then
  echo
  echo "Node/npm assenti: impossibile proseguire con i passi npm."
  exit 1
fi

echo
echo "== Verifica progetto =="
step "npm install" npm install
step "npm run typecheck" npm run typecheck
step "npm test" npm test

if [ "$HAVE_CARGO" -eq 1 ] && [ -n "$MSVC" ]; then
  step "cargo check" cargo check --manifest-path src-tauri/Cargo.toml
else
  echo
  echo "SALTATO cargo check (richiede cargo e MSVC Build Tools)"
fi

echo
echo "== Riepilogo =="
[ ${#MISSING[@]} -gt 0 ] && printf 'Prerequisiti mancanti: %s\n' "$(IFS=,; echo "${MISSING[*]}")"
[ ${#FAILED[@]} -gt 0 ] && printf 'Passi falliti: %s\n' "$(IFS=,; echo "${FAILED[*]}")"
if [ ${#MISSING[@]} -eq 0 ] && [ ${#FAILED[@]} -eq 0 ]; then
  echo "Tutto verde."
else
  [ "$DEV" -eq 1 ] && echo "--dev ignorato: sistema prima i punti sopra."
  exit 1
fi

if [ "$DEV" -eq 1 ]; then
  echo
  echo "==> npm run tauri dev"
  exec npm run tauri dev
fi
