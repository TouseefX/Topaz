#!/usr/bin/env bash
# Decompile every Lua file in a corpus directory and report quality metrics.
#
# usage: decompile_corpus.sh TOPAZ_BIN LUAU_COMPILE CORPUS_DIR OUT_DIR
#
# For each <name>.lua:
#   1. luau-compile -O1 -g2 --binary  -> <name>.bin      (the input we decompile)
#   2. topaz decompile                -> <name>.decompiled.lua
#   3. luau-compile <decompiled>      -> does the decompiled text still parse?
# Writes OUT_DIR/SUMMARY.md with one row per file.
set -u

TOPAZ="$1"
LUAUC="$2"
CORPUS="$3"
OUT="$4"

mkdir -p "$OUT"
SUMMARY="$OUT/SUMMARY.md"

{
  echo "| file | topaz exit | lines | goto | -- block | INCOMPLETE | recompiles |"
  echo "|---|---|---|---|---|---|---|"
} > "$SUMMARY"

for src in "$CORPUS"/*.lua; do
  name="$(basename "$src" .lua)"
  bin="$OUT/$name.bin"
  dec="$OUT/$name.decompiled.lua"

  if ! "$LUAUC" -O1 -g2 --binary "$src" > "$bin" 2> "$OUT/$name.compile-err.txt"; then
    echo "| $name | (luau-compile failed) | | | | | |" >> "$SUMMARY"
    continue
  fi

  timeout 300 "$TOPAZ" decompile -i "$bin" -o "$dec" --time-budget 120 \
    > "$OUT/$name.topaz.log" 2>&1
  rc=$?

  lines=$(wc -l < "$dec" 2>/dev/null || echo 0)
  gotos=$(grep -c 'goto' "$dec" 2>/dev/null || true)
  blocks=$(grep -c -- '-- block' "$dec" 2>/dev/null || true)
  incomplete=$(grep -c 'TOPAZ_INCOMPLETE' "$dec" 2>/dev/null || true)

  if [ -f "$dec" ] && "$LUAUC" "$dec" > /dev/null 2> "$OUT/$name.recompile-err.txt"; then
    recompiles=yes
  else
    recompiles=NO
  fi

  echo "| $name | $rc | ${lines:-0} | ${gotos:-0} | ${blocks:-0} | ${incomplete:-0} | $recompiles |" >> "$SUMMARY"
done

cat "$SUMMARY"
