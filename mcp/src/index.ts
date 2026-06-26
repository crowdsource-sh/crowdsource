#!/usr/bin/env node
/**
 * crowdsource MCP server.
 *
 * Connects AI agents to live crowdsource prediction competitions over the Model
 * Context Protocol: discover open competitions, inspect schemas/leaderboards/
 * history (read-only resources + tools), and submit forecasts (authenticated
 * tools). Wraps the public REST API; no SDK build step required.
 *
 * Config (env):
 *   CROWDSOURCE_SERVER_URL  API base URL (default https://api.crowdsource.sh)
 *   CROWDSOURCE_API_KEY     API key (X-API-Key) — required only for write/account tools
 */
import { McpServer, ResourceTemplate } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';

const BASE = (process.env.CROWDSOURCE_SERVER_URL || 'https://api.crowdsource.sh').replace(/\/$/, '');
const API_KEY = process.env.CROWDSOURCE_API_KEY;

type ApiOpts = { method?: string; body?: unknown; auth?: boolean };

async function api(path: string, opts: ApiOpts = {}): Promise<unknown> {
    const { method = 'GET', body, auth = false } = opts;
    const headers: Record<string, string> = { accept: 'application/json' };
    if (body !== undefined) headers['content-type'] = 'application/json';
    if (auth) {
        if (!API_KEY) throw new Error('CROWDSOURCE_API_KEY is required for this action; set it in the server environment.');
        headers['X-API-Key'] = API_KEY;
    }
    const res = await fetch(`${BASE}${path}`, {
        method,
        headers,
        body: body !== undefined ? JSON.stringify(body) : undefined,
    });
    const text = await res.text();
    if (!res.ok) throw new Error(`${method} ${path} -> ${res.status} ${res.statusText}: ${text.slice(0, 400)}`);
    return text ? JSON.parse(text) : null;
}

type ToolResult = { content: { type: 'text'; text: string }[]; isError?: boolean };

function ok(data: unknown): ToolResult {
    return { content: [{ type: 'text', text: JSON.stringify(data, null, 2) }] };
}
function fail(e: unknown): ToolResult {
    return { content: [{ type: 'text', text: `Error: ${e instanceof Error ? e.message : String(e)}` }], isError: true };
}

const server = new McpServer({ name: 'crowdsource', version: '0.1.0' });

// --- Tools -----------------------------------------------------------------

server.registerTool(
    'list_competitions',
    {
        title: 'List competitions',
        description: 'List prediction competitions, newest first. Filter by status, type, and tag. Read-only, no auth.',
        inputSchema: {
            status: z.enum(['open', 'closed', 'scored', 'all']).optional().describe('Lifecycle filter (default open).'),
            type: z.enum(['regression', 'classification']).optional(),
            tag: z.string().optional().describe('Category tag, e.g. "crypto", "weather", "free".'),
            limit: z.number().int().min(1).max(100).optional().describe('Max results (default 25).'),
        },
    },
    async ({ status, type, tag, limit }) => {
        try {
            const q = new URLSearchParams();
            q.set('status', status ?? 'open');
            if (type) q.set('type', type);
            if (tag) q.set('tag', tag);
            q.set('limit', String(limit ?? 25));
            return ok(await api(`/v1/competitions?${q.toString()}`));
        } catch (e) {
            return fail(e);
        }
    }
);

server.registerTool(
    'search_competitions',
    {
        title: 'Search competitions',
        description: 'Search open competitions by free-text query over title and description (optionally filtered by tag/type). Read-only.',
        inputSchema: {
            query: z.string().describe('Text to match in the title or description.'),
            tag: z.string().optional(),
            type: z.enum(['regression', 'classification']).optional(),
        },
    },
    async ({ query, tag, type }) => {
        try {
            const q = new URLSearchParams({ status: 'open', limit: '100' });
            if (tag) q.set('tag', tag);
            if (type) q.set('type', type);
            const res = (await api(`/v1/competitions?${q.toString()}`)) as { competitions?: Record<string, unknown>[] };
            const needle = query.toLowerCase();
            const matches = (res.competitions ?? []).filter((c) =>
                `${c.title ?? ''} ${c.description ?? ''}`.toLowerCase().includes(needle)
            );
            return ok({ query, count: matches.length, competitions: matches.slice(0, 25) });
        } catch (e) {
            return fail(e);
        }
    }
);

server.registerTool(
    'get_competition',
    {
        title: 'Get competition',
        description: 'Fetch a single competition by id (rules, metric, bounty, schedule). Read-only.',
        inputSchema: { id: z.string().describe('Competition id (UUID).') },
    },
    async ({ id }) => {
        try {
            return ok(await api(`/v1/competitions/${encodeURIComponent(id)}`));
        } catch (e) {
            return fail(e);
        }
    }
);

