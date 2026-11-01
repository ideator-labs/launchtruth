#!/usr/bin/env bash
# Re-fetches the scanner parsers from pinned upstream commits and cuts out
# the launch-command parsing functions by line range. Run it to check that the
# vendored files in this repo are byte-for-byte what upstream ships.
set -euo pipefail
cd "$(dirname "$0")"

cut_lines() { # repo sha path from to
  curl -fsSL "https://raw.githubusercontent.com/$1/$2/$3" | sed -n "$4,$5p"
}

{
  echo "// Vendored verbatim from aaif-goose/goose@b23d21cabd9b4de0c045dcdd731821a3a15c2bfc"
  echo "// crates/goose/src/agents/extension_malware_check.rs lines 61-72 and 85-361. Do not edit."
  cut_lines aaif-goose/goose b23d21cabd9b4de0c045dcdd731821a3a15c2bfc crates/goose/src/agents/extension_malware_check.rs 61 72
  echo
  cut_lines aaif-goose/goose b23d21cabd9b4de0c045dcdd731821a3a15c2bfc crates/goose/src/agents/extension_malware_check.rs 85 361
} > rust/src/goose.rs.new

{
  echo "// Vendored verbatim from highflame-ai/ramparts@14ff71ef40bc6ff28ce57ded77466e138a562267"
  echo "// src/osv.rs lines 31-135 and 202-212. Do not edit."
  cut_lines highflame-ai/ramparts 14ff71ef40bc6ff28ce57ded77466e138a562267 src/osv.rs 31 135
  echo
  cut_lines highflame-ai/ramparts 14ff71ef40bc6ff28ce57ded77466e138a562267 src/osv.rs 202 212
} > rust/src/ramparts.rs.new

{
  echo "// Vendored verbatim from pivoshenko/kasetto@7211b9bcfb52ad72912c9f6a503f400feceea17c"
  echo "// src/audit/mcp.rs lines 10, 17-99 and 449-606. Do not edit."
  cut_lines pivoshenko/kasetto 7211b9bcfb52ad72912c9f6a503f400feceea17c src/audit/mcp.rs 10 10
  cut_lines pivoshenko/kasetto 7211b9bcfb52ad72912c9f6a503f400feceea17c src/audit/mcp.rs 17 99
  echo
  cut_lines pivoshenko/kasetto 7211b9bcfb52ad72912c9f6a503f400feceea17c src/audit/mcp.rs 449 606
} > rust/src/kasetto.rs.new

{
  echo "# Vendored verbatim from NousResearch/hermes-agent@c627ec84664d43e0b18b95d9d86d8963747d56c7"
  echo "# tools/osv_check.py lines 195-254. Do not edit."
  echo "import re"
  echo "from typing import Optional"
  echo
  cut_lines NousResearch/hermes-agent c627ec84664d43e0b18b95d9d86d8963747d56c7 tools/osv_check.py 195 254
} > hermes/osv_check_vendored.py.new

status=0
for f in rust/src/goose.rs rust/src/ramparts.rs rust/src/kasetto.rs hermes/osv_check_vendored.py; do
  if [[ "${1:-}" == "--write" ]]; then
    mv "$f.new" "$f"; echo "wrote $f"
  elif cmp -s "$f" "$f.new"; then
    echo "match $f"; rm "$f.new"
  else
    echo "DIFFERS $f"; status=1; rm "$f.new"
  fi
done
exit $status
