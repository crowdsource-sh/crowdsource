---
name: crowdsource-oracle-validator
description: Verify that a candidate oracle URL + resolution_path resolves to a usable number before creating a crowdsource competition. Use when checking, debugging, or choosing a competition's data source.
---

# crowdsource oracle validator

A competition can only resolve if its oracle returns an objective value the
platform can extract. Validate the URL + `resolution_path` before authoring.

## Requirements an oracle must meet

- **HTTPS**, on a **public** host (no private/loopback/reserved IPs — the platform DNS-pins and blocks SSRF).
- Returns JSON (or XML/HTML/CSV with the matching `resolution_*` extractor).
- `resolution_path` extracts exactly **one number** (numeric strings are coerced).

## resolution_path cheatsheet

- Dotted keys + indices: `bitcoin.usd`, `data.0.value`
- Negative index (latest): `-1.kp_index`
- Aggregators over one `*` wildcard: `sum:data.stations.*.num_bikes_available` (also `avg`/`min`/`max`/`count`)
- A scalar response is wrapped as `{ "value": x }`.
- URL templating renders from the close instant: `{date}`, `{datetime}`, `{unixtime}`, offsets like `{date-1d}`, custom `{date:%Y/%m/%d}`.

## Workflow

1. **Fetch it** and confirm 200 + JSON:
   ```bash
   curl -fsS "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd"
   ```
2. **Apply the path** and confirm a finite number falls out:
   ```bash
   curl -fsS "$URL" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['bitcoin']['usd'])"
   ```
   For an aggregator/wildcard, sum/avg the matched leaves and confirm it's numeric.
3. **Check stability**: the value should exist on every cycle and settle by the close
   (use `resolution_offset_minutes` if the source lags). Re-fetch a minute later to confirm it's not transiently null.
4. **Templating**: if the URL needs the cycle date, verify the rendered URL (substitute `{date}` etc.) returns data for a past day.
5. **Report**: PASS with the extracted value, or FAIL with the reason (non-HTTPS, non-public host, 4xx/5xx, path missing, non-numeric, or intermittently null).

## Common failures

- Path points at an object/array, not a number → narrow it or add an aggregator.
- Endpoint requires a key → not allowed for user competitions (`{secret:NAME}` is admin-only).
- Value is null outside market hours → add an offset or pick a different field.
