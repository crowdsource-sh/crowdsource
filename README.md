# crowdsource

crowdsource.sh

[![Build Status](https://github.com/1kbgz/crowdsource/actions/workflows/build.yaml/badge.svg?branch=main&event=push)](https://github.com/1kbgz/crowdsource/actions/workflows/build.yaml)
[![codecov](https://codecov.io/gh/1kbgz/crowdsource/branch/main/graph/badge.svg)](https://codecov.io/gh/1kbgz/crowdsource)
[![License](https://img.shields.io/github/license/1kbgz/crowdsource)](https://github.com/1kbgz/crowdsource)
[![PyPI](https://img.shields.io/pypi/v/crowdsource.svg)](https://pypi.python.org/pypi/crowdsource)

The official first-party client for the [crowdsource](https://crowdsource.sh) prediction-competition platform: a single Rust core with Python, JavaScript/TypeScript, and a terminal-first **CLI** built on top of it. Anything you can do on the website, you can do from your shell or a script.

## Install

```bash
pip install crowdsource          # Python SDK + the `crowdsource` CLI
npm install @crowdsource/client  # JavaScript / TypeScript (browser, via wasm)
cargo add crowdsource            # Rust
```

## CLI

```bash
# Authenticate (stored in ~/.crowdsource/config.toml, chmod 600)
$ crowdsource login
API key: cs_****************************
✓ authenticated as quant@example.com (Diamond L2)

$ crowdsource status
48 competitions live
◎29,400 in active bounties
0 predictions today
1,204 predictors · 119 competitions all-time

$ crowdsource competitions --status open --limit 3
ID        TITLE              TYPE        BOUNTY  STATUS  CLOSES
fa347337  ETH/USD next hour  regression    ◎200  open        6m
daeb3525  BTC/USD next hour  regression    ◎200  open        6m
...

# Submit a prediction (scalars wrap as {"value": …})
$ crowdsource predict fa347337-… 3845.20
✓ prediction submitted for ETH/USD next hour
  value: {"value": 3845.2}
  cost: ◎5

# Pipe from a model / script; machine-readable output anywhere
$ my_model.py | crowdsource predict <id> --stdin
$ crowdsource competitions --format json | jq '.[].title'
```

Credentials resolve `--api-key` → `CROWDSOURCE_API_KEY` → `~/.crowdsource/config.toml`.
Every command supports `--format table|json|csv`. Exit codes: `0` ok, `1` usage,
`2` auth, `3` network/server, `4` insufficient credits. Run `crowdsource --help`
for the full command list (`whoami`, `competition`, `leaderboard`, `balance`,
`rank`, `credits`, `api-keys`, `create`/`publish`/`close`, `data-sources`, `config`).

## Python

```python
from crowdsource import Client

cs = Client.from_env()  # CROWDSOURCE_SERVER_URL + CROWDSOURCE_API_KEY
# or: Client("https://api.crowdsource.sh", "cs_sk_…")

for c in cs.list_competitions(status="open")["competitions"]:
    print(c["title"], c["bounty_amount"])

cs.submit(comp_id, {"payload": {"value": 67950.0}})
print(cs.credit_balance())
```

## JavaScript / TypeScript

```ts
import init, { Client } from "@crowdsource/client";

await init(); // load the wasm module once
const cs = new Client("https://api.crowdsource.sh", apiKey);
const { competitions } = await cs.listCompetitions("open");
await cs.submit(compId, { payload: { value: 67950.0 } });
```

## Rust

```rust
use crowdsource::{Client, CompetitionQuery, CreateSubmission};

let cs = Client::from_env()?;
let open = cs.list_competitions(&CompetitionQuery { status: Some("open".parse()?), ..Default::default() }).await?;
cs.submit(comp_id, &CreateSubmission::from_payload(serde_json::json!({"value": 67950.0}))).await?;
```

## Authentication

The platform accepts an API key (`X-API-Key`, both `cs_pub_…` publishable and
`cs_sk_…` secret keys) or a session bearer token. Create and manage keys with
`crowdsource api-keys`, in the web app, or via `Client.create_api_key`.

## Development

This is a polyglot repo — one Rust core (`rust/`) with wasm (`js/`) and pyo3
(`rust/python/`) bindings and a Python CLI (`crowdsource/`). See
[AGENTS.md](AGENTS.md) and the `Makefile` (`make develop`, `make build`,
`make test`, `make lint`).

> [!NOTE]
> This library was generated using [copier](https://copier.readthedocs.io/en/stable/) from the [Base Python Project Template repository](https://github.com/python-project-templates/base).
