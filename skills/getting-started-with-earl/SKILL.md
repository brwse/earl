---
name: getting-started-with-earl
description: Guides developers through installing Earl, running a quick demo, and creating their first API template. Use when someone is new to Earl, wants to set up Earl, or needs help creating their first template.
---

# Getting Started with Earl

Earl is a CLI tool for calling APIs, databases, and shell commands through HCL template files. This skill guides a developer from zero to a working template.

## Process

1. **Install** — get Earl running
2. **Demo** — show a working example immediately
3. **Discover** — ask what the user wants to build
4. **Build** — create their first custom template
5. **Next steps** — contextual suggestions

## Phase 1: Install

Check if Earl is already installed:

```bash
earl --version
```

If not installed, detect the OS and install:

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/brwse/earl/main/scripts/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/brwse/earl/main/scripts/install.ps1 | iex
```

**Alternative (requires Rust toolchain):**
```bash
cargo install earl
```

After install, verify:

```bash
earl doctor
```

If `earl doctor` succeeds, move on silently. If it fails, diagnose the specific error before continuing.

## Phase 2: Quick Demo

Import a ready-made template and run it so the user sees Earl work before making any decisions:

```bash
earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/bash/system.hcl
```

Run it:

```bash
earl call system.list_files --path .
```

This lists files in the current directory. The user now has a mental model of how Earl works: templates define commands, `earl call` runs them.

Show what templates are available:

```bash
earl templates list
```

## Phase 3: Discover Intent

Ask the user ONE question:

> "What do you want to build with Earl? For example: call a REST API, query a database, run shell commands, call a GraphQL API, or call a gRPC service."

Infer the protocol from their answer:

| User mentions | Protocol | Reference file |
|---|---|---|
| REST, HTTP, API, endpoint, webhook, JSON API | HTTP | [http-templates.md](references/http-templates.md) |
| GraphQL, query/mutation (in API context) | GraphQL | [graphql-templates.md](references/graphql-templates.md) |
| gRPC, protobuf, service mesh | gRPC | [grpc-templates.md](references/grpc-templates.md) |
| shell, bash, CLI, script, command line | Bash | [bash-templates.md](references/bash-templates.md) |
| SQL, database, postgres, mysql, sqlite, query (in DB context) | SQL | [sql-templates.md](references/sql-templates.md) |

Only ask a follow-up if the answer is genuinely ambiguous. If the user says "I want to call the GitHub API," infer HTTP with bearer auth — do not ask "which protocol?"

### SSRF Warning

If the user mentions `localhost`, `127.0.0.1`, `10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`, or any private/loopback IP, warn them immediately:

> Earl blocks requests to private and loopback IP addresses for security (SSRF protection). This is hard-coded and cannot be bypassed. You will need to use a publicly accessible URL, or use the `bash` protocol to call local services via curl.

## Phase 4: Build First Template

1. Read the reference file for the chosen protocol
2. Walk the user through creating an HCL template file
3. Save to `./templates/<provider>.hcl` (local) or `~/.config/earl/templates/<provider>.hcl` (global)
4. If auth is needed, set up the secret: `earl secrets set <key>` (or `earl secrets set <key> --stdin` for piping)
5. Run the template: `earl call <provider>.<command> --param value`
6. Verify the template is listed: `earl templates list`

### On Failure

Categorize the error before suggesting a fix:

| Error type | Symptoms | Fix |
|---|---|---|
| HCL parse error | "expected ...", "unexpected token" | File structure or syntax issue. Check HCL syntax. |
| Jinja render error | "undefined variable", "template error" | Expression issue. Check `{{ args.* }}` references match param names. |
| Auth error | HTTP 401 or 403 | Secret not set or expired. Run `earl secrets set <key>`. |
| SSRF block | "address not allowed", connection refused to private IP | URL points to private/loopback IP. Use a public URL. |
| API error (4xx/5xx) | HTTP status in response body | Earl ran successfully. The API itself returned an error. Check URL, params, and API docs. |

After suggesting a fix, offer to re-run automatically.

### Important Rules

- **CLI flag order:** `--yes` and `--json` must come BEFORE the command args:
  ```bash
  earl call --yes --json provider.command --param value
  ```
- **Template directory:** Verify with `earl templates list` after creating a template
- **HCL parses before Jinja:** All `{{ }}` expressions must be inside valid HCL string values. Never use bare expressions outside of strings.

## Phase 5: What's Next

Suggest next steps based on what the user built. Do not give a generic list.

**Common suggestions:**

- **JSON output for scripting:** `earl call --json provider.command | jq .`
- **Shell completions:** `earl completion bash >> ~/.bashrc` (or zsh/fish equivalent)
- **Add more commands:** Add another `command` block to the same template file
- **MCP integration:** See [mcp-integration.md](references/mcp-integration.md) to expose templates as tools for Claude Desktop or Claude Code
- **Advanced auth:** See [secrets-and-auth.md](references/secrets-and-auth.md) for OAuth, API keys, and advanced auth flows
- **Template schema reference:** See [template-quick-ref.md](references/template-quick-ref.md) for all protocol shapes and field options
