# crowdsource CLI

A terminal-first interface to the crowdsource platform. Authenticate with an API key, browse competitions, submit predictions, check rankings — all from the command line.

## Why a CLI?

- API-first users (quants, ML engineers) want to automate without a browser
- The terminal aesthetic is core to the brand — the landing page hero IS a CLI
- Enables scripting: cron jobs, CI pipelines, model-output-to-submission workflows
- Pairs naturally with the REST API (the CLI is a thin wrapper)

## Auth

```
$ crowdsource login
API key: cs_****************************
✓ authenticated as user_0x7f (Diamond)

$ crowdsource whoami
user_0x7f · Diamond · ◎1,240 balance
```

- Reads API key from `--api-key`, `CROWDSOURCE_API_KEY` env var, or `~/.crowdsource/config.toml`
- `crowdsource login` prompts and stores the key
- `crowdsource logout` removes stored credentials

## Status & Dashboard

```
$ crowdsource status
47 competitions live
◎12,400 in active bounties
891 predictions submitted today
5 tiers · 12,000+ predictors

$ crowdsource status --live
(streams updates via polling)
```

## Competitions

```
$ crowdsource competitions
ID       NAME                    TYPE      BOUNTY    PREDICTIONS  CLOSES
c_8f2a   S&P 500 Weekly Close    numeric   ◎500      142          3h 14m
c_1b3d   BTC End of Month        numeric   ◎750      324          14d
c_a92c   NBA Finals Winner       choice    ◎2,000    891          2mo
c_f7e1   Fed Rate Decision       choice    ◎1,200    567          30d

$ crowdsource competitions --category finance --status open
$ crowdsource competitions --sort bounty --limit 10

$ crowdsource competition c_8f2a
S&P 500 Weekly Close
  type: numeric
  bounty: ◎500
  predictions: 142
  median: 5,410
  std dev: 42.3
  closes: 2026-04-16T20:00:00Z (3h 14m)
  your prediction: (none)
```

## Submitting Predictions

```
$ crowdsource predict c_8f2a 5418.50
✓ prediction submitted for S&P 500 Weekly Close
  value: 5418.50
  cost: ◎1

$ crowdsource predict c_a92c "Oklahoma City Thunder"
✓ prediction submitted for NBA Finals Winner
  value: Oklahoma City Thunder
  cost: ◎1

# Pipe from a script / model output
$ echo "5418.50" | crowdsource predict c_8f2a --stdin

# Batch predict from JSON
$ crowdsource predict --file predictions.json
```

## Rankings & Leaderboard

```
$ crowdsource rank
#47 / 12,000 · Gold ○
accuracy: 0.847
competitions entered: 23
predictions: 156

$ crowdsource leaderboard
#   USER          TIER        ACCURACY
1   user_0x7f     ◆ Diamond   0.934
2   user_0xa3     ◇ Platinum  0.921
3   user_0x1b     ○ Gold      0.908
4   user_0xd2     ○ Silver    0.891
5   user_0xf9     ○ Bronze    0.874

$ crowdsource leaderboard --competition c_8f2a
$ crowdsource leaderboard --tier diamond --limit 20
```

## Credits

```
$ crowdsource balance
◎1,240 available
  ◎1,000 purchased
  ◎240 earned

$ crowdsource credits buy 1000
→ opening checkout in browser...

$ crowdsource credits history --limit 10
```

## Creating Competitions (Sponsors)

```
$ crowdsource create \
    --name "S&P 500 Weekly Close" \
    --type numeric \
    --bounty 500 \
    --closes "2026-04-18T20:00:00Z" \
    --description "Predict the S&P 500 closing price on Friday"

✓ created competition c_8f2a (draft)
  fee: ◎50
  bounty: ◎500
  total cost: ◎550

$ crowdsource publish c_8f2a
✓ competition c_8f2a is now open

$ crowdsource close c_8f2a
✓ competition c_8f2a closed

$ crowdsource ground-truth c_8f2a 5412.30
✓ ground truth set, scoring in progress...
```

## API Keys

```
$ crowdsource api-keys
ID          NAME        CREATED
k_1a2b      default     2026-04-01
k_3c4d      ci-bot      2026-04-10

$ crowdsource api-keys create --name "model-runner"
✓ created key: cs_live_abc123...
  (this is the only time the secret is shown)

$ crowdsource api-keys delete k_3c4d
✓ deleted key k_3c4d
```

## Configuration

```toml
# ~/.crowdsource/config.toml
api_key = "cs_live_abc123..."
default_format = "table"   # table | json | csv
server = "https://api.crowdsource.sh"
```

```
$ crowdsource config set default_format json
$ crowdsource config get server
```

## Output Formats

All commands support `--format` flag:

```
$ crowdsource competitions --format json
$ crowdsource competitions --format csv
$ crowdsource competitions --format table  (default)
```

JSON output for piping into `jq`, scripts, other tools.

## Implementation Notes

- Entry point: `crowdsource` command via `[project.scripts]` in pyproject.toml
- Argument parsing: `argparse` or `click` (TBD)
- HTTP client: `httpx` for async support, or `requests` for simplicity
- Config storage: `~/.crowdsource/config.toml` via `tomllib` / `tomli-w`
- Could also be a Rust binary (`crowdsource-cli` crate) for zero-dependency distribution
- API key auth via `X-API-Key` header (already supported by the server)
- Credit symbol: ◎ (U+25CE, bullseye)
