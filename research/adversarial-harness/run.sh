#!/usr/bin/env bash
# Adversarial partitioning harness. Reproduce with: ./run.sh
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p raw
SEED="${SEED:-20260910}"
SAMPLES="${SAMPLES:-20000}"
{
  echo "# environment"; rustc --version; cargo --version; uname -sr
  echo "# date (UTC)"; date -u +%Y-%m-%dT%H:%M:%SZ
  echo
  cargo run --quiet --release -- --seed "$SEED" --samples "$SAMPLES"
} > raw/output.txt 2> raw/stderr.txt
cat raw/output.txt
