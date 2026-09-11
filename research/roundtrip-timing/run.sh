#!/usr/bin/env bash
# Experiment 06 — reproduce with: ./run.sh
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p raw
{
  echo "# environment"; rustc --version; cargo --version; uname -sr
  echo "# cpus: $(nproc)"
  echo "# date (UTC)"; date -u +%Y-%m-%dT%H:%M:%SZ
  echo
  cargo run --quiet --release -- --sessions "${SESSIONS:-20000}" --threads "${THREADS:-8}"
} > raw/output.txt 2> raw/stderr.txt
cat raw/output.txt
