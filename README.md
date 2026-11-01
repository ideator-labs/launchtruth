# LaunchTruth

Which package does this MCP server launch command really install?

MCP hosts and scanners read `npx ...` and `uvx ...` lines from server configs and check the package against OSV for malware. Each one parses the command line its own way. LaunchTruth runs a shared set of launch commands through the real launchers against a loopback registry, records which package names they ask for, and compares that with what four real scanner parsers would check.

For agents and contributors, start with [AGENTS.md](AGENTS.md).

## Result on 2026-11-01

17 launch commands (4 controls, 13 probes). Launchers: npx and npm exec from npm 11.21.1, pnpm 9.15.9, uvx and uv tool run from uv 0.13.5.

| scanner | source | right package on probes |
|---|---|---|
| goose | `aaif-goose/goose@b23d21c` | 5 of 13 |
| ramparts | `highflame-ai/ramparts@14ff71e` | 3 of 13 |
| kasetto | `pivoshenko/kasetto@7211b9b` | 10 of 13 |
| hermes-agent | `NousResearch/hermes-agent@c627ec8` | 5 of 13 |

What it found:

- [goose#13265](https://github.com/aaif-goose/goose/issues/13265) reproduces. For `npx --package=mcp-evil -c mcp-server-good`, npx fetched `mcp-evil` and goose would check `mcp-server-good`. ramparts does the same.
- hermes-agent says its check was inspired by goose's. It handles `--package=` but not `uvx --from=`, which goose handles. The copies drifted in opposite directions.
- No scanner fully covers `npx -p a -p b`, `uvx --with`, or a value-taking flag before the package (`npx --loglevel warn pkg`, where all four would check a package named `warn`).
- goose lists `--python` as a flag that takes a value, but only uses that list while looking for `--from`. For `uvx --python 3.13 mcp-server-good` it checks `3-13`.
- `npm exec`, `pnpm dlx` and `uv tool run` fetch packages that goose, ramparts and hermes-agent never check (they fail open). kasetto checks all three.

These are parser gaps in a fail-open safety net, not bypasses of user approval. goose#13265 says the same about its case.

## Run

```
bash setup.sh
bash demo.sh           # 8/8 checks pass, exit 0
bash demo.sh --break   # judge scanners only on "parsed something"; checks 3-6 and 8 fail, exit 1
```

Needs Go, Rust (cargo), Python 3, Node with npm, pnpm, uv and jq. No network beyond `cargo` fetching serde on first build. No API keys.

## Limits

The registry stub answers 404 to every request, so the truth is the set of top-level names the launcher asked for, not a full install. Transitive dependencies are out of scope. Scanner code is vendored verbatim by line range (see `scanners/vendor.sh`) and called through a thin adapter; it is not the full host running. bunx is not covered (it did not use the stub registry in a quick test). Package names like `mcp-evil` are placeholders; nothing is downloaded or run.

## License

Apache-2.0 for this repo's own code. Vendored scanner code keeps its upstream license: goose and ramparts Apache-2.0, hermes-agent MIT, kasetto MIT or Apache-2.0.
