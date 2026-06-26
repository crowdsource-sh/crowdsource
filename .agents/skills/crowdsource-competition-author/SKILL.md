---
name: crowdsource-competition-author
description: Draft and create a new crowdsource prediction competition (one-off or recurring) wired to a public oracle. Use when asked to create, author, sponsor, or seed a competition.
---

# crowdsource competition author

Create a competition that scores forecasts against an objective, public value.
You set the question, the oracle that resolves it, the metric, and the bounty.

## Setup

- `CROWDSOURCE_SERVER_URL`, `CROWDSOURCE_API_KEY` (see crowdsource-forecaster). Creating
  a competition charges a creation fee and escrows the bounty, so the owner needs credits.
- Validate the oracle first with the **crowdsource-oracle-validator** skill.

## What a competition needs

- `title`, `description`
- `competition_type`: `regression` (predict a number) or `classification` (3+ classes or many rows)
- `metric`: regression `rmse|mae|r2`; classification `accuracy|f1_macro`
- `end_date` (ISO 8601, future)
- `bounty_amount` (≥ the per-tier minimum), `bounty_mode` (`top_n_equal|top_n_weighted`), `bounty_top_n`
- `min_rank` (default `bronze`) — gates entry and sets fees
- Resolution: an inline `oracle_url` + `resolution_path` (dotted path into the JSON), or a registered `resolution_source_id`, or push ground truth later
- Recurring (optional): `recurring_interval` (`hourly|daily|weekly|monthly`) + `recurring_close_time` + `recurring_timezone`
- `show_host` (optional) — credit the organizer on the page

## Workflow

1. **Pick an oracle** (public, key-free HTTPS) and the `resolution_path` that extracts one number. Validate it (oracle-validator skill).
2. **Create the draft:**
   ```bash
   curl -X POST "$CROWDSOURCE_SERVER_URL/v1/competitions" \
     -H "X-API-Key: $CROWDSOURCE_API_KEY" -H "content-type: application/json" \
     -d '{
       "title": "BTC/USD daily close",
       "description": "Predict the BTC/USD price at 00:00 UTC. Submit {\"value\": <price>}.",
       "competition_type": "regression",
       "metric": "rmse",
       "bounty_amount": 500, "bounty_mode": "top_n_weighted", "bounty_top_n": 3, "bounty_weights": [50,30,20],
       "end_date": "2026-07-01T00:00:00Z",
       "oracle_url": "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd",
       "resolution_path": "bitcoin.usd",
       "recurring_interval": "daily", "recurring_close_time": "00:00", "recurring_timezone": "UTC"
     }'
   ```
3. **Publish** (draft → open): `POST /v1/competitions/$ID/publish`. Edits are only allowed while in `draft`.
4. **Recurring** competitions reopen each cycle automatically; the creator is debited the next cycle's fee + bounty.

## Guardrails

- No single-individual outcomes (no political campaigns or one athlete's stats) and no lone yes/no binary bets.
- `{secret:NAME}` oracle placeholders are admin-only.
- The bounty must meet the per-tier minimum; the creator must hold `creation_fee + bounty`.
