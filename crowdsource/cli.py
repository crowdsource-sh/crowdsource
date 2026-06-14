"""The ``crowdsource`` command-line interface.

A terminal-first client for the crowdsource platform — the CLI is a headline
feature of the product, not a test harness. It is a thin wrapper over the
compiled Rust core (the pyo3 ``crowdsource.Client``): the CLI marshals arguments,
resolves credentials, and formats output; all transport/auth/typing live in Rust.

Credentials resolve flag → env → ``~/.crowdsource/config.toml``. Every command
supports ``--format table|json|csv``. Exit codes: 0 ok, 1 usage, 2 auth,
3 network/server, 4 insufficient credits.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
from getpass import getpass

from crowdsource import __version__

from . import _fmt as fmt
from ._config import (
    KNOWN_KEYS,
    AuthError,
    load_config,
    make_client,
    resolve_format,
    save_config,
)

# ---- exit codes ----
EX_OK = 0
EX_USAGE = 1
EX_AUTH = 2
EX_NET = 3
EX_CREDITS = 4


# ---------------------------------------------------------------------------
# value / payload helpers (pure — unit tested without the extension)
# ---------------------------------------------------------------------------


def coerce_scalar(s: str):
    """Parse a CLI string into int, then float, else leave it as a string."""
    s = s.strip()
    for cast in (int, float):
        try:
            return cast(s)
        except ValueError:
            continue
    return s


def build_submission_body(
    value: str | None = None,
    *,
    raw_json: bool = False,
    file_text: str | None = None,
    stdin_text: str | None = None,
    s3_key: str | None = None,
) -> dict:
    """Build the ``submit`` body. Scalars wrap into ``{"value": x}`` (the platform
    convention, see ``seeds.toml``); ``--json``/``--file`` pass the object through;
    ``--stdin`` auto-detects JSON object vs scalar."""
    if s3_key:
        return {"s3_key": s3_key}
    if file_text is not None:
        return {"payload": json.loads(file_text)}
    if stdin_text is not None:
        text = stdin_text.strip()
        try:
            parsed = json.loads(text)
        except json.JSONDecodeError:
            return {"payload": {"value": coerce_scalar(text)}}
        if isinstance(parsed, (dict, list)):
            return {"payload": parsed}
        return {"payload": {"value": parsed}}
    if value is None:
        raise ValueError("a prediction value is required (or use --stdin / --file / --s3-key)")
    if raw_json:
        return {"payload": json.loads(value)}
    return {"payload": {"value": coerce_scalar(value)}}


def error_to_exit_code(exc: Exception) -> int:
    """Map an SDK exception to a CLI exit code by inspecting its message.

    The pyo3 layer raises ``ValueError`` whose text is the RFC 7807 detail plus
    ``(HTTP <status>)`` for server errors, or ``transport error: …`` for network
    failures."""
    msg = str(exc)
    m = re.search(r"HTTP (\d{3})", msg)
    code = int(m.group(1)) if m else None
    if code in (401, 403):
        return EX_AUTH
    if code == 402:
        return EX_CREDITS
    if code is not None and 500 <= code < 600:
        return EX_NET
    if "transport error" in msg:
        return EX_NET
    return EX_USAGE


def _fmt_of(args) -> str:
    return resolve_format(getattr(args, "format", None))


def _client(args, *, require_key: bool):
    return make_client(
        getattr(args, "api_key", None),
        getattr(args, "server", None),
        require_key=require_key,
    )


def _tier(me: dict) -> str:
    tier = str(me.get("rank_tier", "")).replace("_", " ").title()
    level = me.get("rank_level")
    return f"{tier} L{level}" if level else tier


def _handle(me: dict) -> str:
    return me.get("display_name") or me.get("email") or fmt.short_id(me.get("id"))


# ---------------------------------------------------------------------------
# auth
# ---------------------------------------------------------------------------


def cmd_login(args) -> int:
    key = (getattr(args, "api_key", None) or getpass("API key: ")).strip()
    if not key:
        print("error: no API key provided", file=sys.stderr)
        return EX_USAGE
    cfg = load_config()
    if getattr(args, "server", None):
        cfg["server"] = args.server
    # Validate the key by fetching the profile before storing it.
    client = make_client(key, getattr(args, "server", None))
    me = client.me()
    cfg["api_key"] = key
    save_config(cfg)
    print(f"✓ authenticated as {_handle(me)} ({_tier(me)})")
    return EX_OK


def cmd_logout(args) -> int:
    cfg = load_config()
    if cfg.pop("api_key", None) is None:
        print("no stored credentials")
        return EX_OK
    save_config(cfg)
    print("✓ logged out")
    return EX_OK


def cmd_whoami(args) -> int:
    client = _client(args, require_key=True)
    me = client.me()
    bal = client.credit_balance()
    if _fmt_of(args) == "json":
        fmt.render_detail({**me, "balance": bal}, [], "json")
        return EX_OK
    print(f"{_handle(me)} · {_tier(me)} · {fmt.credits(bal.get('balance'))} balance")
    return EX_OK


def cmd_version(args) -> int:
    server = None
    try:
        server = _client(args, require_key=False).version()
    except Exception:
        pass
    if _fmt_of(args) == "json":
        fmt.render_detail({"client": __version__, "server": server}, [], "json")
        return EX_OK
    print(f"crowdsource {__version__}")
    if isinstance(server, dict) and server.get("version"):
        print(f"server     {server['version']}")
    return EX_OK


# ---------------------------------------------------------------------------
# platform status
# ---------------------------------------------------------------------------


def _print_status(s: dict) -> None:
    print(f"{s.get('competitions_open', 0):,} competitions live")
    print(f"{fmt.credits(s.get('active_bounties', 0))} in active bounties")
    print(f"{s.get('predictions_today', 0):,} predictions today")
    print(f"{s.get('predictors', 0):,} predictors · {s.get('competitions_total', 0):,} competitions all-time")


def cmd_status(args) -> int:
    client = _client(args, require_key=False)
    if _fmt_of(args) == "json" and not getattr(args, "live", False):
        fmt.render_detail(client.summary(), [], "json")
        return EX_OK
    if not getattr(args, "live", False):
        _print_status(client.summary())
        return EX_OK
    interval = max(1, getattr(args, "interval", 5))
    try:
        while True:
            sys.stdout.write("\033[2J\033[H")  # clear screen
            print(f"crowdsource status · refreshing every {interval}s · Ctrl-C to stop\n")
            _print_status(client.summary())
            time.sleep(interval)
    except KeyboardInterrupt:
        return EX_OK


# ---------------------------------------------------------------------------
# competitions
# ---------------------------------------------------------------------------

_COMP_COLUMNS = [
    fmt.Column("id", "ID", fmt.short_id),
    fmt.Column("title", "TITLE", lambda v: str(v)[:30]),
    fmt.Column("competition_type", "TYPE"),
    fmt.Column("bounty_amount", "BOUNTY", fmt.credits, align="right"),
    fmt.Column("status", "STATUS"),
    fmt.Column("end_date", "CLOSES", fmt.time_left, align="right"),
]


def cmd_competitions(args) -> int:
    needs_key = bool(getattr(args, "mine", False) or getattr(args, "hosted", False))
    client = _client(args, require_key=needs_key)
    res = client.list_competitions(
        status=getattr(args, "status", None),
        competition_type=getattr(args, "type", None),
        tag=getattr(args, "tag", None),
        limit=getattr(args, "limit", None),
        mine=getattr(args, "mine", None) or None,
        hosted=getattr(args, "hosted", None) or None,
    )
    comps = res.get("competitions", [])
    fmt.render_rows(comps, _COMP_COLUMNS, _fmt_of(args))
    return EX_OK


def cmd_competition(args) -> int:
    client = _client(args, require_key=False)
    c = client.get_competition(args.id)
    if _fmt_of(args) == "json":
        fmt.render_detail(c, [], "json")
        return EX_OK
    top_n = c.get("bounty_top_n")
    fields = [
        ("title", c.get("title", "")),
        ("id", c.get("id", "")),
        ("type", c.get("competition_type", "")),
        ("metric", c.get("metric", "")),
        ("status", c.get("status", "")),
        ("bounty", f"{fmt.credits(c.get('bounty_amount'))} · {c.get('bounty_mode')} top {top_n}"),
        ("entry fee", fmt.credits(c.get("submission_fee"))),
        ("min rank", c.get("min_rank", "bronze")),
        ("tags", ", ".join(c.get("tags") or []) or "—"),
        ("closes", f"{c.get('end_date', '')} ({fmt.time_left(c.get('end_date', ''))})"),
    ]
    fmt.render_detail(c, fields, "table")
    return EX_OK


# ---------------------------------------------------------------------------
# predictions
# ---------------------------------------------------------------------------


def cmd_predict(args) -> int:
    file_text = None
    if getattr(args, "file", None):
        with open(args.file) as fh:
            file_text = fh.read()
    stdin_text = sys.stdin.read() if getattr(args, "stdin", False) else None
    body = build_submission_body(
        getattr(args, "value", None),
        raw_json=getattr(args, "json_value", False),
        file_text=file_text,
        stdin_text=stdin_text,
        s3_key=getattr(args, "s3_key", None),
    )
    client = _client(args, require_key=True)
    # Best-effort: fetch the title + fee to enrich the confirmation line.
    comp = None
    try:
        comp = client.get_competition(args.id)
    except Exception:
        pass
    row = client.submit(args.id, body)
    if _fmt_of(args) == "json":
        fmt.render_detail(row, [], "json")
        return EX_OK
    title = comp.get("title") if comp else None
    print(f"✓ prediction submitted{' for ' + title if title else ''}")
    if "payload" in body:
        print(f"  value: {json.dumps(body['payload'])}")
    else:
        print(f"  s3_key: {body['s3_key']}")
    if comp and comp.get("submission_fee") is not None:
        print(f"  cost: {fmt.credits(comp['submission_fee'])}")
    return EX_OK


# ---------------------------------------------------------------------------
# leaderboard
# ---------------------------------------------------------------------------

_LB_COLUMNS = [
    fmt.Column("rank", "#", lambda v: "—" if v is None else str(v), align="right"),
    fmt.Column("handle", "PLAYER"),
    fmt.Column("score", "SCORE", lambda v: "—" if v is None else f"{v:.4f}", align="right"),
    fmt.Column("payout", "PAYOUT", lambda v: "—" if v is None else fmt.credits(v), align="right"),
    fmt.Column("is_you", "", lambda v: "← you" if v else ""),
]


def cmd_leaderboard(args) -> int:
    client = _client(args, require_key=False)
    lb = client.leaderboard(args.competition)
    if _fmt_of(args) == "json":
        fmt.render_detail(lb, [], "json")
        return EX_OK
    fmt.render_rows(lb.get("entries", []), _LB_COLUMNS, _fmt_of(args))
    return EX_OK


# ---------------------------------------------------------------------------
# credits / balance / rank
# ---------------------------------------------------------------------------


def cmd_balance(args) -> int:
    client = _client(args, require_key=True)
    b = client.credit_balance()
    if _fmt_of(args) == "json":
        fmt.render_detail(b, [], "json")
        return EX_OK
    print(f"{fmt.credits(b.get('balance'))} available")
    print(f"  {fmt.credits(b.get('purchased_total'))} purchased")
    print(f"  {fmt.credits(b.get('earned_total'))} earned")
    return EX_OK


def cmd_rank(args) -> int:
    client = _client(args, require_key=True)
    action = getattr(args, "action", None)
    if action == "up":
        r = client.rank_up()
    elif action == "down":
        r = client.rank_down()
    else:
        me = client.me()
        if _fmt_of(args) == "json":
            fmt.render_detail(me, [], "json")
            return EX_OK
        print(_tier(me))
        return EX_OK
    if _fmt_of(args) == "json":
        fmt.render_detail(r, [], "json")
        return EX_OK
    tier = str(r.get("rank_tier", "")).title()
    print(f"✓ now {tier} L{r.get('rank_level')} · {fmt.credits(r.get('delta'))} · balance {fmt.credits(r.get('balance'))}")
    return EX_OK


_PACK_COLUMNS = [
    fmt.Column("price_cents", "PRICE", lambda v: f"${(v or 0) / 100:,.2f}", align="right"),
    fmt.Column("credits_granted", "CREDITS", fmt.credits, align="right"),
]


def cmd_credits_packs(args) -> int:
    client = _client(args, require_key=False)
    packs = (client.economic_config().get("config") or {}).get("credit_packs", [])
    fmt.render_rows(packs, _PACK_COLUMNS, _fmt_of(args))
    return EX_OK


def cmd_credits_buy(args) -> int:
    import webbrowser

    cents = int(round(args.amount * 100))
    client = _client(args, require_key=True)
    packs = (client.economic_config().get("config") or {}).get("credit_packs", [])
    valid = {p.get("price_cents") for p in packs}
    if cents not in valid:
        opts = ", ".join(f"${(p.get('price_cents') or 0) / 100:g}" for p in packs)
        print(f"error: no credit pack at ${args.amount:g}. Available: {opts}", file=sys.stderr)
        return EX_USAGE
    res = client.create_checkout(cents)
    url = res.get("checkout_url", "")
    print(f"→ opening checkout for ${args.amount:g} in your browser…")
    print(f"  {url}")
    try:
        webbrowser.open(url)
    except Exception:
        pass
    return EX_OK


# ---------------------------------------------------------------------------
# api keys
# ---------------------------------------------------------------------------

_KEY_COLUMNS = [
    fmt.Column("id", "ID", fmt.short_id),
    fmt.Column("name", "NAME"),
    fmt.Column("created_at", "CREATED", lambda v: str(v or "")[:10]),
    fmt.Column("last_used_at", "LAST USED", lambda v: str(v)[:10] if v else "never"),
]


def cmd_apikeys_list(args) -> int:
    client = _client(args, require_key=True)
    fmt.render_rows(client.list_api_keys(), _KEY_COLUMNS, _fmt_of(args))
    return EX_OK


def cmd_apikeys_create(args) -> int:
    client = _client(args, require_key=True)
    r = client.create_api_key(args.name)
    if _fmt_of(args) == "json":
        fmt.render_detail(r, [], "json")
        return EX_OK
    print(f"✓ created key '{r.get('name')}' ({fmt.short_id(r.get('id'))})")
    print(f"  secret: {r.get('secret')}")
    print("  (this is the only time the secret is shown — store it now)")
    return EX_OK


def cmd_apikeys_delete(args) -> int:
    client = _client(args, require_key=True)
    client.delete_api_key(args.id)
    print(f"✓ deleted key {fmt.short_id(args.id)}")
    return EX_OK


# ---------------------------------------------------------------------------
# data sources
# ---------------------------------------------------------------------------

_DS_COLUMNS = [
    fmt.Column("id", "ID", fmt.short_id),
    fmt.Column("name", "NAME"),
    fmt.Column("http_method", "METHOD"),
    fmt.Column("url", "URL", lambda v: str(v or "")[:48]),
]


def cmd_datasources(args) -> int:
    client = _client(args, require_key=False)
    fmt.render_rows(client.list_data_sources(), _DS_COLUMNS, _fmt_of(args))
    return EX_OK


# ---------------------------------------------------------------------------
# host / sponsor: create, publish, close
# ---------------------------------------------------------------------------


def cmd_create(args) -> int:
    req = {
        "title": args.title,
        "description": args.description or "",
        "competition_type": args.type,
        "metric": args.metric,
        "bounty_amount": args.bounty,
        "bounty_mode": args.bounty_mode,
        "bounty_top_n": args.bounty_top_n,
        "end_date": args.closes,
    }
    if args.tag:
        req["tags"] = args.tag
    if args.min_rank:
        req["min_rank"] = args.min_rank
    client = _client(args, require_key=True)
    c = client.create_competition(req)
    if _fmt_of(args) == "json":
        fmt.render_detail(c, [], "json")
        return EX_OK
    print(f"✓ created competition {fmt.short_id(c.get('id'))} ({c.get('status')})")
    print(f"  bounty: {fmt.credits(c.get('bounty_amount'))}")
    return EX_OK


def cmd_publish(args) -> int:
    client = _client(args, require_key=True)
    c = client.publish_competition(args.id)
    print(f"✓ competition {fmt.short_id(c.get('id'))} is now {c.get('status')}")
    return EX_OK


def cmd_close(args) -> int:
    client = _client(args, require_key=True)
    c = client.close_competition(args.id)
    print(f"✓ competition {fmt.short_id(c.get('id'))} is now {c.get('status')}")
    return EX_OK


# ---------------------------------------------------------------------------
# config
# ---------------------------------------------------------------------------


def cmd_config_get(args) -> int:
    print(load_config().get(args.key, ""))
    return EX_OK


def cmd_config_set(args) -> int:
    if args.key not in KNOWN_KEYS:
        print(f"error: unknown config key '{args.key}'. Known: {', '.join(KNOWN_KEYS)}", file=sys.stderr)
        return EX_USAGE
    cfg = load_config()
    cfg[args.key] = args.value
    save_config(cfg)
    print(f"✓ set {args.key}")
    return EX_OK


def cmd_config_help(args) -> int:
    print("usage: crowdsource config (get <key> | set <key> <value>)", file=sys.stderr)
    print(f"known keys: {', '.join(KNOWN_KEYS)}", file=sys.stderr)
    return EX_USAGE


# ---------------------------------------------------------------------------
# parser
# ---------------------------------------------------------------------------


def build_parser() -> argparse.ArgumentParser:
    # Global options live on a parent parser shared by the root and every
    # subcommand (default=SUPPRESS so a subcommand's value doesn't clobber a
    # value given before the subcommand).
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--api-key", default=argparse.SUPPRESS, help="API key (overrides env/config)")
    common.add_argument("--server", default=argparse.SUPPRESS, help="server base URL")
    common.add_argument(
        "--format",
        choices=("table", "json", "csv"),
        default=argparse.SUPPRESS,
        help="output format (default: table)",
    )
    common.add_argument("-q", "--quiet", action="store_true", default=argparse.SUPPRESS)

    parser = argparse.ArgumentParser(
        prog="crowdsource",
        description="Terminal-first client for the crowdsource platform.",
        parents=[common],
    )
    parser.add_argument("--version", action="version", version=f"crowdsource {__version__}")
    sub = parser.add_subparsers(dest="command", required=True, metavar="<command>")

    def add(name, func, help_, **kw):
        p = sub.add_parser(name, parents=[common], help=help_, **kw)
        p.set_defaults(func=func)
        return p

    # auth
    p = add("login", cmd_login, "store and validate an API key")
    add("logout", cmd_logout, "remove stored credentials")
    add("whoami", cmd_whoami, "show the authenticated user")
    add("version", cmd_version, "show client + server versions")

    # status
    p = add("status", cmd_status, "platform stats")
    p.add_argument("--live", action="store_true", help="poll continuously")
    p.add_argument("--interval", type=int, default=5, help="seconds between refreshes (with --live)")

    # competitions
    p = add("competitions", cmd_competitions, "list competitions")
    p.add_argument("--status", help="draft|open|closed|scoring|scored|scoring_failed")
    p.add_argument("--type", help="classification|regression")
    p.add_argument("--tag", help="filter by tag")
    p.add_argument("--limit", type=int, help="max results")
    p.add_argument("--mine", action="store_true", help="only competitions you've entered")
    p.add_argument("--hosted", action="store_true", help="only competitions your org hosts")

    p = add("competition", cmd_competition, "show one competition")
    p.add_argument("id", help="competition id (UUID)")

    # predict
    p = add("predict", cmd_predict, "submit a prediction")
    p.add_argument("id", help="competition id (UUID)")
    p.add_argument("value", nargs="?", help='prediction value (wrapped as {"value": …})')
    p.add_argument("--json", dest="json_value", action="store_true", help="treat value as raw JSON payload")
    p.add_argument("--stdin", action="store_true", help="read the value/payload from stdin")
    p.add_argument("--file", help="read a JSON payload from a file")
    p.add_argument("--s3-key", dest="s3_key", help="submit by object-storage key instead of inline payload")

    # leaderboard
    p = add("leaderboard", cmd_leaderboard, "competition leaderboard")
    p.add_argument("competition", help="competition id (UUID)")

    # credits / balance / rank
    add("balance", cmd_balance, "credit balance")

    p = add("rank", cmd_rank, "show rank, or rank up/down")
    p.add_argument("action", nargs="?", choices=("up", "down"), help="change rank")

    cr = add("credits", _credits_help, "buy credits / list packs")
    crsub = cr.add_subparsers(dest="cr_cmd", metavar="<subcommand>")
    crsub.add_parser("packs", parents=[common], help="list credit packs").set_defaults(func=cmd_credits_packs)
    b = crsub.add_parser("buy", parents=[common], help="buy a credit pack")
    b.add_argument("amount", type=float, help="dollars, e.g. 10 = $10")
    b.set_defaults(func=cmd_credits_buy)

    # api keys
    ak = add("api-keys", cmd_apikeys_list, "manage API keys (default: list)")
    aksub = ak.add_subparsers(dest="ak_cmd", metavar="<subcommand>")
    aksub.add_parser("list", parents=[common], help="list keys").set_defaults(func=cmd_apikeys_list)
    c = aksub.add_parser("create", parents=[common], help="create a key")
    c.add_argument("--name", required=True)
    c.set_defaults(func=cmd_apikeys_create)
    d = aksub.add_parser("delete", parents=[common], help="revoke a key")
    d.add_argument("id")
    d.set_defaults(func=cmd_apikeys_delete)

    # data sources
    add("data-sources", cmd_datasources, "list data sources")

    # host / sponsor
    p = add("create", cmd_create, "create a competition")
    p.add_argument("--title", required=True)
    p.add_argument("--type", required=True, help="classification|regression")
    p.add_argument("--metric", required=True, help="accuracy|f1_macro|rmse|mae|r2")
    p.add_argument("--bounty", type=int, required=True, help="bounty amount (credits)")
    p.add_argument("--closes", required=True, help="end date (RFC 3339, e.g. 2026-07-01T20:00:00Z)")
    p.add_argument("--description", default="")
    p.add_argument("--bounty-mode", dest="bounty_mode", default="top_n_equal", help="top_n_equal|top_n_weighted")
    p.add_argument("--bounty-top-n", dest="bounty_top_n", type=int, default=1)
    p.add_argument("--tag", action="append", help="repeatable")
    p.add_argument("--min-rank", dest="min_rank", help="bronze|silver|gold|platinum|diamond")

    p = add("publish", cmd_publish, "publish a draft competition")
    p.add_argument("id")
    p = add("close", cmd_close, "close a competition")
    p.add_argument("id")

    # config
    cfgp = add("config", cmd_config_help, "get/set CLI config")
    cfgsub = cfgp.add_subparsers(dest="cfg_cmd", metavar="<subcommand>")
    g = cfgsub.add_parser("get", parents=[common], help="read a config value")
    g.add_argument("key")
    g.set_defaults(func=cmd_config_get)
    s = cfgsub.add_parser("set", parents=[common], help="write a config value")
    s.add_argument("key")
    s.add_argument("value")
    s.set_defaults(func=cmd_config_set)

    return parser


def _credits_help(args) -> int:
    print("usage: crowdsource credits (packs | buy <amount>)", file=sys.stderr)
    return EX_USAGE


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    func = getattr(args, "func", None)
    if func is None:
        parser.print_help(sys.stderr)
        return EX_USAGE
    try:
        return func(args)
    except AuthError as e:
        print(f"error: {e}", file=sys.stderr)
        return EX_AUTH
    except KeyboardInterrupt:
        return 130
    except BrokenPipeError:
        return EX_OK
    except Exception as e:  # noqa: BLE001 — surface SDK/transport errors as exit codes
        print(f"error: {e}", file=sys.stderr)
        return error_to_exit_code(e)


if __name__ == "__main__":
    sys.exit(main())
