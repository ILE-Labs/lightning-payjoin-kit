#!/usr/bin/env bash
# Experiment 01 — reproduce with: ./run.sh
# Deterministic: no RNG. Output is byte-identical across runs on the same toolchain.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p raw
{
  echo "# environment"
  rustc --version
  cargo --version
  uname -sr
  echo "# date (UTC)"
  date -u +%Y-%m-%dT%H:%M:%SZ
  echo
  cargo run --quiet --release
} > raw/output.txt 2>raw/stderr.txt
cat raw/output.txt
