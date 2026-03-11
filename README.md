<p align="center">
  <img src="site/public/social-preview.jpg" alt="Earl - AI-safe CLI for AI agents" width="100%" />
</p>

[![CI](https://github.com/mathematic-inc/earl/actions/workflows/ci.yml/badge.svg)](https://github.com/mathematic-inc/earl/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/earl)](https://crates.io/crates/earl)
[![docs.rs](https://img.shields.io/docsrs/earl)](https://docs.rs/earl)
[![License: Apache-2.0](https://img.shields.io/crates/l/earl)](LICENSE)

[![HTTP](https://img.shields.io/badge/HTTP-005CDE?logo=curl&logoColor=fff)](#)
[![GraphQL](https://img.shields.io/badge/GraphQL-E10098?logo=graphql&logoColor=fff)](#)
[![gRPC](https://img.shields.io/badge/gRPC-244C5A?logo=google&logoColor=fff)](#)
[![Bash](https://img.shields.io/badge/Bash-4EAA25?logo=gnubash&logoColor=fff)](#)
[![SQL](https://img.shields.io/badge/SQL-%23316192.svg?logo=postgresql&logoColor=white)](#)
&ensp;
[![macOS](https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white)](https://github.com/mathematic-inc/earl/releases/latest)
[![Linux](https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black)](https://github.com/mathematic-inc/earl/releases/latest)
[![Windows](https://img.shields.io/badge/Windows-0078D4?logo=windows&logoColor=white)](https://github.com/mathematic-inc/earl/releases/latest)

Earl sits between agents and external services. Operations are HCL files committed to your repository. The LLM sees a tool name and description; it never reads the template body. An injected instruction in an API response has nowhere to land because the LLM isn't reading the part of the request that executes.

Secrets stay in the OS keychain. They aren't in tool arguments, tool descriptions, or output.

**[→ Documentation](https://mathematic-inc.github.io/earl)**

## Quick start

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/mathematic-inc/earl/main/scripts/install.sh | bash

# Import a provider template
earl templates import https://raw.githubusercontent.com/mathematic-inc/earl/main/examples/github.hcl

# Store a secret — prompts for the value, not echoed
earl secrets set github.token

# Call a command
earl call --yes --json github.search_repos --query "language:rust stars:>100"
```

To use Earl as MCP tools in your agent, add it to your MCP config and restart. Claude Code and Cursor use the same format:

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

MCP tools don't activate until after restart. In the current session, use `earl call --yes --json` through the Bash tool.

See [Quick Start](https://mathematic-inc.github.io/earl/docs/quick-start) for the full walkthrough, or [Agent-Assisted Setup](https://mathematic-inc.github.io/earl/docs/agent-assisted-setup) to let an agent handle the install and configuration.

## How it works

You write an HCL template describing an operation: method, URL, auth, parameters. When an agent calls the tool, Earl loads the template, reads the required secret from the OS keychain, renders the Jinja expressions against the agent's supplied values, and executes the request. The LLM only ever provided parameter values. Every other part of the request — the URL, the auth header, the method — was written by a human and committed to the repo.

See [How Earl Works](https://mathematic-inc.github.io/earl/docs/how-earl-works) for the full security model.

## Documentation

- [Introduction](https://mathematic-inc.github.io/earl/docs) — why Earl exists, how the security model works
- [Quick Start](https://mathematic-inc.github.io/earl/docs/quick-start) — install, first call, MCP config in five steps
- [Writing Templates](https://mathematic-inc.github.io/earl/docs/templates) — HTTP, GraphQL, gRPC, Bash, SQL; auth; result formatting
- [Template Schema](https://mathematic-inc.github.io/earl/docs/template-schema) — field-by-field reference
- [Secrets & Auth](https://mathematic-inc.github.io/earl/docs/secrets-and-auth) — OS keychain storage, OAuth2 flows
- [External Secrets](https://mathematic-inc.github.io/earl/docs/external-secrets) — 1Password, Vault, AWS, GCP, Azure
- [MCP Integration](https://mathematic-inc.github.io/earl/docs/mcp) — stdio and HTTP transport, full vs. discovery mode
- [Policy Engine](https://mathematic-inc.github.io/earl/docs/policy-engine) — JWT auth and access control for HTTP deployments
- [Environments](https://mathematic-inc.github.io/earl/docs/environments) — production, staging, and per-environment overrides
- [Hardening](https://mathematic-inc.github.io/earl/docs/hardening) — SSRF protection, egress allowlist, production checklist
- [Commands](https://mathematic-inc.github.io/earl/docs/commands) — complete CLI reference
- [Troubleshooting](https://mathematic-inc.github.io/earl/docs/troubleshooting) — keychain errors, template validation, MCP issues

## License

Apache-2.0

> This project is free and open-source work by a 501(c)(3) non-profit. If you find it useful, please consider [donating](https://github.com/sponsors/mathematic-inc).
