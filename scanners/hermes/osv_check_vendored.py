# Vendored verbatim from NousResearch/hermes-agent@c627ec84664d43e0b18b95d9d86d8963747d56c7
# tools/osv_check.py lines 195-254. Do not edit.
import re
from typing import Optional

_ECOSYSTEM_BY_COMMAND = {
    "npx": "npm", "npx.cmd": "npm",
    "uvx": "PyPI", "uvx.cmd": "PyPI", "uvx.exe": "PyPI",
    "pipx": "PyPI", "pipx.exe": "PyPI",
}


def _infer_ecosystem(command: str) -> Optional[str]:
    # Split on BOTH separators: os.path.basename leaves ``C:\...\uvx.exe`` intact on POSIX
    # (config authored for Windows) and the preflight would silently skip. Only the shim
    # names each runner actually installs are listed; lookalikes stay fail-open.
    return _ECOSYSTEM_BY_COMMAND.get(re.split(r"[\\/]", command)[-1].lower())


def _parse_package_from_args(args: list, ecosystem: str) -> tuple[Optional[str], Optional[str]]:
    """Extract (package_name, version) from command args, or (None, None) if not parseable."""
    # Skip flags to find the package token. npx's explicit install target (--package=NAME /
    # --package NAME / -p NAME) names a package distinct from the executed binary.
    package_token = None
    take_next = False
    for arg in args or ():
        if not isinstance(arg, str):
            continue
        if take_next:
            package_token = arg
            break
        if arg in ("--package", "-p"):
            take_next = True
            continue
        if arg.startswith("--package="):
            package_token = arg[len("--package="):]
            break
        if arg.startswith("-"):
            continue
        package_token = arg
        break
    if not package_token:
        return None, None
    parser = _PACKAGE_PARSERS.get(ecosystem)
    return parser(package_token) if parser else (package_token, None)


def _parse_npm_package(token: str) -> tuple[Optional[str], Optional[str]]:
    """Parse npm package: @scope/name@version or name@version."""
    if token.startswith("@"):
        match = re.match(r"^(@[^/]+/[^@]+)(?:@(.+))?$", token)
        return (match.group(1), match.group(2)) if match else (token, None)
    if "@" in token:
        name, version = token.rsplit("@", 1)
        return name, version if version != "latest" else None
    return token, None


def _parse_pypi_package(token: str) -> tuple[Optional[str], Optional[str]]:
    """Parse PyPI package: name==version or name[extras]==version."""
    match = re.match(r"^([a-zA-Z0-9._-]+)(?:\[[^\]]*\])?(?:==(.+))?$", token)
    return (match.group(1), match.group(2)) if match else (token, None)


_PACKAGE_PARSERS = {"npm": _parse_npm_package, "PyPI": _parse_pypi_package}
