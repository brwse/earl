# CLI to Earl Mapping Reference

This file maps common CLI tools and URL patterns to Earl pre-built template providers.
Use this during the `migrate-to-earl` scan phase to identify which providers to import.

## Pre-built Provider Mapping

**Note:** All `curl.*` patterns should also be run with `wget` substituted for `curl` — codebases
may use either tool for the same API calls.

| Grep pattern | Provider | Import command |
|---|---|---|
| `curl.*api\.github\.com` / `gh ` / `hub ` | `github` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/github.hcl` |
| `curl.*api\.stripe\.com` / `stripe ` | `stripe` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/stripe.hcl` |
| `curl.*slack\.com/api` / `slack ` | `slack` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/slack.hcl` |
| `curl.*api\.notion\.com` / `notion ` | `notion` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/notion.hcl` |
| `curl.*api\.openai\.com` / `openai ` | `openai` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/openai.hcl` |
| `curl.*api\.anthropic\.com` | `anthropic` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/anthropic.hcl` |
| `curl.*discord\.com/api` | `discord` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/discord.hcl` |
| `curl.*gitlab\.com/api` / `gitlab ` | `gitlab` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/gitlab.hcl` |
| `curl.*atlassian\.(com\|net)` / `jira ` | `jira` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/jira.hcl` |
| `curl.*api\.linear\.app` / `linear ` | `linear` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/linear.hcl` |
| `curl.*api\.pagerduty\.com` | `pagerduty` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/pagerduty.hcl` |
| `curl.*api\.twilio\.com` / `twilio ` | `twilio` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/twilio.hcl` |
| `curl.*api\.sendgrid\.com` | `sendgrid` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/sendgrid.hcl` |
| `curl.*api\.cloudflare\.com` | `cloudflare` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/cloudflare.hcl` |
| `curl.*api\.vercel\.com` / `vercel ` | `vercel` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/vercel.hcl` |
| `curl.*api\.render\.com` | `render` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/render.hcl` |
| `curl.*shopify\.com/admin` | `shopify` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/shopify.hcl` |
| `curl.*api\.hub(api\|spot)\.com` | `hubspot` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/hubspot.hcl` |
| `curl.*api\.mailchimp\.com` | `mailchimp` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/mailchimp.hcl` |
| `curl.*api\.datadoghq\.com` / `datadog ` | `datadog` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/datadog.hcl` |
| `curl.*sentry\.io/api` | `sentry` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/sentry.hcl` |
| `curl.*api\.airtable\.com` | `airtable` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/airtable.hcl` |
| `curl.*api\.resend\.com` | `resend` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/resend.hcl` |
| `curl.*auth0\.com` | `auth0` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/auth0.hcl` |
| `curl.*supabase\.(co\|com)` | `supabase` | `earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/supabase.hcl` |

## No Pre-built Template

For these patterns, no pre-built template exists. Use `create-template` to author a custom one:

| Pattern | Protocol to use |
|---|---|
| `psql ` / `mysql ` / `sqlite3 ` | `sql` |
| `grpcurl ` | `grpc` |
| Any other `curl` / `wget` URL | `http` |
| Shell scripts | `bash` |

## Files to Scan

Grep across these file patterns:
- Shell scripts: `*.sh`, `*.bash`, `*.zsh`
- CI/CD: `.github/workflows/*.yml`, `.gitlab-ci.yml`, `Jenkinsfile`, `*.yaml`
- Docker: `Dockerfile`, `docker-compose*.yml`, `compose*.yaml`
- Build tools: `Makefile`, `*.mk`
- Source code: `*.py`, `*.js`, `*.ts`, `*.rb`, `*.go`, `*.rs`
- Agent instructions: `CLAUDE.md`, `.cursorrules`, `.github/copilot-instructions.md`

## Complex Pipeline Flag

Multi-pipe commands should be flagged, not rewritten:
```bash
# Flag this for manual review:
curl https://api.github.com/repos/... | jq '.items[]' | xargs -I{} ...

# Add comment and move on:
# TODO: migrate to earl (complex pipeline — review manually)
curl https://api.github.com/repos/... | jq '.items[]' | xargs -I{} ...
```
