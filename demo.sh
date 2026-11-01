#!/usr/bin/env bash
# Runs every launch command through the real launcher against a loopback registry,
# asks four real scanner parsers which package they would check, and judges each answer.
# --break judges scanners only on "parsed some package", the way their unit tests do.
set -uo pipefail
cd "$(dirname "$0")"
lenient=""
[[ "${1:-}" == "--break" ]] && lenient="-lenient" && echo "BREAK MODE: any parsed package counts as OK"

mkdir -p out
go build -o out/launchtruth ./cmd/launchtruth || exit 1
cargo build --release --quiet --manifest-path scanners/rust/Cargo.toml || exit 1

./out/launchtruth truth -cases corpus/cases.json -out out/truth.json || exit 1
jq '[.[] | {id, command, args}]' corpus/cases.json > out/inputs.json
{
  ./scanners/rust/target/release/launch-scanners < out/inputs.json
  (cd scanners/hermes && python3 scan.py) < out/inputs.json
} > out/scans.jsonl || exit 1
echo
./out/launchtruth report -cases corpus/cases.json -truth out/truth.json -scans out/scans.jsonl -out out/report.json $lenient || exit 1
echo

R=out/report.json
pass=0; fail=0; n=0
check() {
  n=$((n + 1))
  if [[ "$(jq -r "$2" "$R")" == "true" ]]; then
    echo "PASS $n. $1"; pass=$((pass + 1))
  else
    echo "FAIL $n. $1"; fail=$((fail + 1))
  fi
}
v() { echo ".cases[] | select(.id == \"$1\") | .verdicts.$2"; }

check "real npx, npm exec, pnpm dlx, uvx and uv tool run fetched exactly the labelled packages in all 17 cases, with no other registry requests" \
  '(.cases | length == 17) and all(.cases[]; .label_matches and (.other_requests | length == 0))'
check "controls: all four scanners check the right package for the four plain npx/uvx commands" \
  '[.cases[] | select(.kind == "control") | .verdicts[]] | length == 16 and all(. == "OK")'
check "goose#13265 reproduces: for npx --package=mcp-evil -c mcp-server-good, goose checks mcp-server-good while npx fetched mcp-evil" \
  "($(v c05-npx-package-equals goose)) == \"WRONG\" and (.cases[] | select(.id == \"c05-npx-package-equals\") | .checked.goose.name) == \"mcp-server-good\""
check "copies diverge: hermes (whose check says it was inspired by goose's) gets c05 right where goose is wrong, and gets uvx --from= wrong where goose is right" \
  "($(v c05-npx-package-equals hermes)) == \"OK\" and ($(v c12-uvx-from-equals hermes)) == \"WRONG\" and ($(v c12-uvx-from-equals goose)) == \"OK\""
check "shared gaps: no scanner fully covers npx with two -p packages, a value-taking npm flag, or uvx --with" \
  '[.cases[] | select(.id == "c08-npx-two-packages" or .id == "c09-npx-value-flag" or .id == "c14-uvx-with") | .verdicts[]] | length == 12 and all(. != "OK")'
check "goose knows --python takes a value but still checks \"3-13\" for uvx --python 3.13 mcp-server-good" \
  "($(v c15-uvx-python-flag goose)) == \"WRONG\" and (.cases[] | select(.id == \"c15-uvx-python-flag\") | .checked.goose.name) == \"3-13\""
check "runner aliases: npm exec, pnpm dlx and uv tool run fetch packages that goose, ramparts and hermes never check (fail open); only kasetto checks them" \
  '[.cases[] | select(.id == "c10-npm-exec" or .id == "c11-pnpm-dlx" or .id == "c17-uv-tool-run")] | all(.verdicts.kasetto == "OK" and .verdicts.goose == "SKIP" and .verdicts.ramparts == "SKIP" and .verdicts.hermes == "SKIP")'
check "no scanner checks the right package on every probe case" \
  '. as $r | [$r.scanners[] as $s | [$r.cases[] | select(.kind == "probe") | .verdicts[$s]] | all(. == "OK")] | any | not'

echo
for s in $(jq -r '.scanners[]' "$R"); do
  jq -r --arg s "$s" '"\($s): right package on \([.cases[] | select(.kind == "probe" and .verdicts[$s] == "OK")] | length) of \([.cases[] | select(.kind == "probe")] | length) probe cases"' "$R"
done
echo
echo "$pass/$n checks passed"
[[ $fail == 0 ]]
