"""Output helpers for the crowdsource CLI.

Pure-Python, no dependency on the compiled extension — so these are unit-testable
on their own. Three output formats are supported everywhere: ``table`` (default,
human), ``json`` (for piping into ``jq``), and ``csv``.
"""

from __future__ import annotations

import csv
import io
import json
import sys
from datetime import datetime, timezone

# Credit symbol — U+25CE BULLSEYE. The brand's unit mark.
CREDIT = "◎"


def credits(n: object) -> str:
    """Render a credit amount, e.g. ``◎1,240``. Non-numeric values pass through."""
    try:
        return f"{CREDIT}{int(n):,}"
    except (TypeError, ValueError):
        return f"{CREDIT}{n}"


def mask_key(key: str) -> str:
    """Mask an API key for display: keep the prefix and last 4 chars."""
    if not key:
        return ""
    if len(key) <= 10:
        return key[:3] + "…"
    return f"{key[:7]}…{key[-4:]}"


def short_id(value: object) -> str:
    """First 8 chars of a UUID-ish id (the human-friendly handle)."""
    s = str(value or "")
    return s[:8]


def _parse_dt(value: str) -> datetime | None:
    """Parse an RFC 3339 / ISO 8601 timestamp; tolerate a trailing ``Z``."""
    if not value:
        return None
    try:
        return datetime.fromisoformat(str(value).replace("Z", "+00:00"))
    except ValueError:
        return None


def time_left(end: str, now: datetime | None = None) -> str:
    """Human countdown to ``end`` (an ISO timestamp): ``3h 14m``, ``14d``, ``closed``."""
    dt = _parse_dt(end)
    if dt is None:
        return "—"
    now = now or datetime.now(timezone.utc)
    secs = int((dt - now).total_seconds())
    if secs <= 0:
        return "closed"
    days, rem = divmod(secs, 86400)
    hours, rem = divmod(rem, 3600)
    mins, _ = divmod(rem, 60)
    if days >= 1:
        return f"{days}d {hours}h" if hours else f"{days}d"
    if hours >= 1:
        return f"{hours}h {mins}m" if mins else f"{hours}h"
    return f"{mins}m"


class Column:
    """A table/csv column: a key into each row dict, a header, and a renderer."""

    def __init__(self, key: str, header: str, render=None, align: str = "left"):
        self.key = key
        self.header = header
        self.render = render or (lambda v: "" if v is None else str(v))
        self.align = align

    def cell(self, row: dict) -> str:
        return self.render(row.get(self.key))


def render_rows(rows: list[dict], columns: list[Column], fmt: str, *, file=None) -> None:
    """Render a list of dict rows in the requested format."""
    file = file or sys.stdout
    if fmt == "json":
        json.dump(rows, file, indent=2, default=str)
        file.write("\n")
        return
    if fmt == "csv":
        writer = csv.writer(file)
        writer.writerow([c.header for c in columns])
        for row in rows:
            writer.writerow([c.cell(row) for c in columns])
        return
    _render_table(rows, columns, file)


def _render_table(rows: list[dict], columns: list[Column], file) -> None:
    cells = [[c.cell(row) for c in columns] for row in rows]
    widths = [len(c.header) for c in columns]
    for line in cells:
        for i, val in enumerate(line):
            widths[i] = max(widths[i], len(val))

    def fmt_cell(text: str, width: int, align: str) -> str:
        return text.rjust(width) if align == "right" else text.ljust(width)

    header = "  ".join(fmt_cell(c.header, widths[i], c.align) for i, c in enumerate(columns))
    file.write(header.rstrip() + "\n")
    for line in cells:
        out = "  ".join(fmt_cell(val, widths[i], columns[i].align) for i, val in enumerate(line))
        file.write(out.rstrip() + "\n")


def render_detail(obj: dict, fields: list[tuple[str, str]], fmt: str, *, file=None) -> None:
    """Render a single object: ``json`` dumps it raw; ``table``/``csv`` show key/value pairs.

    ``fields`` is a list of ``(label, value)`` pairs already formatted for display.
    """
    file = file or sys.stdout
    if fmt == "json":
        json.dump(obj, file, indent=2, default=str)
        file.write("\n")
        return
    if fmt == "csv":
        writer = csv.writer(file)
        writer.writerow(["field", "value"])
        for label, value in fields:
            writer.writerow([label, value])
        return
    width = max((len(label) for label, _ in fields), default=0)
    for label, value in fields:
        file.write(f"{label.rjust(width)}  {value}\n")


def to_csv_string(rows: list[dict], columns: list[Column]) -> str:
    """Render rows to a CSV string (test helper)."""
    buf = io.StringIO()
    render_rows(rows, columns, "csv", file=buf)
    return buf.getvalue()
