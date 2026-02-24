<p align="center">
  <img src="site/public/social-preview.jpg" alt="Earl - AI-safe CLI for AI agents" width="100%" />
</p>

An AI agent with shell access can read your `.env` file, call any URL, and exfiltrate credentials without leaving a trace. Earl gives agents exactly the access you've approved — nothing more.

[![CI](https://github.com/brwse/earl/actions/workflows/ci.yml/badge.svg)](https://github.com/brwse/earl/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/earl)](https://crates.io/crates/earl)
[![docs.rs](https://img.shields.io/docsrs/earl)](https://docs.rs/earl)
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

When you use Earl, agents stop calling raw `curl` and start calling `earl call github.search_repos`. Secrets live in the OS keychain and get injected at request time — they never appear in process arguments, environment variables, or output. Outbound traffic goes only to hosts you've explicitly allowed. Write operations ask for confirmation before they run. The HCL templates that define all of this are plain text files you can read, review, and check into your repo alongside your code. That last part matters: the security boundary is auditable, not implied.

## Fastest path

Prompt your coding agent:

```
Fetch https://raw.githubusercontent.com/brwse/earl/main/skills/setup-earl/SKILL.md
and any files it references under https://raw.githubusercontent.com/brwse/earl/main/skills/references/
then follow the skill to help me get started with Earl.
```

Your agent will install Earl, walk through setup, and build your first template from a single prompt.

Or install manually:

```bash
curl -fsSL https://raw.githubusercontent.com/brwse/earl/main/scripts/install.sh | bash
# or: cargo install earl
```

## What it looks like

A template for GitHub repository search. One param, one secret, one HTTP operation:

```hcl
version  = 1
provider = "github"

command "search_repos" {
  title   = "Search repositories"
  summary = "Search GitHub repositories by query"

  annotations {
    mode    = "read"
    secrets = ["github.token"]
  }

  param "query" {
    type     = "string"
    required = true
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.github.com/search/repositories"

    auth {
      kind   = "bearer"
      secret = "github.token"
    }

    query = {
      q    = "{{ args.query }}"
      sort = "stars"
    }
  }
}
```

Store your token and search:

```bash
earl secrets set github.token
earl call github.search_repos --query "language:rust stars:>1000"
```

Earl resolves the token from your keychain, builds the request, and returns the result. The token never touches the terminal.

## Why not a wrapper script

A shell wrapper around `curl` is fine until it isn't. You can't audit what a wrapper script will do before it runs. You can't prevent an agent from modifying it. And nothing stops it from leaking secrets into process arguments or log output.

Earl templates are reviewed artifacts, not executable scripts. SSRF protection is on by default and cannot be disabled from inside a template; private IP ranges are blocked in the binary. Secrets are resolved from the OS keychain at call time and never written anywhere. When you run `earl mcp`, agents see only the tools you've loaded, with full parameter schemas, and nothing else. The attack surface is small by construction, not by convention.

## Documentation

Full docs at [brwse.github.io/earl/docs](https://brwse.github.io/earl/docs/):

[Quick Start](https://brwse.github.io/earl/docs/quick-start) ·
[Templates](https://brwse.github.io/earl/docs/templates) ·
[How Earl Works](https://brwse.github.io/earl/docs/how-earl-works) ·
[MCP Server](https://brwse.github.io/earl/docs/mcp)

For team deployments: [External Secret Managers](https://brwse.github.io/earl/docs/external-secret-managers) · [Policy Engine](https://brwse.github.io/earl/docs/policy-engine) · [Hardening Guide](https://brwse.github.io/earl/docs/security)

## License

[MIT](LICENSE)
