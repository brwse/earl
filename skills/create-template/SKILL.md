---
name: create-template
description: Creates a new Earl HCL template for a specific API, database, or shell command. Use when adding a new service to Earl's template library, or when a pre-built template doesn't cover a needed command.
---

# Create Template

Creates an Earl HCL template file for a specific service and protocol. Each template defines
the commands, parameters, authentication, and protocol shape for one provider.

## Process

1. **Discover** — understand what service and command to build
2. **Infer protocol** — map the user's description to one of Earl's 5 protocols
3. **Load reference** — read the protocol reference for HCL shapes and patterns
4. **Write template** — create the HCL file
5. **Review** — show the user the complete template before running it
6. **Validate** — run `earl templates validate`
7. **Secrets** — print checklist for the human to set secrets
8. **Verify** — run a test `earl call`

---

## Phase 1: Discover Intent

If the request doesn't name a provider, command, and protocol, ask one question:

> "What service do you want to call, and what should the command do? For example:
> 'Call the GitHub API to create an issue' or 'Query my PostgreSQL database for user records'."

### Check for pre-built templates first

Earl ships with 25 ready-made provider templates. If the user names a known service, check
whether it is already imported before offering to import it:

```bash
earl templates list
```

Check the list carefully:

- If the **specific command** needed is already present (e.g. `github.create_issue` appears in
  the list), skip the import and go directly to Phase 7 to set any missing secrets.
- If the **provider** is imported but the specific command is **not** in the list (e.g. `github`
  commands appear but not `github.create_issue`), skip the import and proceed to custom template
  authoring (phases 2–6) to add the missing command to the existing file.
- If the provider is **not imported at all**, offer to import the pre-built template:

```bash
# Available: github, stripe, slack, notion, openai, anthropic, discord, gitlab, jira, linear,
#            pagerduty, twilio, sendgrid, cloudflare, vercel, render, shopify, hubspot,
#            mailchimp, datadog, sentry, airtable, auth0, supabase, resend
earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/<provider>.hcl
```

If a pre-built template was imported, skip to **Phase 7: Set Secrets** — phases 2–6 are not
needed. Then continue to Phase 8 to verify the template works.

Only proceed to custom template authoring (phases 2–6) if no pre-built template covers the
needed command.

---

## Phase 2: Infer Protocol

Map the user's description to a protocol:

