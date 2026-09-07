#!/usr/bin/env bash
#
# Times the whole compile pipeline (parse, type-check, lower, emit bytecode) over generated
# source, producing the numbers behind the "Compile speed" table in the book. Unlike compare.sh
# this needs no hyperfine -- `mimas build` reports its own timing, and we take the best of N to
# cut scheduler noise.
#
# usage: benchmarks/compile-bench.sh [runs]   (default 5)

set -euo pipefail

cd "$(dirname "$0")/.."  # repo root

RUNS=${1:-5}
MIMAS=target/release/mimas
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

echo "building mimas (release)..."
cargo build --release -p mimas-cli --quiet

# "a small module" / "a project" / "a large project" in the book's table
SIZES=(100 10000 100000)

printf "\n%-16s %10s %12s %16s\n" "target lines" "actual" "best compile" "lines/sec"
printf -- "------------------------------------------------------------\n"

for size in "${SIZES[@]}"; do
    out="$WORK/fodder_$size"
    "$MIMAS" run tools/fodder -- "$size" "$out" >/dev/null

    # `mimas build` prints e.g. "Compiled 10,183 lines in 16ms." -- the sub-millisecond unit is a
    # non-ascii micro sign, so match on what it is *not* rather than on the character itself
    best_us=""
    lines=""
    for _ in $(seq "$RUNS"); do
        report=$("$MIMAS" build "$out" | grep 'Compiled')
        lines=$(printf '%s' "$report" | sed -E 's/.*Compiled ([0-9,]+) lines.*/\1/' | tr -d ',')
        value=$(printf '%s' "$report" | sed -E 's/.*lines in (.+)\.$/\1/')
        us=$(printf '%s' "$value" | awk '{
            n = $0 + 0
            if ($0 ~ /ms$/)            printf "%.0f", n * 1000
            else if ($0 ~ /^[0-9.]+s$/) printf "%.0f", n * 1000000
            else                        printf "%.0f", n
        }')
        if [ -z "$best_us" ] || [ "$us" -lt "$best_us" ]; then best_us=$us; fi
    done

    human=$(awk -v us="$best_us" 'BEGIN {
        if (us < 1000) printf "< 1 ms";
        else if (us < 1000000) printf "~%d ms", us / 1000;
        else printf "~%.2f s", us / 1000000;
    }')
    rate=$(awk -v l="$lines" -v us="$best_us" 'BEGIN { printf "%d", l / (us / 1000000) }')
    printf "%-16s %10s %12s %16s\n" "~$size" "$lines" "$human" "$rate"
done

printf -- "------------------------------------------------------------\n"
echo "best of $RUNS runs per size. update the Compile speed table in"
echo "book/src/introduction/benchmarks.md with the two right-hand columns."
