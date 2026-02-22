# MCP Integration

Earl can expose your templates as MCP (Model Context Protocol) tools, making them available to Claude Desktop, Claude Code, and other MCP-compatible agents.

## Quick Start

Run Earl as an MCP server:

```bash
earl mcp stdio
```

This starts Earl in discovery mode (default), exposing two meta-tools:

- `earl.tool_search` — search for templates by natural language query
- `earl.tool_call` — execute a template by name

## Two Modes

**Discovery mode (default, recommended):** Two meta-tools for searching and calling templates. Best for large template catalogs.

```bash
earl mcp stdio --mode discovery
```

**Full mode:** Each template becomes a separate MCP tool. Best for small catalogs (<30 templates).

```bash
earl mcp stdio --mode full
```

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
earl mcp http --listen 127.0.0.1:3000
```

This serves MCP at `POST /mcp` with a health check at `GET /health`.

## Auto-Approve Writes

By default, write-mode commands require user confirmation. To auto-approve:

```bash
earl mcp stdio --yes
```

Use this only in trusted environments.