| User mentions                                | Protocol  | Reference file                                                                                                                         |
| -------------------------------------------- | --------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| REST, HTTP, API, endpoint, JSON API, webhook | `http`    | `../references/http-templates.md` ([raw](https://raw.githubusercontent.com/brwse/earl/main/skills/references/http-templates.md))       |
| GraphQL, query/mutation (in API context)     | `graphql` | `../references/graphql-templates.md` ([raw](https://raw.githubusercontent.com/brwse/earl/main/skills/references/graphql-templates.md)) |
| gRPC, protobuf, service mesh                 | `grpc`    | `../references/grpc-templates.md` ([raw](https://raw.githubusercontent.com/brwse/earl/main/skills/references/grpc-templates.md))       |
| shell, bash, CLI, script, command line       | `bash`    | `../references/bash-templates.md` ([raw](https://raw.githubusercontent.com/brwse/earl/main/skills/references/bash-templates.md))       |
| SQL, database, postgres, mysql, sqlite       | `sql`     | `../references/sql-templates.md` ([raw](https://raw.githubusercontent.com/brwse/earl/main/skills/references/sql-templates.md))         |

If the answer is genuinely ambiguous, ask one follow-up question.

### SSRF Warning

If the user mentions `localhost`, `127.0.0.1`, `0.0.0.0`, or any private IP range (10.x, 172.16-31.x,
192.168.x), warn immediately:

> Earl blocks requests to private and loopback IP addresses (SSRF protection). This cannot be
> bypassed. Use a publicly accessible URL, or use the `bash` protocol to call local services.

---

## Phase 3: Load Reference

Read the reference file for the chosen protocol before writing any HCL. The reference file
contains the complete template shape, required fields, auth patterns, and known gotchas.

**Critical rule for all protocols:** HCL parses before Jinja renders. All `{{ }}` expressions
must be inside valid HCL string values.

```hcl
# WRONG — invalid HCL:
params = [{{ args.limit }}]

# CORRECT — Jinja expression inside a string, rendered to a number at call time:
params = ["{{ args.limit }}"]
```

---

## Phase 4: Draft Template

**Do NOT write the file to disk yet.** Compose the template content in memory — it will be
written to disk only after Phase 5 human review and approval.

**Target path** (determine now, write after approval):

- Local (project-specific): `./templates/<provider>.hcl`
- Global (all projects): `~/.config/earl/templates/<provider>.hcl` (macOS/Linux) or
  `%APPDATA%\earl\templates\<provider>.hcl` (Windows)

Default to local if the current directory is a project (contains `.git/`, `package.json`,
`Cargo.toml`, or similar). Default to global otherwise.

**Provider naming:** lowercase letters and underscores only. No hyphens, dots, or uppercase.
Examples: `github`, `my_company_api`, `internal_db`.

**If the file already exists:** Read it first. Add the new `command` block to the existing
file rather than overwriting it.

**Environments (optional):** If the user needs staging/production separation, add an
`environments` block at the provider level. Environment variables are available as `vars.*`
in all template expressions. See the [template schema docs](https://earl.dev/docs/template-schema#environments)
for full syntax. Only add environments when the user explicitly needs them — most templates
don't.

**Template structure:**

```hcl
version = 1
provider = "<provider_name>"

command "<command_name>" {
  title       = "<Short title, shown in tool listings>"
  summary     = "<One-line summary>"
  description = <<-EOT
    <Full description of what this command does.>

    Parameters:
    - param_name: description

    ## Guidance for AI agents
    Use this command to <explain when to use it>.
    Example: `earl call --yes --json <provider>.<command> --param_name value`
  EOT

  annotations {
    mode    = "<read|write>"
    secrets = ["<provider>.<secret_key>"]
  }

  param "<param_name>" {
    type        = "<string|number|boolean>"
    description = "<What this parameter controls>"
    required    = <true|false>
    default     = "<default_value>"   # omit if required = true
  }

  operation {
    protocol = "<http|graphql|grpc|bash|sql>"
    # ... protocol-specific fields from the reference file
  }

  result {
    output = "{{ operation.response }}"
  }
}
```

**Required for every template:**

- `annotations.mode`: `"read"` if the command reads data, `"write"` if it creates/modifies/deletes
- `annotations.secrets`: list all secret keys the template needs (format: `"provider.key_name"`)
- `description` must include a `## Guidance for AI agents` section

---

## Phase 5: Human Review (Required)

Show the user the complete template content before writing the file:

> "Here is the template I've drafted. Please review it before I write it to disk:
>
> [show full template content]
>
> Does this look correct? Should I write it and run `earl templates validate`?"

Do not write the file or proceed until the user explicitly approves. Once written to disk,
the template is immediately callable — there is no staging step. Approval here is the only
gate before it becomes live.

---

## Phase 6: Validate

```bash
earl templates validate
```

Fix any errors reported and re-validate. Common errors:

| Error                                  | Cause                                     | Fix                                      |
| -------------------------------------- | ----------------------------------------- | ---------------------------------------- |
| `HCL parse error` / `unexpected token` | Invalid HCL syntax                        | Check structure and quotes               |
| `template root must be an object`      | Missing version/provider fields           | Add `version = 1` and `provider = "..."` |
| `undefined variable` in Jinja          | `{{ args.x }}` doesn't match a param name | Check param names match references       |
| `params = [{{ ... }}]` syntax error    | Bare Jinja in HCL array                   | Wrap in string: `["{{ ... }}"]`          |

---

## Phase 7: Set Secrets

Check `annotations.secrets` in the template file for required secret keys. For pre-built
imports, read the imported file at `~/.config/earl/templates/<provider>.hcl` (macOS/Linux)
or `%APPDATA%\earl\templates\<provider>.hcl` (Windows) to find them.
Print a checklist:

```
Template ready. Set the required secrets in your terminal:

  earl secrets set <provider>.<key>

(Repeat for each secret listed above)

Tell me when you're done and I'll verify they're set.
```

**On macOS:** Warn the user that the first `earl secrets set` run may show a system dialog asking
to allow Earl keychain access — click "Always Allow" to avoid repeated prompts.

After the user confirms, verify:

```bash
earl secrets list
```

Check that all required keys appear. If any are missing, re-print just the missing ones.

---

## Phase 8: Verify

Run a test call with representative parameters:

```bash
earl call --yes --json <provider>.<command> --<param> <test_value>
```

**Important:** If `annotations.mode = "write"`, the test call will create/modify/delete real
data. If the template defines environments (check the `environments` block for valid names),
use `--env <name>` to select a non-production environment for the test call. Otherwise, use a
test or sandbox account, a safe test value (e.g. a dedicated test repo), or choose a read-only
command for the initial verification. Warn the user before running write-mode test calls.

If the call fails:

- HTTP 401/403 → secret not set or wrong key name
- `no such command` → template not loaded, check `earl templates list`
- Any other error → invoke `troubleshoot-earl`

---

## Next Steps

- To add another command to this template: invoke `create-template` again for the same provider
- To replace existing CLI calls with Earl: invoke `migrate-to-earl`
- To enforce Earl usage at the platform level: invoke `secure-agent`
- If something isn't working: invoke `troubleshoot-earl`
