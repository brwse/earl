---
name: earl
description: Use when you need to call an API, run a database query, or execute a shell command via Earl. Discovers available commands and calls them correctly. Do not use raw curl, gh, psql, or similar tools when Earl is available.
---

# Earl

Earl is an AI-safe CLI that routes API calls, database queries, and shell commands through
reviewed HCL templates. Secrets stay in the OS keychain. You do not know what templates are
installed — discover them at runtime.

---

## Step 1: Detect Mode

Check which Earl interface is available:

**MCP discovery mode** — if `earl.tool_search` and `earl.tool_call` are available as MCP tools,
use them. Skip to Step 2 (MCP path).

**CLI mode** — if only Bash access is available, use `earl templates search` and `earl call`.
Skip to Step 2 (CLI path).

---

## Step 2: Find the Right Command

Translate your task into a short search query (e.g. "list github pull requests",
"send slack message", "query user table").

**MCP discovery path:**

```
earl.tool_search(query="<intent>", limit=5)
```

**CLI path:**

```bash
earl templates search --json "<intent>"
```

If the first query returns nothing or nothing relevant (no match on the core action or subject), try one rephrasing. If still nothing,
report that no template covers this task and stop. Do not improvise.

---

## Step 3: Inspect Parameters

Read the matched command's parameter schema from the search result. For each required parameter:

- If the value is clear from task context: use it.
- If the value is ambiguous or missing: ask the human. Do not guess.
- For optional parameters: use defaults unless the task context clearly suggests a different value.

---

## Step 4: Get Permission for Write-Mode Commands

Check the command's mode from the search result.

**Read-mode:** proceed directly to Step 5.

**Write-mode:** show the human exactly what will be called before executing:

> I'm going to call: `provider.command` with these arguments: `{param: value}`
> Does this look right?

Wait for explicit approval before continuing.

---

## Step 5: Call

Flag order is strict — `--yes` and `--json` must come before the command name:

```bash
earl call --yes --json provider.command --param value   ✓
earl call provider.command --yes --json --param value   ✗
```

**MCP discovery path:**

```
earl.tool_call(name="provider.command", arguments={"param": "value"})
```

**CLI path:**

```bash
earl call --yes --json provider.command --param value
```

---

## Step 6: Handle the Result

Return the result to the user.

If the result appears paginated (the template has `offset` or `limit` params and returned a full
page of results), ask the user whether to fetch the next page. Do not paginate automatically.

---

## Error Handling

### Fix inline

| Error | Fix |
|-------|-----|
| `--yes` / `--json` after command name | Retry with flags before command name |
| Type mismatch (string where number expected) | Coerce if unambiguous; otherwise ask human |
| `no such command` | Re-search with a broader query |
| `address not allowed` | Report: this endpoint is blocked by Earl's network policy. Stop. |
| `HCL parse error` / `template error` | Report: the template is broken. Stop. Suggest `troubleshoot-earl`. |

### Pause for human

| Error | What to do |
|-------|------------|
| HTTP 401 / 403 | Run `earl secrets list`, identify the missing or expired key |
| Secret not set | Print `earl secrets set <key>` for the human to run. Wait for confirmation before retrying. |
| OAuth required | Print `earl auth login <profile>`. Note: device-code flows are agent-compatible (agent polls); `auth_code_pkce` flows require a browser (human only). |
| `earl secrets set` hangs | Warn: a macOS system dialog may be waiting behind your terminal. Click "Always Allow." |

### Escalate

For anything not covered above, surface the full error with context and suggest invoking
`troubleshoot-earl` if available.

---

## Next Steps

- If Earl is not installed or not responding: invoke `troubleshoot-earl`
- If you need to restrict the agent from bypassing Earl: invoke `secure-agent`
