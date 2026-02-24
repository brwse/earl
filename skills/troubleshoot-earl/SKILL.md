---
name: troubleshoot-earl
description: Diagnoses and fixes Earl problems including installation failures, template errors, authentication issues, MCP connectivity, and SSRF blocks. Use when Earl isn't working, when earl call returns errors, when MCP tools are not visible, or when earl doctor reports failures.
---

# Troubleshoot Earl

Cold-start diagnostic for when Earl isn't working. Follow the steps in order — each step
may reveal the root cause.

## What Agents Can Fix

- Earl not installed
- HCL parse errors
- Jinja render errors
- Template not found
- Wrong CLI flag order
- MCP config missing or incorrect
- SSRF blocks (suggest workaround)

## What Requires Human Action

- Secret values — agent prints checklist; human runs `earl secrets set <key>` manually
- `auth_code_pkce` OAuth — agent provides command; human completes browser flow
- Agent restart for MCP — agent notes this; continues with CLI mode
- macOS keychain access dialog — agent warns; human clicks "Always Allow"

---

## Step 1: Earl Doctor

```bash
earl doctor
```

If this fails with `command not found: earl`, Earl is not installed. Install it:

```bash
cargo install earl  # requires Rust toolchain + Node.js + pnpm
# or: curl -fsSL https://raw.githubusercontent.com/brwse/earl/main/scripts/install.sh | bash
# Windows: irm https://raw.githubusercontent.com/brwse/earl/main/scripts/install.ps1 | iex
```

If `earl doctor` reports other failures, address each one before continuing.

---

## Step 2: Validate Templates

```bash
earl templates validate
```

Fix any errors found. Common errors and fixes:

| Error message                                                   | Cause                                     | Fix                                                    |
| --------------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------------ |
| `HCL parse error` / `unexpected token`                          | Invalid HCL syntax                        | Check structure, quotes, and braces                    |
| `template root must be an object`                               | Missing top-level fields                  | Add `version = 1` and `provider = "..."`               |
| `template must define either 'commands' or 'command', not both` | Used both singular and plural form        | Use only `command` blocks                              |
| `command X has empty title`                                     | Missing `title` field                     | Add `title = "..."` to the command block               |
| `undefined variable`                                            | `{{ args.x }}` doesn't match a param name | Check that param names match `{{ args.x }}` references |
| `params = [{{ ... }}]`                                          | Bare Jinja in HCL array                   | Wrap in string: `params = ["{{ ... }}"]`               |

---

## Step 3: Check Template Exists

```bash
earl templates list
```

If the expected `provider.command` is not listed:

- Check that the file is in `./templates/` or the global template directory:
  `~/.config/earl/templates/` (macOS/Linux) or `%APPDATA%\earl\templates\` (Windows)
- Check that `provider` in the HCL matches what you're calling (`earl call <provider>.<command>`)
- Re-run `earl templates validate` to catch parse errors

---

## Step 4: Check Secrets

```bash
earl secrets list
```

If required secrets are missing, print the checklist:

```
Run in your terminal:
  earl secrets set <provider>.<key>

Tell me when done and I'll verify.
```

**macOS note:** If `earl secrets set` hangs for more than 5 seconds, check for a macOS system
dialog behind your terminal window asking to allow Earl keychain access. Click "Always Allow."

After confirmation, re-run `earl secrets list` to verify keys are set.

---

## Step 5: Check MCP Config (if using MCP)

Check that the MCP config file exists and contains the `earl` entry:

**Claude Code:** `.claude/settings.json`

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

Note: `["mcp", "stdio", "--mode", "discovery"]` is also valid — it is used when the template
count is ≥ 30 (per `setup-earl` Phase 3). Do not remove the `--mode discovery` flag from a
config that already has it; that would silently downgrade to full mode.

**Claude Desktop:** `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS),
`%APPDATA%\Claude\claude_desktop_config.json` (Windows), or
`~/.config/Claude/claude_desktop_config.json` (Linux)

**Cursor:** `.cursor/mcp.json` (project directory)

**Windsurf:** `.windsurf/mcp.json` (project directory)

If the config is missing or incorrect, write it (merging with existing entries, not overwriting).

**MCP tools not visible?** The config is correct but MCP tools only activate after restarting
the agent. In the current session, use `earl call --yes --json` via Bash instead.

---

## Step 6: Run the Failing Command

If all above pass, run the failing command directly and examine the output:

```bash
earl call --yes --json <provider>.<command> --<param> <value>
```

Match the error to the table:

| Symptom                                            | Category                                  | Fix                                                                                                                                                                                                                  |
| -------------------------------------------------- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `HCL parse error` / `unexpected token`             | HCL syntax                                | Agent: edit template, re-validate                                                                                                                                                                                    |
| `undefined variable` / `template error`            | Jinja render                              | Agent: fix `{{ args.* }}` references                                                                                                                                                                                 |
| HTTP 401 / 403                                     | Auth failure                              | Human: `earl secrets set <key>`                                                                                                                                                                                      |
| `address not allowed`                              | SSRF block                                | Agent: use a public URL, or use `bash` protocol for local services                                                                                                                                                   |
| `command not found: earl`                          | Not installed                             | Agent: run install script                                                                                                                                                                                            |
| `no such command`                                  | Wrong command name                        | Agent: check `earl templates list`                                                                                                                                                                                   |
| MCP tool not visible in agent                      | MCP not configured or restart needed      | Agent: write/fix config; tell user to restart                                                                                                                                                                        |
| `earl call` hangs waiting for input                | `--yes` flag in wrong position            | Agent: reorder — `earl call --yes --json provider.command ...`                                                                                                                                                       |
| `earl secrets set` hangs                           | macOS keychain dialog                     | Human: click "Always Allow" in system dialog                                                                                                                                                                         |
| OAuth flow required                                | Browser-based auth                        | Human: run `earl auth login <profile>`, complete browser flow                                                                                                                                                        |
| `unknown environment` / `invalid environment name` | Bad `--env` value                         | Agent: check `environments` block in template for valid names                                                                                                                                                        |
| `vars.*` resolves to empty string                  | No active environment                     | Agent: pass `--env <name>` or set `[environments] default` in config.toml                                                                                                                                            |
| `environment protocol switching not allowed`       | Protocol mismatch in environment override | Agent: prefer `vars.*` for different base URLs; only add `allow_environment_protocol_switching = true` to `annotations` if a protocol change is genuinely required (bypasses HTTP egress rules — see `secure-agent`) |

---

## Next Steps

- If the issue was a missing template: invoke `create-template`
- If the issue was MCP config: config is now written; restart your agent for MCP tools to activate
- If the issue required browser OAuth: run `earl auth login <profile>`, then retry the command
- If Earl still isn't working after all steps: run `earl doctor --json` and share the output
