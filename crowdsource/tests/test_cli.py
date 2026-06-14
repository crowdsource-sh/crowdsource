"""Unit tests for the CLI's pure helpers (no network, no compiled extension)."""

import os
from datetime import datetime, timezone

import pytest

from crowdsource import _config, _fmt, cli

# ---- payload building ----


def test_scalar_value_wraps_in_value_key():
    assert cli.build_submission_body("5418.50") == {"payload": {"value": 5418.5}}


def test_integer_value_coerced():
    assert cli.build_submission_body("42") == {"payload": {"value": 42}}


def test_string_value_kept_as_string():
    assert cli.build_submission_body("Oklahoma City Thunder") == {"payload": {"value": "Oklahoma City Thunder"}}


def test_raw_json_passes_object_through():
    assert cli.build_submission_body('{"a": 1}', raw_json=True) == {"payload": {"a": 1}}


def test_s3_key_body():
    assert cli.build_submission_body(s3_key="preds/run1.csv") == {"s3_key": "preds/run1.csv"}


def test_file_text_used_as_payload():
    assert cli.build_submission_body(file_text='{"value": 7}') == {"payload": {"value": 7}}


def test_stdin_scalar_text_wrapped():
    assert cli.build_submission_body(stdin_text="5418.50\n") == {"payload": {"value": 5418.5}}


def test_stdin_json_object_passthrough():
    assert cli.build_submission_body(stdin_text='{"value": 1}') == {"payload": {"value": 1}}


def test_missing_value_raises():
    with pytest.raises(ValueError):
        cli.build_submission_body()


# ---- exit-code mapping ----


@pytest.mark.parametrize(
    "msg,code",
    [
        ("forbidden (HTTP 403)", cli.EX_AUTH),
        ("unauthorized (HTTP 401)", cli.EX_AUTH),
        ("insufficient credits: need 1100, have 50 (HTTP 402)", cli.EX_CREDITS),
        ("internal error (HTTP 500)", cli.EX_NET),
        ("transport error: connection refused", cli.EX_NET),
        ("bad request (HTTP 400)", cli.EX_USAGE),
    ],
)
def test_error_to_exit_code(msg, code):
    assert cli.error_to_exit_code(Exception(msg)) == code


# ---- formatting ----


def test_credits_symbol_and_grouping():
    assert _fmt.credits(1240) == "◎1,240"


def test_mask_key():
    assert _fmt.mask_key("cs_sk_0123456789abcdef") == "cs_sk_0…cdef"


def test_time_left_buckets():
    now = datetime(2026, 6, 14, 12, 0, 0, tzinfo=timezone.utc)
    assert _fmt.time_left("2026-06-14T15:14:00Z", now) == "3h 14m"
    assert _fmt.time_left("2026-06-28T12:00:00Z", now) == "14d"
    assert _fmt.time_left("2026-06-14T11:00:00Z", now) == "closed"


def test_render_rows_csv():
    cols = [_fmt.Column("id", "ID", _fmt.short_id), _fmt.Column("n", "N")]
    out = _fmt.to_csv_string([{"id": "abcdef123456", "n": 5}], cols)
    assert "ID,N" in out
    assert "abcdef12,5" in out


# ---- config round-trip ----


def test_config_roundtrip(tmp_path, monkeypatch):
    monkeypatch.setenv("CROWDSOURCE_CONFIG_DIR", str(tmp_path))
    _config.save_config({"api_key": "cs_sk_secret", "default_format": "json"})
    cfg = _config.load_config()
    assert cfg["api_key"] == "cs_sk_secret"
    assert cfg["default_format"] == "json"
    # File must be owner-only (it holds a secret). POSIX-only: Windows doesn't
    # honor POSIX permission bits, so chmod(0o600) leaves the mode at 0o666.
    if os.name == "posix":
        mode = (_config.config_path().stat().st_mode) & 0o777
        assert mode == 0o600


def test_resolve_precedence(tmp_path, monkeypatch):
    monkeypatch.setenv("CROWDSOURCE_CONFIG_DIR", str(tmp_path))
    monkeypatch.delenv("CROWDSOURCE_API_KEY", raising=False)
    monkeypatch.delenv("CROWDSOURCE_SERVER_URL", raising=False)
    _config.save_config({"api_key": "from_file", "server": "https://file.example"})
    # flag beats env beats file
    assert _config.resolve_api_key("from_flag") == "from_flag"
    monkeypatch.setenv("CROWDSOURCE_API_KEY", "from_env")
    assert _config.resolve_api_key() == "from_env"
    monkeypatch.delenv("CROWDSOURCE_API_KEY")
    assert _config.resolve_api_key() == "from_file"
    assert _config.resolve_server() == "https://file.example"
    assert _config.resolve_server("https://flag.example") == "https://flag.example"


def test_resolve_default_server(tmp_path, monkeypatch):
    monkeypatch.setenv("CROWDSOURCE_CONFIG_DIR", str(tmp_path))
    monkeypatch.delenv("CROWDSOURCE_SERVER_URL", raising=False)
    assert _config.resolve_server() == _config.DEFAULT_SERVER


# ---- parser wiring ----


def test_parser_builds_and_dispatches():
    parser = cli.build_parser()
    args = parser.parse_args(["competitions", "--status", "open", "--format", "json"])
    assert args.func is cli.cmd_competitions
    assert args.status == "open"
    assert getattr(args, "format") == "json"


def test_parser_global_flag_before_subcommand():
    parser = cli.build_parser()
    args = parser.parse_args(["--format", "csv", "balance"])
    assert args.func is cli.cmd_balance
    assert getattr(args, "format") == "csv"


def test_apikeys_defaults_to_list():
    parser = cli.build_parser()
    args = parser.parse_args(["api-keys"])
    assert args.func is cli.cmd_apikeys_list
