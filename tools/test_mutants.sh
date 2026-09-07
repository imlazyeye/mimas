#!/usr/bin/env bash
set -euo pipefail

export CARGO_TARGET_DIR=target
export CARGO_PROFILE_DEV_OPT_LEVEL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_DEV_SPLIT_DEBUGINFO=off
export CARGO_PROFILE_DEV_INCREMENTAL=true
export CARGO_PROFILE_DEV_CODEGEN_UNITS=256

cargo mutants -v \
    --package mimas-parse \
    --package mimas-solve \
    --package mimas-compile \
    --package mimas-vm \
    --package mimas-shared \
    --output mutants \
    --test-workspace true