// Vendored verbatim from highflame-ai/ramparts@14ff71ef40bc6ff28ce57ded77466e138a562267
// src/osv.rs lines 31-135 and 202-212. Do not edit.
/// A parsed (ecosystem, name, version) triple ready to send to OSV.
#[derive(Debug, Clone)]
pub struct PackageSpec {
    pub ecosystem: &'static str,
    pub name: String,
    pub version: Option<String>,
}

/// Parse a stdio MCP launch command into a `PackageSpec` suitable for an
/// OSV query. Returns `None` when the command isn't a recognized package
/// runner or the args don't yield a clean package reference. We only
/// recognize the conservative subset we can parse without false positives:
///
/// - `npx [-y|--yes] [pkg@ver] [...rest]` → npm:pkg@ver
/// - `uvx [--from pkg[==ver]] [pkg[==ver]] [...rest]` → PyPI:pkg@ver
///
/// Anything more exotic (custom scripts, shell pipelines, npx-with-multiple-
/// packages) returns `None` so we don't emit false positives.
pub fn parse_package_spec_from_command(command: &str, args: &[String]) -> Option<PackageSpec> {
    match command {
        "npx" => parse_npx(args),
        "uvx" => parse_uvx(args),
        _ => None,
    }
}

fn parse_npx(args: &[String]) -> Option<PackageSpec> {
    // First positional arg that doesn't start with `-` is the package.
    let pkg_arg = args.iter().find(|a| !a.starts_with('-'))?;
    let (name, version) = split_npm_spec(pkg_arg)?;
    Some(PackageSpec {
        ecosystem: "npm",
        name,
        version,
    })
}

/// Split an npm package reference into (name, version). Handles scoped
/// packages (`@scope/pkg`) where the leading `@` must NOT be parsed as a
/// version separator.
fn split_npm_spec(raw: &str) -> Option<(String, Option<String>)> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    // Scoped package: @scope/pkg[@ver]
    if let Some(stripped) = raw.strip_prefix('@') {
        let (scope, rest) = stripped.split_once('/')?;
        if let Some((pkg, ver)) = rest.split_once('@') {
            return Some((format!("@{scope}/{pkg}"), version_or_none(ver)));
        }
        return Some((format!("@{scope}/{rest}"), None));
    }
    if let Some((name, ver)) = raw.split_once('@') {
        return Some((name.to_string(), version_or_none(ver)));
    }
    Some((raw.to_string(), None))
}

fn parse_uvx(args: &[String]) -> Option<PackageSpec> {
    // `uvx --from pkg[==ver] cmd ...` or `uvx pkg[==ver] [...]`. Take the
    // first non-flag token after a known flag, or the first non-flag token
    // overall.
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.peek() {
        if *arg == "--from" {
            iter.next();
            if let Some(spec) = iter.next() {
                let (name, version) = split_pypi_spec(spec)?;
                return Some(PackageSpec {
                    ecosystem: "PyPI",
                    name,
                    version,
                });
            }
            return None;
        }
        if arg.starts_with('-') {
            iter.next();
            continue;
        }
        break;
    }
    let pkg = iter.next()?;
    let (name, version) = split_pypi_spec(pkg)?;
    Some(PackageSpec {
        ecosystem: "PyPI",
        name,
        version,
    })
}

fn split_pypi_spec(raw: &str) -> Option<(String, Option<String>)> {
    // PyPI version specifiers we care about: `pkg==1.2.3`, `pkg`. We leave
    // looser specs (`pkg>=1.0`) alone because OSV needs an exact version
    // to give a useful answer.
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Some((name, ver)) = raw.split_once("==") {
        return Some((name.to_string(), version_or_none(ver)));
    }
    Some((raw.to_string(), None))
}

fn version_or_none(s: &str) -> Option<String> {
    let s = s.trim();
    // npm/PyPI both treat "latest" as a tag rather than a real version. OSV
    // can't resolve tags, so drop these and return the package alone — OSV
    // will tell us "any known vulns at all", which is still useful info.
    if s.is_empty() || s.eq_ignore_ascii_case("latest") {
        None
    } else {
        Some(s.to_string())
    }
}
