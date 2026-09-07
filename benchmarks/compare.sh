#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."  # repo root

# selected languages (empty = all)
REQUESTED=("$@")
want() {
    [ ${#REQUESTED[@]} -eq 0 ] && return 0
    local l
    for l in "${REQUESTED[@]}"; do [ "$l" = "$1" ] && return 0; done
    return 1
}

if [ ${#REQUESTED[@]} -gt 0 ]; then
    echo "running languages: ${REQUESTED[*]}"
fi

WARMUP=3
RESULTS="benchmarks/results"
mkdir -p "$RESULTS"

# embedded runners (one bin each). mimas itself is embedded the same way now (path dep into the
# main workspace). the koto/dyon/steel/boa/rustpython ones are other pure-Rust scripting languages
# shown only on the home page; they only run the physics test below.
RUN="benchmarks/runners/target/release"
MIMAS_RUN="$RUN/mimas-run"
RHAI_RUN="$RUN/rhai-run"
RUNE_RUN="$RUN/rune-run"
LUAU_RUN="$RUN/luau-run"
KOTO_RUN="$RUN/koto-run"
DYON_RUN="$RUN/dyon-run"
STEEL_RUN="$RUN/steel-run"
BOA_RUN="$RUN/boa-run"
RUSTPYTHON_RUN="$RUN/rustpython-run"
GML_RUN="$RUN/gml-run"

# when only some languages are requested, fold their timings into the saved results instead of
# replacing the file -- lets a single language be re-measured without re-running the whole suite
MERGE=0
[ ${#REQUESTED[@]} -gt 0 ] && MERGE=1

# build only the requested runner bins, so a luau-only run doesn't compile boa/rustpython
if [ -f benchmarks/runners/Cargo.toml ]; then
    runner_bins=()
    if want mimas;      then runner_bins+=( --bin mimas-run ); fi
    if want rhai;       then runner_bins+=( --bin rhai-run ); fi
    if want rune;       then runner_bins+=( --bin rune-run ); fi
    if want luau;       then runner_bins+=( --bin luau-run ); fi
    if want koto;       then runner_bins+=( --bin koto-run ); fi
    if want dyon;       then runner_bins+=( --bin dyon-run ); fi
    if want steel;      then runner_bins+=( --bin steel-run ); fi
    if want boa;        then runner_bins+=( --bin boa-run ); fi
    if want rustpython; then runner_bins+=( --bin rustpython-run ); fi
    if want fabricator; then runner_bins+=( --bin gml-run ); fi
    if [ ${#runner_bins[@]} -gt 0 ]; then
        echo "building embedded runners..."
        ( cd benchmarks/runners && cargo build --release --quiet "${runner_bins[@]}" ) || echo "  runner build failed; skipping embedded runtimes"
    fi
fi

# BENCHES=".." narrows which workloads run, so one language (or one repaired result) can be
# re-measured without sitting through the whole suite. results merge per the MERGE logic above.
ALL_BENCHES="strings physics mandelbrot prime_numbers fibonacci eval collections"
for bench in ${BENCHES:-$ALL_BENCHES}; do
    cmds=()
    if want mimas && [ -x "$MIMAS_RUN" ] && [ -f "benchmarks/$bench/$bench.mim" ]; then
        cmds+=( -n "mimas" "$MIMAS_RUN benchmarks/$bench/$bench.mim" )
    fi
    if want rune && [ -x "$RUNE_RUN" ] && [ -f "benchmarks/$bench/$bench.rn" ]; then
        cmds+=( -n "rune" "$RUNE_RUN benchmarks/$bench/$bench.rn" )
    fi
    if want rhai && [ -x "$RHAI_RUN" ] && [ -f "benchmarks/$bench/$bench.rhai" ]; then
        cmds+=( -n "rhai" "$RHAI_RUN benchmarks/$bench/$bench.rhai" )
    fi
    if want luau && [ -x "$LUAU_RUN" ] && [ -f "benchmarks/$bench/$bench.lua" ]; then
        cmds+=( -n "luau" "$LUAU_RUN benchmarks/$bench/$bench.lua" )
    fi

    # other pure-Rust scripting languages: physics + mandelbrot only, for the home-page charts.
    # rustpython reuses physics.py / mandelbrot.py; the rest have their own files.
    if [ "$bench" = physics ] || [ "$bench" = mandelbrot ]; then
        if want koto       && [ -x "$KOTO_RUN" ]       && [ -f "benchmarks/$bench/$bench.koto" ]; then cmds+=( -n "koto"       "$KOTO_RUN benchmarks/$bench/$bench.koto" ); fi
        if want dyon       && [ -x "$DYON_RUN" ]       && [ -f "benchmarks/$bench/$bench.dyon" ]; then cmds+=( -n "dyon"       "$DYON_RUN benchmarks/$bench/$bench.dyon" ); fi
        if want steel      && [ -x "$STEEL_RUN" ]      && [ -f "benchmarks/$bench/$bench.scm" ];  then cmds+=( -n "steel"      "$STEEL_RUN benchmarks/$bench/$bench.scm" ); fi
        if want boa        && [ -x "$BOA_RUN" ]        && [ -f "benchmarks/$bench/$bench.js" ];   then cmds+=( -n "boa"        "$BOA_RUN benchmarks/$bench/$bench.js" ); fi
        if want rustpython && [ -x "$RUSTPYTHON_RUN" ] && [ -f "benchmarks/$bench/$bench.py" ];   then cmds+=( -n "rustpython" "$RUSTPYTHON_RUN benchmarks/$bench/$bench.py" ); fi
        if want fabricator && [ -x "$GML_RUN" ]        && [ -f "benchmarks/$bench/$bench.gml" ];  then cmds+=( -n "fabricator" "$GML_RUN benchmarks/$bench/$bench.gml" ); fi
    fi

    if [ ${#cmds[@]} -eq 0 ]; then
        continue
    fi

    echo
    echo "================  $bench  ================"

    out="$RESULTS/results-$bench.json"
    if [ "$MERGE" = 1 ] && [ -f "$out" ]; then
        tmp="$out.new"
    else
        tmp="$out"
    fi

    hyperfine \
        --warmup "$WARMUP" \
        --shell=none \
        --prepare 'sleep 1' \
        --export-json "$tmp" \
        "${cmds[@]}"

    if [ "$tmp" != "$out" ]; then
        python3 - "$out" "$tmp" <<'PY'
import json, sys

old_path, new_path = sys.argv[1], sys.argv[2]
old = json.load(open(old_path))
new = json.load(open(new_path))

# key on the hyperfine display name (`-n`), so a re-measured language replaces its own entry
# and a new one is appended; every other language keeps its saved timing
merged = {r["command"]: r for r in old["results"]}
for r in new["results"]:
    merged[r["command"]] = r
old["results"] = list(merged.values())

json.dump(old, open(old_path, "w"))
print(f"  merged into {old_path}: {', '.join(sorted(merged))}")
PY
        rm -f "$tmp"
    fi
done

echo
echo "per-workload json written to $RESULTS/results-*.json"
