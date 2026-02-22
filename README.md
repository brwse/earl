# Earl

[![CI](https://github.com/brwse/earl/actions/workflows/ci.yml/badge.svg)](https://github.com/brwse/earl/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/earl)](https://crates.io/crates/earl)
[![Docs](https://img.shields.io/badge/docs-brwse.github.io%2Fearl-blue)](https://brwse.github.io/earl/)
[![License: MIT](https://img.shields.io/crates/l/earl)](LICENSE)

[![HTTP](https://img.shields.io/badge/HTTP-005CDE?logo=curl&logoColor=fff)](#)
[![GraphQL](https://img.shields.io/badge/GraphQL-E10098?logo=graphql&logoColor=fff)](#)
[![gRPC](https://img.shields.io/badge/gRPC-244C5A?logo=google&logoColor=fff)](#)
[![Bash](https://img.shields.io/badge/Bash-4EAA25?logo=gnubash&logoColor=fff)](#)
[![SQL](https://img.shields.io/badge/SQL-%23316192.svg?logo=postgresql&logoColor=white)](#)
&ensp;
[![macOS](https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white)](https://github.com/brwse/earl/releases/latest)
[![Linux](https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black)](https://github.com/brwse/earl/releases/latest)
[![Windows](https://img.shields.io/badge/Windows-0078D4?logo=windows&logoColor=white)](https://github.com/brwse/earl/releases/latest)

AI-safe CLI for AI agents. Earl sits between your agent and external
services, ensuring secrets stay in the OS keychain, requests follow reviewed
templates, and outbound traffic obeys egress rules.

## Why

AI agents with shell or network access can read secrets in plaintext and make
arbitrary API calls. Earl eliminates that risk:

- Agents run `earl call provider.command --param value` instead of raw `curl`
- Secrets are stored in the OS keychain and injected at request time
- Every request is defined by an HCL template that can be reviewed ahead of time
- Outbound traffic is restricted via `[[network.allow]]` egress rules
- Private IPs are blocked to prevent SSRF
- Bash and SQL execution runs in a sandbox

## Install

```bash
cargo install earl
```

Or use the installer scripts:

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/brwse/earl/main/scripts/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/brwse/earl/main/scripts/install.ps1 | iex
```

## Quick start

Prompt your coding agent:

```
Fetch https://raw.githubusercontent.com/brwse/earl/main/skills/getting-started-with-earl/SKILL.md
and any files it references under
https://raw.githubusercontent.com/brwse/earl/main/skills/getting-started-with-earl/references/
then follow the skill to help me get started with Earl.
```

Or manually:

```bash
# Import a template
earl templates import ./examples/bash/system.hcl

# Call a command defined in the template
earl call system.disk_usage --path /tmp
```

Templates are HCL files that define commands, parameters, and protocol
operations:

```hcl
version = 1
provider = "system"

command "disk_usage" {
  title       = "Check disk usage"
  summary     = "Reports disk usage for a given path"
  description = "Runs du -sh in a sandboxed bash environment."

  param "path" {
    type     = "string"
    required = true
  }

  operation {
    protocol = "bash"

    bash {
      script = "du -sh {{ args.path }}"
      sandbox {
        network = false
      }
    }
  }
}
```

## Documentation

Full docs at [brwse.github.io/earl/docs](https://brwse.github.io/earl/docs/):

[Quick Start](https://brwse.github.io/earl/docs/quick-start) ·
[Security Model](https://brwse.github.io/earl/docs/security) ·
[Templates](https://brwse.github.io/earl/docs/templates) ·
[Configuration](https://brwse.github.io/earl/docs/configuration) ·
[MCP Integration](https://brwse.github.io/earl/docs/mcp) ·
[CLI Reference](https://brwse.github.io/earl/docs/commands)

## License

[MIT](LICENSE)
