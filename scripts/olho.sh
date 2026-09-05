#!/usr/bin/env bash
# Tira uma foto do Mikro MK3 pela webcam e salva com o nome dado.
# Uso: scripts/olho.sh <nome>
FFMPEG="$HOME/AppData/Local/Microsoft/WinGet/Packages/Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe/ffmpeg-9.0.1-full_build/bin/ffmpeg.exe"
CAM="Insta360 Link 2C"
OUT_DIR="${OLHO_DIR:-$HOME/AppData/Local/Temp/claude/C--Users-<usuario>-Documents-MikroDeck/8d18aff7-a061-4449-8e3a-4f5673d70acf/scratchpad/cam}"
mkdir -p "$OUT_DIR"
NOME="${1:-frame}"
# hflip+vflip gira 180 graus (uma vez so): a webcam está apontada com o aparelho de cabeça para baixo.
"$FFMPEG" -hide_banner -loglevel error -f dshow -video_size 1920x1080 -i video="$CAM" \
  -frames:v 1 -vf "hflip,vflip" -y "$OUT_DIR/$NOME.jpg" 2>&1 | tail -3
echo "$OUT_DIR/$NOME.jpg"
