// Vendored verbatim from aaif-goose/goose@b23d21cabd9b4de0c045dcdd731821a3a15c2bfc
// crates/goose/src/agents/extension_malware_check.rs lines 61-72 and 85-361. Do not edit.
fn command_ecosystem(cmd: &str) -> Option<&'static str> {
    let executable = cmd
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match executable.as_str() {
        "npx" | "npx.cmd" => Some("npm"),
        "uvx" | "uvx.exe" => Some("PyPI"),
        _ => None,
    }
}

fn parse_first_package_arg(ecosystem: &str, args: &[String]) -> Option<(String, Option<String>)> {
    let is_flag = |s: &str| s.starts_with('-');
    let positional = || {
        args.iter()
            .find(|a| !is_flag(a.as_str()))
            .map(String::as_str)
    };
    let token = if ecosystem == "PyPI" {
        uvx_from_package(args).or_else(positional)?
    } else {
        positional()?
    }
    .trim()
    .to_string();
    if token.is_empty() {
        return None;
    }
    match ecosystem {
        "npm" => parse_npm_token(&token),
        "PyPI" => parse_pypi_token(&token),
        _ => None,
    }
}

fn uvx_from_package(args: &[String]) -> Option<&str> {
    let mut index = 0;
    while let Some(argument) = args.get(index).map(String::as_str) {
        if argument == "--" || !argument.starts_with('-') {
            break;
        }
        if let Some(package) = argument
            .strip_prefix("--from=")
            .filter(|package| !package.is_empty())
        {
            return Some(package);
        }
        if argument == "--from" {
            return args.get(index + 1).map(String::as_str);
        }

        index += if uvx_option_takes_separate_value(argument)
            && args
                .get(index + 1)
                .is_some_and(|value| !value.starts_with('-'))
        {
            2
        } else {
            1
        };
    }
    None
}

fn uvx_option_takes_separate_value(argument: &str) -> bool {
    if let Some(cluster) = argument
        .strip_prefix('-')
        .filter(|cluster| !cluster.starts_with('-'))
    {
        for (index, option) in cluster.char_indices() {
            if matches!(option, 'C' | 'P' | 'b' | 'c' | 'f' | 'i' | 'p' | 'w') {
                return index + option.len_utf8() == cluster.len();
            }
        }
        return false;
    }
    if argument.contains('=') {
        return false;
    }
    if argument.strip_prefix("--extra-") == Some("index-url") {
        return true;
    }

    matches!(
        argument,
        "--allow-insecure-host"
            | "--build-constraint"
            | "--build-constraints"
            | "--cache-dir"
            | "--color"
            | "--config-file"
            | "--config-setting"
            | "--config-settings"
            | "--config-settings-package"
            | "--constraint"
            | "--constraints"
            | "--default-index"
            | "--directory"
            | "--env-file"
            | "--exclude-newer"
            | "--exclude-newer-package"
            | "--find-links"
            | "--fork-strategy"
            | "--generate-shell-completion"
            | "--index"
            | "--index-strategy"
            | "--index-url"
            | "--keyring-provider"
            | "--link-mode"
            | "--no-binary-package"
            | "--no-build-isolation-package"
            | "--no-build-package"
            | "--no-sources-package"
            | "--override"
            | "--overrides"
            | "--prerelease"
            | "--prerelease-package"
            | "--preview-feature"
            | "--preview-features"
            | "--project"
            | "--python"
            | "--python-fetch"
            | "--python-platform"
            | "--python-preference"
            | "--refresh-package"
            | "--reinstall-package"
            | "--resolution"
            | "--torch-backend"
            | "--trusted-host"
            | "--upgrade-group"
            | "--upgrade-package"
            | "--with"
            | "--with-editable"
            | "--with-requirements"
    )
}

fn parse_npm_token(token: &str) -> Option<(String, Option<String>)> {
    // Handles:
    //   react@18.3.1
    //   @scope/pkg@1.2.3   (split at the LAST '@')
    //   eslint              (no version)
    if token.starts_with('@') {
        if let Some(idx) = token.rfind('@') {
            if idx > 0 {
                let (name, ver) = token.split_at(idx);
                let ver = ver.trim_start_matches('@');
                if !ver.is_empty() && ver != "latest" {
                    return Some((name.to_string(), Some(ver.to_string())));
                } else {
                    return Some((name.to_string(), None));
                }
            }
        }
        Some((token.to_string(), None))
    } else if let Some(idx) = token.find('@') {
        let (name, ver) = token.split_at(idx);
        let ver = ver.trim_start_matches('@');
        if !name.is_empty() {
            if !ver.is_empty() && ver != "latest" {
                return Some((name.to_string(), Some(ver.to_string())));
            } else {
                return Some((name.to_string(), None));
            }
        }
        None
    } else {
        Some((token.to_string(), None))
    }
}

fn parse_pypi_token(token: &str) -> Option<(String, Option<String>)> {
    let token = token.trim();
    let name_end = token
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_alphanumeric() && !matches!(character, '-' | '_' | '.'))
                .then_some(index)
        })
        .unwrap_or(token.len());
    let (name, requirement) = token.split_at(name_end);
    let name = name.trim();
    if name.is_empty()
        || !name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        || !name
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric())
    {
        return None;
    }

    let mut raw_requirement = requirement;
    let mut requirement = raw_requirement.trim_start();
    if !requirement.is_empty()
        && !requirement
            .chars()
            .next()
            .is_some_and(|character| matches!(character, '[' | '@' | ';' | '('))
        && !["===", "~=", "==", "!=", "<=", ">=", "<", ">"]
            .iter()
            .any(|operator| requirement.starts_with(operator))
    {
        return None;
    }
    if let Some(extras) = requirement.strip_prefix('[') {
        let (_, remaining) = extras.split_once(']')?;
        raw_requirement = remaining;
        requirement = remaining.trim_start();
    }

    let version = parse_pypi_exact_version(requirement).or_else(|| {
        raw_requirement
            .strip_prefix('@')
            .and_then(parse_concrete_version)
    });

    Some((normalize_pypi_name(name), version))
}

fn parse_pypi_exact_version(requirement: &str) -> Option<String> {
    let requirement = requirement.trim();
    let comparison = if let Some(parenthesized) = requirement
        .strip_prefix('(')
        .and_then(|requirement| requirement.strip_suffix(')'))
    {
        let comparison = parenthesized.trim();
        if comparison.contains(['(', ')']) {
            return None;
        }
        comparison
    } else {
        if requirement.contains(['(', ')']) {
            return None;
        }
        requirement
    };

    let version = comparison.strip_prefix("==")?.trim();
    if version.starts_with('=') {
        return None;
    }
    parse_concrete_version(version)
}

fn parse_concrete_version(version: &str) -> Option<String> {
    let version = version.trim();
    let numeric_version = version
        .strip_prefix('v')
        .or_else(|| version.strip_prefix('V'))
        .unwrap_or(version);
    if version.is_empty()
        || version.eq_ignore_ascii_case("latest")
        || !numeric_version
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
        || !version.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '+' | '!')
        })
    {
        return None;
    }

    Some(version.to_string())
}

fn normalize_pypi_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut pending_separator = false;

    for character in name.chars() {
        if matches!(character, '-' | '_' | '.') {
            pending_separator = true;
        } else {
            if pending_separator && !normalized.is_empty() {
                normalized.push('-');
            }
            normalized.push(character.to_ascii_lowercase());
            pending_separator = false;
        }
    }

    normalized
}
