# MCP Integration

Earl can expose your templates as MCP (Model Context Protocol) tools, making them available to Claude Desktop, Claude Code, and other MCP-compatible agents.

## Quick Start

Run Earl as an MCP server:

```bash
earl mcp stdio
```

This starts Earl in full mode (default), where each template becomes a separate MCP tool.

## Two Modes

**Full mode (default):** Each template becomes a separate MCP tool. Best for small catalogs (<30 templates).

```bash
earl mcp stdio --mode full
```

**Discovery mode (recommended for large catalogs):** Two meta-tools for searching and calling templates.

```bash
earl mcp stdio --mode discovery
```

Discovery mode exposes two meta-tools:

- `earl.tool_search` — search for templates by natural language query
- `earl.tool_call` — execute a template by name

## Claude Desktop Configuration

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "earl": {
      "command": "earl",
      "args": ["mcp", "stdio"]
    }
  }
}
```

## Claude Code Configuration

Add to `.claude/settings.json` in your project:

```json
{
  "mcpServers": {
    "earl": {
      "command": "earl",
      "args": ["mcp", "stdio"]
    }
  }
}
```

## HTTP Transport

For remote or shared deployments, use HTTP transport:

```bash
earl mcp http --listen 127.0.0.1:8977 --allow-unauthenticated
```

This serves MCP at `POST /mcp` with a health check at `GET /health`. The default listen address is `127.0.0.1:8977`.

HTTP transport requires either JWT authentication (`[auth.jwt]` in config) or `--allow-unauthenticated`. See the full MCP docs for JWT and policy configuration.

## Auto-Approve Writes

By default, write-mode commands require user confirmation. To auto-approve:

```bash
earl mcp stdio --yes
```

Use this only in trusted environments.
