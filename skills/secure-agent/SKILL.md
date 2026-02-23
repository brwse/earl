---
name: secure-agent
description: Locks down an AI agent by configuring platform-level tool restrictions (deniedTools) and Earl network egress rules. Use after Earl is working and templates are created, to make Earl's security guarantee enforceable rather than advisory.
---

# Secure Agent

After Earl is set up and working, this skill restricts the agent's ability to bypass Earl
and make raw API/CLI calls directly. Without this step, CLAUDE.md is just a suggestion.

## What This Does

1. **`deniedTools`**: Blocks specific bash commands (curl, gh, stripe-cli, etc.) at the
   platform level for Claude Code. Agents cannot run these commands at all.
2. **Egress rules**: Restricts which URLs Earl templates can contact, preventing Earl itself
   from being used as an open proxy.

## Important Limitation

`deniedTools` pattern matching can be bypassed via alternative tools (`python3 -c "import
urllib..."`, `node -e "fetch(...)"`, etc.). This blocks accidental or habitual CLI use — the
common case. For stronger containment, pair with OS-level firewall rules or a network proxy.

## Platform Support

| Platform | Mechanism | Hard restriction? |
|---|---|---|
| **Claude Code** | `deniedTools` in `.claude/settings.json` | Yes — platform enforced |
| **Cursor** | `.cursor/mcp.json` or Cursor settings UI | Partial — check Cursor docs for per-tool restrictions |
| **Claude Desktop** | No bash access | N/A |
| **Non-MCP CLI agents** | CLAUDE.md instructions only | No — advisory only |

This skill primarily targets Claude Code. Instructions for other platforms are best-effort.

---

## Step 1: Check Coverage

Only deny tools for services that have Earl templates. Denying a tool for a service with no
Earl template would leave the agent unable to interact with that service at all.

```bash
earl templates list --json
```

Note which providers are covered. Map each to the CLI tools to deny:

| Earl template covers | Deny these tools |
|---|---|
| `github` | `Bash(gh *)`, `Bash(hub *)` |
| `stripe` | `Bash(stripe *)` |
| `slack` | `Bash(slack *)` |
| `openai` | `Bash(openai *)` |
| `vercel` | `Bash(vercel *)` |
| Any HTTP API template | `Bash(curl *)`, `Bash(wget *)`, `Bash(http *)`, `Bash(httpie *)` |
| Any SQL template | `Bash(psql *)`, `Bash(mysql *)`, `Bash(sqlite3 *)` |
| Any gRPC template | `Bash(grpcurl *)` |

**Do NOT deny `Bash` entirely.** Earl's bash protocol and legitimate shell operations still
need it.

---

## Step 2: Generate and Present Denylist

Based on the covered providers, generate the `deniedTools` array. Show it to the user before
applying anything:

> "Based on your Earl templates, I'd add these restrictions to `.claude/settings.json`:
>
> ```json
> {
>   "deniedTools": [
>     "Bash(curl *)",
>     "Bash(wget *)",
>     "Bash(gh *)",
>     "Bash(stripe *)"
>   ]
> }
> ```
>
> This blocks the listed tools for this agent in this project. Other projects are unaffected.
> Shall I apply this?"

Do not apply until the user explicitly approves.

---

## Step 3: Apply Denylist

For Claude Code: read `.claude/settings.json`, merge the `deniedTools` array (do not overwrite
other keys), write it back.

If `deniedTools` already exists, merge arrays — do not duplicate entries.

---

## Step 4: Verify

**For Claude Code:** Attempt to run a denied command:

```bash
curl https://example.com
```

Claude Code will refuse to run this command. The "tool denied" or "not allowed" error message
from Claude Code **is the success signal** — it means the denylist is active. You cannot
distinguish success from failure by looking at the exit code; look at whether Claude Code
blocked it before the shell ran it.

If Claude Code runs `curl` without blocking it, the `deniedTools` pattern syntax is wrong.
Check the format against current Claude Code documentation — the exact pattern syntax may vary
by version. Then re-apply with the corrected format.

**For other platforms:** Ask the user to attempt a denied command manually in their agent
session and confirm it is blocked.

Test that Earl still works:

```bash
earl call --yes --json system.list_files --path .
```

Expected: succeeds (Earl is not in the denylist).

---

## Step 5: Configure Egress Rules (Strongly Recommended)

Without egress rules, Earl is an open proxy — any HTTP template with a parameterized URL can
reach any public endpoint. Add `[[network.allow]]` rules to
`~/.config/earl/config.toml` to restrict which hosts Earl templates can contact.

For each provider template, add a rule:

```toml
[[network.allow]]
hosts = ["api.github.com"]

[[network.allow]]
hosts = ["api.stripe.com"]

[[network.allow]]
hosts = ["api.slack.com", "slack.com"]
```

After editing, verify:

```bash
earl doctor
```

Earl doctor checks that the config is valid. Any `[[network.allow]]` parse errors will be
reported.

---

## What This Does Not Cover

- **Bash templates**: The `bash` protocol runs user-defined scripts in Earl's sandbox. The
  denylist does not restrict what Earl's own bash protocol can do. Ensure bash templates
  explicitly set `sandbox.network = false` unless network access is required.
- **Agents without per-tool restriction**: For non-Claude-Code agents, the CLAUDE.md instruction
  is the only constraint. This is advisory, not enforced.
- **Alternative interpreters**: Python, Node, Ruby, and other interpreters are not blocked by
  a curl-specific denylist. Pair with OS-level firewall rules for stronger containment.

---

## Next Steps

- Earl is now the enforced channel for all covered API calls
- To add egress rules for a new provider: add `[[network.allow]]` blocks to
  `~/.config/earl/config.toml`
- If the agent is blocked from a call it needs: add an Earl template for that service, or
  remove the specific deny rule for that tool
- If something breaks: invoke `troubleshoot-earl`
