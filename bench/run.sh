#!/usr/bin/env bash
# Runs crunchit over a copy of bench/images/ and prints a markdown savings table.
set -euo pipefail

cd "$(dirname "$0")/.."
if ! ls bench/images/* >/dev/null 2>&1; then
    echo "Put sample images in bench/images/ first." >&2
    exit 1
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp bench/images/* "$work"/

cargo build --release --quiet

declare -A before
for f in "$work"/*; do
    before[$(basename "$f")]=$(stat -c%s "$f")
done

start=$(date +%s.%N)
./target/release/crunchit "$work" >/dev/null
elapsed=$(echo "$(date +%s.%N) - $start" | bc)

echo
echo "| File | Before | After | Saved |"
echo "|---|---|---|---|"
total_b=0; total_a=0
for f in "$work"/*; do
    name=$(basename "$f")
    b=${before[$name]}
    a=$(stat -c%s "$f")
    total_b=$((total_b + b)); total_a=$((total_a + a))
    pct=$(echo "scale=1; ($b - $a) * 100 / $b" | bc)
    printf '| %s | %s | %s | %s%% |\n' "$name" "$(numfmt --to=iec "$b")" "$(numfmt --to=iec "$a")" "$pct"
done
pct=$(echo "scale=1; ($total_b - $total_a) * 100 / $total_b" | bc)
printf '| **Total** | %s | %s | **%s%%** |\n' "$(numfmt --to=iec "$total_b")" "$(numfmt --to=iec "$total_a")" "$pct"
echo
echo "Wall time: ${elapsed}s"
