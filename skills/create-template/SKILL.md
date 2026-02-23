---
name: create-template
description: Creates a new Earl HCL template (or adds a command to an existing one). Use when someone asks to create, add, generate, or build an Earl template for any protocol (HTTP, GraphQL, gRPC, Bash, SQL).
---

# Create Earl Template

Earl templates are HCL files that define commands for calling APIs, databases, and shell scripts. This skill walks through creating or extending one.

## Step 1: Discover Intent

If the request already names a provider, command, and protocol, skip this step.

Otherwise ask ONE question:

> "What do you want to build? Include the service/provider name and what the command should do — for example: 'a GitHub command that lists open issues for a repo' or 'a Postgres command that queries recent orders'."

## Step 2: Infer Protocol

Map the user's description to a protocol:

| User mentions                                                  | Protocol  | Reference file                                          |
| -------------------------------------------------------------- | --------- | ------------------------------------------------------- |
| REST, HTTP, API, endpoint, webhook, JSON API                   | `http`    | [references/http.md](references/http.md)                |
| GraphQL, query/mutation (in API context)                       | `graphql` | [references/graphql.md](references/graphql.md)          |
| gRPC, protobuf, service mesh                                   | `grpc`    | [references/grpc.md](references/grpc.md)                |
| shell, bash, CLI, script, command line                         | `bash`    | [references/bash.md](references/bash.md)                |
| SQL, database, postgres, mysql, sqlite, query (in DB context)  | `sql`     | [references/sql.md](references/sql.md)                  |

Only ask a follow-up if the protocol is genuinely ambiguous.

## Step 3: Read the Reference File

Read the reference file for the chosen protocol (path shown in table above). It contains the HCL skeleton, key fields, and common patterns you must follow.

For auth/secrets patterns also read [references/secrets-and-auth.md](references/secrets-and-auth.md).

## Step 4: Determine Target File

- **Local templates:** `./templates/<provider>.hcl` — available in the current project only
- **Global templates:** `~/.config/earl/templates/<provider>.hcl` — available everywhere

Default to local unless the user asks for global.

If the file already exists, read it first and add the new command block — do not overwrite existing commands.

## Step 5: Write the Template

Apply ALL of the following constraints:

### Schema fields (all commands must have)

- `version = 1` at file top
- `provider = "<name>"` at file top
- `command "<name>" { ... }` block with:
  - `title` — short human-readable name
  - `summary` — one sentence
  - `description` — markdown; see Description Rules below
  - `annotations { mode = "read" | "write" }` — `write` for any mutating operation
  - `param` blocks for each input (`type`, `required`, `default` if optional)
  - `operation { ... }` — protocol-specific shape (see reference file)
  - `result { decode = "..." output = "..." }`

### Description rules

Every description must include:

1. A plain explanation of what the command does (1-2 sentences)
2. A `## Guidance for AI agents` section with:
   - When to use this command
   - A concrete `earl call` usage example: `earl call <provider>.<command> --param value`

### Mode selection

- `read` — fetching, querying, listing (no side effects)
- `write` — creating, updating, deleting, sending, inserting

### Auth / secrets (least-privilege)

- Only request secrets that the command actually needs
- For secret references in `auth` blocks use the dotted key (e.g. `github.token`)
- For secret references in template expressions use underscores (e.g. `{{ secrets.github_token }}`)
- Declare required secrets in `annotations { secrets = ["..."] }`
- See [references/secrets-and-auth.md](references/secrets-and-auth.md) for patterns

### HCL/Jinja critical rule

HCL is parsed BEFORE Jinja rendering. All `{{ }}` expressions must be inside valid HCL string values. For SQL params this means:

```hcl
params = ["{{ args.limit }}"]   # correct
params = [{{ args.limit }}]     # WRONG — invalid HCL
```

## Step 6: Validate

After writing the file, run:

```bash
earl templates validate
```

If validation fails, read the error, fix the template, and run again. Common errors:

| Error message                         | Likely cause                                                  |
| ------------------------------------- | ------------------------------------------------------------- |
| "unexpected token"                    | HCL syntax error — check braces, quotes, block structure      |
| "undefined variable"                  | `{{ args.x }}` param name doesn't match a `param` block name  |
| "missing required field"              | Protocol-specific required field omitted (see reference file) |
| "expected discriminator field `kind`" | `body` or `auth` block missing `kind = "..."` field           |

## Step 7: Report

After successful validation, output:

1. The file path written
2. The `earl call` command to run it
3. Any secrets the user needs to set: `earl secrets set <key>`
4. Any assumptions made about the API or behavior
