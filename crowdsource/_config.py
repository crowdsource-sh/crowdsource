"""Config + credential resolution for the crowdsource CLI.

Credentials live in ``~/.crowdsource/config.toml`` (written ``chmod 600`` since it
holds an API key). The CLI resolves the server URL and API key with this
precedence: explicit flag → environment variable → config file → built-in default.

No third-party deps: the config is a small flat TOML table, parsed with stdlib
``tomllib`` when available (3.11+) and a minimal fallback otherwise, and written
by hand (all values are strings).
"""

from __future__ import annotations

import os
from pathlib import Path

# Matches the Rust core's ``DEFAULT_BASE_URL`` (crowdsource::client).
DEFAULT_SERVER = "https://api.crowdsource.sh"

# Keys the CLI recognizes in config.toml / ``config set``.
KNOWN_KEYS = ("api_key", "server", "default_format")


def config_dir() -> Path:
    return Path(os.environ.get("CROWDSOURCE_CONFIG_DIR", Path.home() / ".crowdsource"))


def config_path() -> Path:
    return config_dir() / "config.toml"


def _parse_toml(text: str) -> dict:
    """Parse our flat ``key = "value"`` config. Prefers stdlib tomllib."""
    try:
        import tomllib

        return tomllib.loads(text)
    except ModuleNotFoundError:
        pass
    except Exception:
        return {}
    cfg: dict = {}
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line or "=" not in line:
            continue
        key, _, value = line.partition("=")
        cfg[key.strip()] = value.strip().strip('"').strip("'")
    return cfg


def load_config() -> dict:
    path = config_path()
    if not path.exists():
        return {}
    try:
        return _parse_toml(path.read_text())
    except OSError:
        return {}


def save_config(cfg: dict) -> None:
    """Write the config table (sorted, only known keys) with owner-only perms."""
    path = config_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = ["# crowdsource CLI config — created automatically.", ""]
    for key in sorted(cfg):
        value = str(cfg[key]).replace("\\", "\\\\").replace('"', '\\"')
        lines.append(f'{key} = "{value}"')
    path.write_text("\n".join(lines) + "\n")
    try:
        path.chmod(0o600)
    except OSError:
        pass


def resolve_server(flag: str | None = None, cfg: dict | None = None) -> str:
    cfg = load_config() if cfg is None else cfg
    return flag or os.environ.get("CROWDSOURCE_SERVER_URL") or cfg.get("server") or DEFAULT_SERVER


def resolve_api_key(flag: str | None = None, cfg: dict | None = None) -> str | None:
    cfg = load_config() if cfg is None else cfg
    return flag or os.environ.get("CROWDSOURCE_API_KEY") or cfg.get("api_key")


def resolve_format(flag: str | None = None, cfg: dict | None = None) -> str:
    cfg = load_config() if cfg is None else cfg
    return flag or cfg.get("default_format") or "table"


class AuthError(RuntimeError):
    """No API key available for a command that requires authentication."""


def _load_client_class():
    """Import the compiled pyo3 ``Client`` lazily (so CLI helpers stay importable)."""
    from crowdsource.crowdsource import Client  # type: ignore

    return Client


def make_client(api_key_flag: str | None, server_flag: str | None, *, require_key: bool = True):
    """Build a ``Client`` from resolved server + API key."""
    cfg = load_config()
    server = resolve_server(server_flag, cfg)
    api_key = resolve_api_key(api_key_flag, cfg)
    if require_key and not api_key:
        raise AuthError("no API key found. Run `crowdsource login`, pass --api-key, or set CROWDSOURCE_API_KEY.")
    Client = _load_client_class()
    return Client(server, api_key)
