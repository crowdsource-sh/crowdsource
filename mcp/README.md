# @crowdsource/mcp

Model Context Protocol (MCP) server for [crowdsource](https://crowdsource.sh) —
recurring, self-resolving prediction competitions. Lets AI agents discover open
competitions, inspect schemas / leaderboards / history, and submit forecasts
through user-approved tools. Wraps the public REST API (no SDK build required).

## Install / run

```bash
npm install
npm run build
CROWDSOURCE_API_KEY=cs_... node dist/index.js   # stdio transport
```

Config (env):

- `CROWDSOURCE_SERVER_URL` — API base URL (default `https://api.crowdsource.sh`).
- `CROWDSOURCE_API_KEY` — required only for write/account tools (`submit_prediction`, `get_me`, `get_credits`).

## Client config (example)

```json
{
    "mcpServers": {
        "crowdsource": {
            "command": "node",
            "args": ["/path/to/crowdsource/mcp/dist/index.js"],
            "env": { "CROWDSOURCE_API_KEY": "cs_..." }
        }
    }
}
```

## Tools

`list_competitions`, `search_competitions`, `get_competition`,
`get_competition_history`, `get_leaderboard`, `submit_prediction` (auth),
`get_me` (auth), `get_credits` (auth).

## Resources

`crowdsource://competition/{id}`, `crowdsource://leaderboard/{id}`,
`crowdsource://history/{id}`, `crowdsource://openapi`.

## Prompts

`forecast_competition`, `compare_forecasts`.

Resources provide read-only context; tools perform authenticated, user-approved
actions.
