#!/usr/bin/env bash
# Decompile every Lua file in a corpus directory and report quality metrics.
#
# usage: decompile_corpus.sh TOPAZ_BIN LUAU_COMPILE CORPUS_DIR OUT_DIR [LUAU_REPL]
#
# For each <name>.lua:
#   1. luau-compile -O1 -g2 --binary  -> <name>.bin      (the input we decompile)
#   2. topaz decompile                -> <name>.decompiled.lua
#   3. luau-compile <decompiled>      -> does the decompiled text still parse?
#   4. LUAU_REPL (optional): run the original source and the decompiled
#      source; "same output" = identical stdout and exit code.
# Writes OUT_DIR/SUMMARY.md with one row per file.
set -u

TOPAZ="$1"
LUAUC="$2"
CORPUS="$3"
OUT="$4"
REPL="${5:-}"

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"  # absolute: run_as_prog changes directory
SUMMARY="$OUT/SUMMARY.md"

{
  echo "| file | topaz exit | lines | goto | -- block | INCOMPLETE | recompiles | same output |"
  echo "|---|---|---|---|---|---|---|---|"
} > "$SUMMARY"

# Run $1 (a .lua file) as prog.lua inside a fresh temp dir, so error messages
# are identical no matter where the file came from. Output goes to $2.
run_as_prog() {
  local src="$1" dst="$2" dir
  dir="$(mktemp -d)"
  cp "$src" "$dir/prog.lua"
  (cd "$dir" && timeout 60 "$REPL" prog.lua > "$dst" 2>&1; echo "exit=$?" >> "$dst")
  rm -rf "$dir"
}

for src in "$CORPUS"/*.lua; do
  name="$(basename "$src" .lua)"
  bin="$OUT/$name.bin"
  dec="$OUT/$name.decompiled.lua"

  if ! "$LUAUC" -O1 -g2 --binary "$src" > "$bin" 2> "$OUT/$name.compile-err.txt"; then
    echo "| $name | (luau-compile failed) | | | | | | |" >> "$SUMMARY"
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

  same=n/a
  if [ -n "$REPL" ] && [ -f "$dec" ]; then
    run_as_prog "$src" "$OUT/$name.orig.out"
    run_as_prog "$dec" "$OUT/$name.dec.out"
    if cmp -s "$OUT/$name.orig.out" "$OUT/$name.dec.out"; then
      same=yes
    else
      same=NO
      diff "$OUT/$name.orig.out" "$OUT/$name.dec.out" > "$OUT/$name.diff" 2>&1 || true
    fi
  fi

  echo "| $name | $rc | ${lines:-0} | ${gotos:-0} | ${blocks:-0} | ${incomplete:-0} | $recompiles | $same |" >> "$SUMMARY"
done

cat "$SUMMARY"
