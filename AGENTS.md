# AGENTS.md

Handoff for a coding agent with no prior context. Read this before changing anything.

## What this proves

MCP hosts and scanners that screen stdio servers for malware read the launch command (`npx ...`, `uvx ...`) and pick one package to send to OSV. LaunchTruth checks that pick against what the real launcher actually fetches. The claim: the same launch command yields different "checked" packages in different scanners, and several of them check a package that is never installed while the real one goes unchecked. A shared corpus with measured truth catches this for every scanner at once.

## Status

Real:
- Launchers. Real `npx` and `npm exec` (npm 11.21.1), `pnpm dlx` (pnpm 9.15.9), `uvx` and `uv tool run` (uv 0.13.5) run every case.
- Registry requests. A loopback HTTP server records every request path. That record is the truth.
- Scanner parsers. Vendored verbatim from pinned commits by `scanners/vendor.sh`:
  - goose `crates/goose/src/agents/extension_malware_check.rs` @ `b23d21c`
  - ramparts `src/osv.rs` @ `14ff71e`
  - kasetto `src/audit/mcp.rs` @ `7211b9b`
  - hermes-agent `tools/osv_check.py` @ `c627ec8`

Stubbed:
- The registry answers 404 to everything, so launchers stop after asking. Nothing is installed or run.
- No OSV query is made. Only the package each scanner would query is compared.
- Package names (`mcp-evil`, `mcp-server-good`, `@acme/mcp-server`) are placeholders.

## Data shape

- `corpus/cases.json`: array of `{id, kind: "control"|"probe", command, args[], installs: [{ecosystem: "npm"|"PyPI", name}], note, source?}`. `installs` is the hand label. Check 1 proves every label equals the measured truth.
- `out/truth.json`: array of `{id, requested: [Package], other_requests?: [path], launcher_exit}`.
- `out/scans.jsonl`: one line per (case, scanner): `{id, scanner, package: Package|null}`.
- `out/report.json`: `{lenient, scanners[], cases: [{id, kind, command, requested, label_matches, other_requests, checked: {scanner: Package|null}, verdicts: {scanner: Verdict}}]}`.
- Verdict: `OK` (checked the one package fetched), `PARTIAL` (checked one of several fetched), `WRONG` (checked a package that was not fetched), `SKIP` (checked nothing). PyPI names compare after PEP 503 normalization.

## Repo map

- `cmd/launchtruth/model.go`: types, normalization, `judge`.
- `cmd/launchtruth/truth.go`: recording registry, launcher environment, `measureTruth`.
- `cmd/launchtruth/main.go`: `truth` and `report` subcommands, table printer.
- `cmd/launchtruth/model_test.go`: path mapping and verdict tests.
- `scanners/vendor.sh`: fetches the pinned upstream files and cuts the parser functions. No args compares; `--write` overwrites.
- `scanners/rust/`: one binary that `include!`s the goose, ramparts and kasetto excerpts and calls each the way its host does.
- `scanners/hermes/`: `osv_check_vendored.py` (excerpt) and `scan.py` (adapter).
- `setup.sh`: checks tools, builds both binaries.
- `demo.sh`: the real artifact. Measures truth, runs scanners, writes the report, runs 8 checks.

## Fresh clone: setup, run, verify

```
git clone https://github.com/ideator-labs/launchtruth && cd launchtruth
bash setup.sh                 # prints "setup ok"
go test ./...                 # ok
bash scanners/vendor.sh       # four "match" lines (needs network)
bash demo.sh                  # "8/8 checks passed", exit 0
bash demo.sh --break          # "3/8 checks passed", exit 1; checks 3, 4, 5, 6, 8 fail
```

Expected per-scanner line in the normal run: goose 5 of 13, ramparts 3 of 13, kasetto 10 of 13, hermes 5 of 13. A whole run takes about 5 seconds after the first build.

## Design decisions

- Truth comes from the launcher, not from reading npm or uv docs. The docs are easy to misread, and the claim is about what really runs.
- A 404 registry instead of a fake package server. Every launcher tested asks for all top-level packages before failing (checked for `-p a -p b` and `--with`), so 404 is enough and nothing executes.
- Vendor exact upstream lines instead of reimplementing. A reimplementation would test my reading of their code. The adapters only add the call each host makes (goose `command_ecosystem` then `parse_first_package_arg`; ramparts `parse_package_spec_from_command`; kasetto `launched_package`; hermes `_infer_ecosystem` then `_parse_package_from_args`).
- `--break` uses lenient judging ("parsed any package"), which is what the scanners' own unit tests assert. It shows the checks depend on measured truth.

## Limits

- Top-level packages only. Transitive dependencies and lockfiles are out of scope.
- Linux only, tested with the versions above. Launcher behavior can change between versions; the corpus records which versions were measured.
- bunx is not covered. It did not hit the stub registry with `BUN_CONFIG_REGISTRY` in a quick test.
- Each scanner is tested as a parser, not as a running host. A host might reject a config before the check runs.
- Findings are about a fail-open safety net. None of them bypass user approval of an extension.

## Next steps

1. Add bunx, pipx run and `docker run` cases. Done when each has a measured truth row and check 1 still passes.
2. Add MCP Registry `server.json` input: derive the command from `packages[]` (`registryType`, `identifier`, `runtimeArguments`) and flag entries whose `runtimeArguments` make the launcher fetch a different package than `identifier`. Done when a fixture with `runtimeArguments: ["--package=mcp-evil"]` is reported as a mismatch.
3. Publish a canonical expectation file (`corpus/expected.json`, command to package list) that scanners can import as test vectors without running launchers. Done when a goose unit test generated from it fails on `b23d21c` for c05.
4. Add Hivemind OS's MCP launcher (`crates/hive-mcp/src/runtime.rs`) if it gains a package check. Done when it appears as a fifth column.
5. A GitHub Action that runs `demo.sh` against the latest npm and uv weekly. Done when a version bump that changes truth fails check 1.

## Working rules

- Never hand-edit vendored files. Change the pinned commit and line ranges in `vendor.sh`, run `vendor.sh --write`, then `vendor.sh` must print four matches.
- Every new case needs a measured truth. If check 1 fails, fix the label or find out why the launcher disagrees; do not loosen the check.
- Keep the demo local. No real registry, no OSV calls, no API keys.
- Commit as `danielgerlag <daniel@gerlag.ca>`.
- Plain prose in docs: short sentences, no em-dashes.

## Sources

- goose#13265, npm `--package=` not parsed: https://github.com/aaif-goose/goose/issues/13265
- goose malware check: https://github.com/aaif-goose/goose/blob/b23d21cabd9b4de0c045dcdd731821a3a15c2bfc/crates/goose/src/agents/extension_malware_check.rs
- hermes-agent OSV check: https://github.com/NousResearch/hermes-agent/blob/c627ec84664d43e0b18b95d9d86d8963747d56c7/tools/osv_check.py
- ramparts OSV module: https://github.com/highflame-ai/ramparts/blob/14ff71ef40bc6ff28ce57ded77466e138a562267/src/osv.rs
- kasetto MCP audit: https://github.com/pivoshenko/kasetto/blob/7211b9bcfb52ad72912c9f6a503f400feceea17c/src/audit/mcp.rs
- npm exec docs: https://docs.npmjs.com/cli/commands/npm-exec
- uv tool run docs: https://docs.astral.sh/uv/reference/cli/#uv-tool-run
