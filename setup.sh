#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
missing=0
for tool in go cargo python3 npx npm pnpm uv uvx jq; do
  if ! command -v "$tool" >/dev/null; then echo "missing: $tool"; missing=1; fi
done
[[ $missing == 0 ]] || { echo "install the missing tools, then rerun"; exit 1; }
cargo build --release --quiet --manifest-path scanners/rust/Cargo.toml
go build -o out/launchtruth ./cmd/launchtruth
echo "setup ok"
