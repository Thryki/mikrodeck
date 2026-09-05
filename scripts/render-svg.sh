#!/usr/bin/env bash
# Converte design/mikro-mk3.svg em design/mikro-mk3.png (1920 px de largura).
# Usa rsvg-convert, resvg ou o fallback em Node (scripts/render-svg.js, requer `npm i` em scripts/).
set -e
cd "$(dirname "$0")/.."
IN=design/mikro-mk3.svg
OUT=design/mikro-mk3.png
if command -v rsvg-convert >/dev/null; then
  rsvg-convert -w 1920 "$IN" -o "$OUT"
  echo "Gerado: $OUT"
elif command -v resvg >/dev/null; then
  resvg -w 1920 "$IN" "$OUT"
  echo "Gerado: $OUT"
elif [ -f scripts/node_modules/@resvg/resvg-js/package.json ]; then
  node scripts/render-svg.js "$IN" "$OUT" 1920
else
  echo "Instale rsvg-convert (brew install librsvg), resvg (cargo install resvg) ou rode 'npm i' em scripts/"; exit 1
fi
if [ -f scripts/node_modules/sharp/package.json ]; then
  node scripts/compare.js
fi
