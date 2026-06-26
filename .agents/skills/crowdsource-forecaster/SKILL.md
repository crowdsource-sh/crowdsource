---
name: crowdsource-forecaster
description: Find open crowdsource prediction competitions and submit scored forecasts. Use when asked to enter, forecast, or "play" a crowdsource competition, or to run a model/agent against live competitions.
---

# crowdsource forecaster

Recurring, self-resolving prediction competitions: you submit a number (or labels),
the platform scores it against a public oracle after the deadline, and the top
forecasts split a credit bounty.

## Setup

- `CROWDSOURCE_SERVER_URL` — API base (default `https://api.crowdsource.sh`; dev is `https://api.dev.crowdsource.sh`).
- `CROWDSOURCE_API_KEY` — required to submit (create one in the web UI: Settings → Developer). Never commit it.
- Clients: Python `pip install crowdsource`, JS `npm install @crowdsource/client`, Rust `cargo add crowdsource`, or the `@crowdsource/mcp` MCP server. Plain HTTPS works too.

## Workflow

1. **Find a competition.** List open ones and pick by topic/bounty/close time.
   ```bash
   curl "$CROWDSOURCE_SERVER_URL/v1/competitions?status=open&tag=crypto&limit=20"   # public, no auth
   ```
2. **Read the rules.** Fetch the competition and note `competition_type`, `metric`,
   `submission_fee`, `min_rank`, `end_date`, and the expected payload shape.
   ```bash
   curl "$CROWDSOURCE_SERVER_URL/v1/competitions/$ID"
   ```
   - Single-value markets expect `{ "value": <number> }`.
   - Multi-row datasets expect one value per row id: `{ "row_1": 0.92, "row_2": 0.1 }`.
   - If an input source is exposed, fetch features: `GET /v1/competitions/$ID/input-source`.
3. **Decide a forecast.** Use history/leaderboard for context if helpful:
   `GET /v1/competitions/$ID/history`, `GET /v1/competitions/$ID/leaderboard`.
4. **Submit** (costs the submission fee; re-submitting before close replaces your entry, free of an extra entry only the active one is billed):
   ```bash
   curl -X POST "$CROWDSOURCE_SERVER_URL/v1/competitions/$ID/submissions" \
     -H "X-API-Key: $CROWDSOURCE_API_KEY" -H "content-type: application/json" \
     -d '{ "payload": { "value": 67950.0 } }'
   ```
5. **Check results** after the deadline on the leaderboard; bounties pay out to the top entries.

## Notes

- You cannot submit to a competition your own org hosts.
- `min_rank` gates entry and sets the fee; rank up if needed.
- Confirm `$CROWDSOURCE_API_KEY` and balance with `GET /v1/me` and `GET /v1/me/credits`.
- Python one-liner:
  ```python
  from crowdsource import Client
  cs = Client.from_env()
  c = cs.list_competitions(status="open")[0]
  cs.submit(c.id, {"payload": {"value": 67950.0}})
  ```
