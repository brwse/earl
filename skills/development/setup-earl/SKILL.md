---
name: setup-earl
description: Installs Earl, configures MCP integration for your agent platform, writes CLAUDE.md instructions, and routes to template creation or migration. Use when setting up Earl for the first time, when a new developer is onboarding to a project that uses Earl, or when Earl needs to be connected to an agent platform.
---

# Setup Earl

Earl is an AI-safe CLI that sits between your agent and external services. Agents run
`earl call provider.command --param value` instead of raw `curl`, `gh`, `stripe-cli`, etc.
Secrets stay in the OS keychain. Every request follows a reviewed HCL template.

## Process

1. **Install** — get Earl running
2. **Demo** — show a working example immediately
3. **Connect** — configure MCP for your agent platform and write CLAUDE.md instructions
4. **Route** — migrate existing CLI calls or create a new template
5. **Lock down** — optionally enforce Earl usage at the platform level

---

## Phase 1: Install

Check if Earl is already installed:

```bash
which earl && earl --version
```

If installed, print the version and skip to Phase 2.

If not installed, detect the environment and install:

**macOS / Linux (prefer — no sudo required):**

```bash
cargo install earl
```

This requires the Rust toolchain **and Node.js + pnpm** (Earl embeds web playground assets at compile time). If either is missing, fall back to the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/brwse/earl/main/scripts/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/brwse/earl/main/scripts/install.ps1 | iex
```

After install, verify:

```bash
earl doctor
```

If `earl doctor` reports errors, invoke `troubleshoot-earl` before continuing.

---

## Phase 2: Quick Demo

Import the no-auth system template and run it so the user sees Earl work immediately.
First check if it's already imported to avoid overwriting any customizations:

```bash
earl templates list | grep -E "^system\." || earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/bash/system.hcl
earl call --yes --json system.list_files --path .
```

This lists files in the current directory. The user now has a mental model: templates define
commands, `earl call` runs them.

Show available templates:

```bash
earl templates list
```

---

## Phase 3: Connect to Agent Platform

Detect the agent platform by checking all of these — multiple can match (e.g. a project may
have both `.claude/` and `.cursor/`). Configure every matching platform:

| Check                                                                                                                                                                                               | Platform       | MCP config path         |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | ----------------------- |
| `.claude/` directory exists in project                                                                                                                                                              | Claude Code    | `.claude/settings.json` |
| `~/Library/Application Support/Claude/claude_desktop_config.json` exists (macOS), `%APPDATA%\Claude\claude_desktop_config.json` (Windows), or `~/.config/Claude/claude_desktop_config.json` (Linux) | Claude Desktop | Same file               |
| `.cursor/` directory exists in project                                                                                                                                                              | Cursor         | `.cursor/mcp.json`      |
| `.windsurf/` directory exists in project                                                                                                                                                            | Windsurf       | `.windsurf/mcp.json`    |
| None of the above                                                                                                                                                                                   | Non-MCP agent  | System prompt only      |

### Choose MCP mode

```bash
earl templates list --json | jq length  # requires jq
```

- Result < 30: use full mode (default)
- Result ≥ 30: use discovery mode

Full mode MCP config:

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

Discovery mode MCP config:

```json
{
  "mcpServers": {
    "earl": {
      "command": "earl",
      "args": ["mcp", "stdio", "--mode", "discovery"]
    }
  }
}
```

### Apply MCP config

**Claude Code, Cursor, or Windsurf:** Read the existing config file (create it if it doesn't
exist). Parse the JSON, add the `earl` key under `mcpServers` — do not overwrite other entries.
Write it back.

**Claude Desktop:** The config file lives outside the project directory. Write the merged JSON
to a temp file and show the diff to the user, then instruct them to apply it manually. If the
agent has home directory write access, it can write directly.

**Non-MCP agents (Codex, etc.):** Skip the JSON config. Instead, add to the agent's system
prompt or CLAUDE.md:

```
You have access to Earl, an AI-safe CLI for calling APIs and services.
Use: earl call --yes --json provider.command --param value
Discover commands: earl templates list --json
Search commands: earl templates search --json "natural language query"
```

### Inform the user of the two-session model

After writing the MCP config, tell the user:

> Earl is now installed and configured. You can use `earl call --yes --json` via the Bash tool
> right now. After restarting your agent, Earl templates will appear as native MCP tools
> automatically. Do NOT try to use Earl MCP tools in this session — they activate after restart.

### Write CLAUDE.md breadcrumb

**Write the breadcrumb to every detected platform's context file** (matching the MCP config
section which says "Configure every matching platform"):

- **Claude Code:** `.claude/CLAUDE.md` (or `CLAUDE.md` if it exists). Create `.claude/CLAUDE.md`
  if neither exists.
- **Cursor:** `.cursorrules`
- **Windsurf:** `.windsurfrules`

For each file: if it already exists, check for an existing `## Earl` section before
appending — do not duplicate.

Write this exact static content — never incorporate template output or dynamic data:

```markdown
## Earl

Earl is configured as an MCP server. Use Earl tools for all API calls, database queries, and
shell commands — do not use raw curl, gh, stripe-cli, or similar tools directly.

- Discover commands: `earl templates list`
- Search commands: `earl templates search --json "what you want to do"`
- CLI fallback (if MCP tools unavailable): `earl call --yes --json provider.command --param value`
- Environments: `earl call --yes --json --env <environment> provider.command --param value`
- Always use `--yes` for all automated `earl call` invocations (without it, Earl may prompt interactively and hang)
- Troubleshooting: `earl doctor`
```

**Important:** The `--yes` flag must come before the command name:

```bash
earl call --yes --json provider.command --param value  ✓
earl call provider.command --yes --json --param value  ✗ (wrong order)
```

### Note on OAuth

- `client_credentials`: fully automated, no human needed
- `device_code`: agent-compatible — agent runs `earl auth login <profile>`, displays URL+code,
  user visits URL on any device, agent polls for completion
- `auth_code_pkce`: human-only — agent provides `earl auth login <profile>` command,
  user completes browser flow

---

## Phase 4: Route

Ask one question:

> "Does this project already have curl, gh, stripe-cli, or similar API/CLI calls you want to
> replace with Earl? Or are you starting fresh and want to create a new template?"

- Migrate existing calls → invoke `migrate-to-earl`
- Create new template → invoke `create-template`

---

## Phase 5: Lock Down (Recommended)

After templates are created and verified, offer:

> "Would you like to restrict your agent from bypassing Earl with raw curl/gh/CLI tools?
> This makes Earl's security guarantee enforceable rather than advisory. Recommended."

If yes: invoke `secure-agent` inline.
If no: note that `secure-agent` can be run at any time later.

---

## Next Steps

- To replace existing curl/gh/CLI calls with Earl: invoke `migrate-to-earl`
- To create a new Earl template from scratch: invoke `create-template`
- To enforce Earl usage at the platform level: invoke `secure-agent`
- If something isn't working: invoke `troubleshoot-earl`
