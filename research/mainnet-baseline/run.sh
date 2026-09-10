#!/usr/bin/env bash
# Mainnet structural baseline. Reproduce with: ./run.sh
# Fetches the block sample if absent, then analyses it.
set -euo pipefail
cd "$(dirname "$0")"
# Keep the tree free of compiled bytecode.
export PYTHONDONTWRITEBYTECODE=1
mkdir -p raw
[ -f raw/manifest.json ] || python3 src/fetch_blocks.py
{
  echo "# environment"; python3 --version; uname -sr
  echo "# date (UTC)"; date -u +%Y-%m-%dT%H:%M:%SZ
  echo
  python3 src/analyse.py
} > raw/baseline.txt 2> raw/stderr.txt
cat raw/baseline.txt