server.registerTool(
    'get_competition_history',
    {
        title: 'Get competition history',
        description: 'Past cycles of a recurring competition (prior results), newest first. Read-only.',
        inputSchema: { id: z.string(), limit: z.number().int().min(1).max(100).optional() },
    },
    async ({ id, limit }) => {
        try {
            const q = limit ? `?limit=${limit}` : '';
            return ok(await api(`/v1/competitions/${encodeURIComponent(id)}/history${q}`));
        } catch (e) {
            return fail(e);
        }
    }
);

server.registerTool(
    'get_leaderboard',
    {
        title: 'Get leaderboard',
        description: 'Current standings for a competition (ranks, scores, payouts). Read-only.',
        inputSchema: { id: z.string() },
    },
    async ({ id }) => {
        try {
            return ok(await api(`/v1/competitions/${encodeURIComponent(id)}/leaderboard`));
        } catch (e) {
            return fail(e);
        }
    }
);

server.registerTool(
    'submit_prediction',
    {
        title: 'Submit prediction',
        description:
            'Submit (or replace) your forecast for a competition. Requires CROWDSOURCE_API_KEY and costs the submission fee. The payload is an object keyed by the schema row ids; single-value markets use { "value": <number> }.',
        inputSchema: {
            id: z.string().describe('Competition id (UUID).'),
            payload: z.record(z.string(), z.unknown()).describe('Prediction payload, e.g. { "value": 67950.0 }.'),
        },
    },
    async ({ id, payload }) => {
        try {
            return ok(await api(`/v1/competitions/${encodeURIComponent(id)}/submissions`, { method: 'POST', body: { payload }, auth: true }));
        } catch (e) {
            return fail(e);
        }
    }
);

server.registerTool(
    'get_me',
    {
        title: 'Get current user',
        description: 'Your account profile and resolved capabilities. Requires CROWDSOURCE_API_KEY.',
        inputSchema: {},
    },
    async () => {
        try {
            return ok(await api(`/v1/me`, { auth: true }));
        } catch (e) {
            return fail(e);
        }
    }
);

server.registerTool(
    'get_credits',
    {
        title: 'Get credit balance',
        description: 'Your credit balance and accounting summary. Requires CROWDSOURCE_API_KEY.',
        inputSchema: {},
    },
    async () => {
        try {
            return ok(await api(`/v1/me/credits`, { auth: true }));
        } catch (e) {
            return fail(e);
        }
    }
);

// --- Resources (read-only context) -----------------------------------------

server.registerResource(
    'competition',
    new ResourceTemplate('crowdsource://competition/{id}', { list: undefined }),
    { title: 'Competition', description: 'A competition record by id.' },
    async (uri, { id }) => ({
        contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(await api(`/v1/competitions/${encodeURIComponent(String(id))}`), null, 2) }],
    })
);

server.registerResource(
    'leaderboard',
    new ResourceTemplate('crowdsource://leaderboard/{id}', { list: undefined }),
    { title: 'Leaderboard', description: 'Standings for a competition by id.' },
    async (uri, { id }) => ({
        contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(await api(`/v1/competitions/${encodeURIComponent(String(id))}/leaderboard`), null, 2) }],
    })
);

server.registerResource(
    'history',
    new ResourceTemplate('crowdsource://history/{id}', { list: undefined }),
    { title: 'Competition history', description: 'Past cycles of a recurring competition by id.' },
    async (uri, { id }) => ({
        contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(await api(`/v1/competitions/${encodeURIComponent(String(id))}/history`), null, 2) }],
    })
);

server.registerResource(
    'openapi',
    'crowdsource://openapi',
    { title: 'OpenAPI spec', description: 'The crowdsource REST API OpenAPI specification.' },
    async (uri) => ({
        contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(await api(`/v1/openapi.json`), null, 2) }],
    })
);

// --- Prompts ---------------------------------------------------------------

server.registerPrompt(
    'forecast_competition',
    {
        title: 'Forecast a competition',
        description: 'Analyze a competition and propose a forecast (does not submit unless you ask).',
        argsSchema: { id: z.string().describe('Competition id to forecast.') },
    },
    ({ id }) => ({
        messages: [
            {
                role: 'user',
                content: {
                    type: 'text',
                    text: `Use get_competition on "${id}" (and get_competition_history / get_leaderboard if helpful). Explain what is being predicted and the scoring metric, then propose a single forecast value with brief reasoning. Do not call submit_prediction unless I explicitly ask.`,
                },
            },
        ],
    })
);

server.registerPrompt(
    'compare_forecasts',
    {
        title: 'Compare open competitions',
        description: 'Survey open competitions in a tag and suggest where to focus.',
        argsSchema: { tag: z.string().describe('Category tag, e.g. "crypto".') },
    },
    ({ tag }) => ({
        messages: [
            {
                role: 'user',
                content: { type: 'text', text: `List open competitions with tag "${tag}" (list_competitions), then compare bounty, entry fee, metric, and close time, and recommend the best 2-3 to enter and why.` },
            },
        ],
    })
);

// --- Connect ---------------------------------------------------------------

const transport = new StdioServerTransport();
await server.connect(transport);
console.error('crowdsource MCP server running on stdio');
