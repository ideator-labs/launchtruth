// Vendored verbatim from pivoshenko/kasetto@7211b9bcfb52ad72912c9f6a503f400feceea17c
// src/audit/mcp.rs lines 10, 17-99 and 449-606. Do not edit.
use std::path::Path;
/// The launch-relevant fields of a server definition, normalized across the
/// agent-native shapes kasetto writes (`mcpServers`, VS Code, OpenCode's
/// command array and `environment`, Codex TOML, Gemini's `httpUrl`).
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct ServerSpec {
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env: Vec<(String, String)>,
}

impl ServerSpec {
    pub(super) fn from_definition(def: &serde_json::Value) -> ServerSpec {
        let text = |v: &serde_json::Value| match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let mut spec = ServerSpec::default();
        match def.get("command") {
            Some(serde_json::Value::String(c)) => spec.command = Some(c.clone()),
            Some(serde_json::Value::Array(parts)) => {
                let mut parts = parts.iter().map(text);
                spec.command = parts.next();
                spec.args.extend(parts);
            }
            _ => {}
        }
        if let Some(args) = def.get("args").and_then(|a| a.as_array()) {
            spec.args.extend(args.iter().map(text));
        }
        spec.url = ["url", "serverUrl", "httpUrl"]
            .iter()
            .find_map(|k| def.get(*k).and_then(|u| u.as_str()))
            .map(str::to_string);
        for key in ["env", "environment"] {
            if let Some(env) = def.get(key).and_then(|e| e.as_object()) {
                spec.env
                    .extend(env.iter().map(|(k, v)| (k.clone(), text(v))));
            }
        }
        spec
    }

    /// The command's file name without a Windows `.exe` / `.cmd` suffix, so
    /// `/usr/local/bin/npx` and `npx.cmd` both read as `npx`.
    fn program(&self) -> Option<String> {
        let cmd = self.command.as_deref()?;
        let base = Path::new(cmd).file_name()?.to_string_lossy().to_lowercase();
        Some(
            base.strip_suffix(".exe")
                .or_else(|| base.strip_suffix(".cmd"))
                .unwrap_or(&base)
                .to_string(),
        )
    }
}

/// A package registry the launched server is pulled from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ecosystem {
    Npm,
    PyPI,
}

impl Ecosystem {
    /// The ecosystem name as OSV spells it.
    pub(super) fn osv_name(self) -> &'static str {
        match self {
            Ecosystem::Npm => "npm",
            Ecosystem::PyPI => "PyPI",
        }
    }
}

/// The package a server launches through a package runner (`npx`, `uvx`, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Package {
    pub ecosystem: Ecosystem,
    pub name: String,
    /// The exact version the definition pins, `None` when it floats (no
    /// version, a dist-tag like `latest`, or a range).
    pub version: Option<String>,
}

/// The registry package a server is launched from, when its command is a
/// package runner. Local paths, git URLs, and tarballs are not packages.
pub(super) fn launched_package(spec: &ServerSpec) -> Option<Package> {
    let program = spec.program()?;
    let args: Vec<&str> = spec.args.iter().map(String::as_str).collect();
    let (ecosystem, rest) = match (program.as_str(), args.as_slice()) {
        ("npx" | "bunx", rest) => (Ecosystem::Npm, rest),
        ("pnpm" | "yarn" | "bun", ["dlx" | "x", rest @ ..]) => (Ecosystem::Npm, rest),
        ("npm", ["exec" | "x", rest @ ..]) => (Ecosystem::Npm, rest),
        ("uvx", rest) => (Ecosystem::PyPI, rest),
        ("uv", ["tool", "run", rest @ ..]) => (Ecosystem::PyPI, rest),
        ("pipx", ["run", rest @ ..]) => (Ecosystem::PyPI, rest),
        _ => return None,
    };
    let spec_arg = package_arg(ecosystem, rest)?;
    match ecosystem {
        Ecosystem::Npm => parse_npm_spec(spec_arg),
        Ecosystem::PyPI => parse_pypi_spec(spec_arg),
    }
}

/// The package spec among a runner's arguments: an explicit `--package` /
/// `--from` / `--spec` value wins, else the first positional argument.
fn package_arg<'a>(ecosystem: Ecosystem, args: &[&'a str]) -> Option<&'a str> {
    let (explicit, value_flags): (&[&str], &[&str]) = match ecosystem {
        Ecosystem::Npm => (
            &["-p", "--package"],
            &["-c", "--call", "--registry", "--cache"],
        ),
        Ecosystem::PyPI => (
            &["--from", "--spec"],
            &[
                "--with",
                "--python",
                "-p",
                "--index",
                "--index-url",
                "--extra-index-url",
                "--pip-args",
            ],
        ),
    };
    let mut i = 0;
    while i < args.len() {
        let arg = args[i];
        if let Some((flag, value)) = arg.split_once('=') {
            if explicit.contains(&flag) {
                return Some(value);
            }
        }
        if explicit.contains(&arg) {
            return args.get(i + 1).copied();
        }
        if arg == "--" {
            return args.get(i + 1).copied();
        }
        if !arg.starts_with('-') {
            return Some(arg);
        }
        if value_flags.contains(&arg) {
            i += 1;
        }
        i += 1;
    }
    None
}

fn parse_npm_spec(spec: &str) -> Option<Package> {
    let spec = spec.strip_prefix("npm:").unwrap_or(spec);
    if !spec.is_ascii()
        || spec.is_empty()
        || spec.starts_with(['.', '/', '~'])
        || spec.contains("://")
    {
        return None;
    }
    if spec.contains(':') || spec.ends_with(".tgz") {
        return None;
    }
    // A scoped name carries its own leading `@`; the version separator is the
    // last `@` after it.
    let (name, version) = match spec[1..].rfind('@') {
        Some(at) => (&spec[..at + 1], Some(&spec[at + 2..])),
        None => (spec, None),
    };
    let mut parts = name.strip_prefix('@').unwrap_or(name).splitn(2, '/');
    let valid = match (name.starts_with('@'), parts.next(), parts.next()) {
        (true, Some(scope), Some(base)) => npm_name_part(scope) && npm_name_part(base),
        (false, Some(base), None) => npm_name_part(base),
        _ => false,
    };
    if !valid || name.len() > 214 {
        return None;
    }
    Some(Package {
        ecosystem: Ecosystem::Npm,
        name: name.to_string(),
        version: version.filter(|v| is_exact_version(v)).map(str::to_string),
    })
}

fn parse_pypi_spec(spec: &str) -> Option<Package> {
    if spec.is_empty() || spec.starts_with(['.', '/', '~']) || spec.contains("://") {
        return None;
    }
    let (name, version) = if let Some((n, v)) = spec.split_once("==") {
        (n, Some(v))
    } else if let Some((n, v)) = spec.split_once('@') {
        (n, Some(v))
    } else {
        let end = spec.find(['<', '>', '~', '!', '=']).unwrap_or(spec.len());
        (&spec[..end], None)
    };
    let name = name.split('[').next().unwrap_or(name).trim();
    let valid = name.starts_with(|c: char| c.is_ascii_alphanumeric())
        && name.ends_with(|c: char| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !valid || looks_like_a_secret(name) {
        return None;
    }
    Some(Package {
        ecosystem: Ecosystem::PyPI,
        name: name.to_string(),
        version: version
            .map(str::trim)
            .filter(|v| is_exact_version(v))
            .map(str::to_string),
    })
}

/// One side of an npm name (`scope` or `name`): lowercase, URL-safe, and not
/// a resolved secret that happens to sit where a package name goes.
fn npm_name_part(part: &str) -> bool {
    !part.is_empty()
        && !part.starts_with(['.', '_'])
        && part.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.' | '_' | '~')
        })
        && !looks_like_a_secret(part)
}

/// The definition is read with secrets resolved, so a `${kst_...}` placeholder
/// in the package position arrives as its value. Registry names are words
/// joined by separators; a long run with none is a token, never sent to OSV
/// or echoed in a verdict.
fn looks_like_a_secret(name: &str) -> bool {
    name.split(['-', '.', '_']).any(|word| word.len() >= 24)
}

/// A concrete release (`1.2.3`, `0.4.0-beta.1`), not a dist-tag or a range.
fn is_exact_version(v: &str) -> bool {
    v.starts_with(|c: char| c.is_ascii_digit())
        && !looks_like_a_secret(v)
        && !v.contains(['^', '~', '>', '<', '*', '|', ' ', ','])
        && !v.split('.').any(|p| p.eq_ignore_ascii_case("x"))
}
