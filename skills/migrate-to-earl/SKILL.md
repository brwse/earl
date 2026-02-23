---
name: migrate-to-earl
description: Scans a codebase for raw API/CLI calls (curl, gh, stripe-cli, psql, grpcurl, etc.) and replaces them with Earl templates — one provider at a time. Use when migrating a project from direct CLI tool usage to Earl, or when replacing raw HTTP calls with reviewed templates.
---

# Migrate to Earl

Converts existing raw CLI/API calls in a codebase to Earl templates. Works one provider at a
time to keep changes reviewable and reversible.

## Scope Constraints (Read First)

These rules prevent context window exhaustion and partial-migration messes:

1. **One provider per run.** If multiple providers are found, ask which to migrate first.
2. **Cap at 10 call sites.** If more than 10 are found, ask which 10 to prioritize. Flag the
   rest with `# TODO: migrate to earl` comments and note them in the final report.
3. **Commit checkpoint.** Pause after template creation, before rewriting call sites. Let the
   user commit the template before the rewrite begins.
4. **Flag complex pipelines.** Multi-pipe commands (`curl | jq | xargs`) are not rewritten.
   Add `# TODO: migrate to earl (complex pipeline — review manually)` and move on.

---

## Phase 1: Scan All Providers

Read `references/cli-to-earl-mapping.md` (or fetch from `https://raw.githubusercontent.com/brwse/earl/main/skills/migrate-to-earl/references/cli-to-earl-mapping.md`) for the full list of patterns and files to scan.

Grep across all relevant files for every pattern in the mapping table. Even on repeated
invocations, always do a full scan first.

Present a summary before asking the user to pick a provider:

> "Found API/CLI calls to:
> - GitHub (12 sites in 4 files)
> - Stripe (8 sites in 2 files)
> - Slack (3 sites in 1 file)
>
> Which provider would you like to migrate first?"

If only one provider is found, proceed with it directly.

Note the remaining providers in the report so the user knows what's left for follow-up runs.

---

## Phase 2: Import Pre-built or Create Custom

Check if a pre-built template exists for the chosen provider (see `references/cli-to-earl-mapping.md`).

**If pre-built exists:**

Check `earl templates list` first — if the provider is already imported, skip this step.

```bash
earl templates import https://raw.githubusercontent.com/brwse/earl/main/examples/<provider>.hcl
earl templates list
```

Show the user which commands are available in the imported template.

**If no pre-built exists:**

Invoke `create-template` for the provider. The create-template skill includes its own
human review, validation, and secrets steps — do not duplicate those steps here.

---

## Phase 3: Map Call Sites

For each found call site, determine the corresponding Earl command:

```bash
earl templates list --json
```

Match each raw call to an Earl command:
- `curl -X GET https://api.github.com/repos/...` → `earl call github.get_repo`
- `gh issue create ...` → `earl call github.create_issue`
- `stripe customers list` → `earl call stripe.list_customers`

For calls that don't map to any existing command, invoke `create-template` to add it.

---

## Phase 4: Set Secrets

**Skip this phase** if Phase 2 invoked `create-template` — that skill already ran secrets setup
(its own Phase 7 and Phase 8) and confirmed secrets are set. Proceed to Phase 5.

Otherwise, print the checklist of required secrets for the imported templates:

```
Before replacing call sites, set the required secrets in your terminal:

  earl secrets set <provider>.<key>

Tell me when you're done and I'll verify before we proceed.
```

**On macOS:** First run of `earl secrets set` may show a system keychain access dialog.
Click "Always Allow" to avoid repeated prompts.

After the user confirms, verify:

```bash
earl secrets list
```

Check that all required keys appear. Re-print missing ones if needed.

---

## Phase 5: Verify Earl Commands Work

Run a test call for each mapped command before touching any call sites:

```bash
earl call --yes --json <provider>.<command> --<param> <representative_value>
```

If any call fails, resolve it (via `troubleshoot-earl` if needed) before proceeding.

---

## Phase 6: Checkpoint — Confirm Before Rewriting

Show the user:
1. The list of call sites that will be rewritten
2. The Earl equivalents they'll be replaced with
3. The call sites flagged for manual review (complex pipelines)

Ask explicitly:

> "I'm about to rewrite these X call sites. This will modify your source files.
> Should I proceed? (You should commit your current changes first if you haven't.)"

Do not rewrite anything until the user approves.

---

## Phase 7: Replace Call Sites

For each approved call site, rewrite to the Earl equivalent:

```bash
# Before:
curl -H "Authorization: Bearer $GITHUB_TOKEN" https://api.github.com/repos/owner/repo

# After:
earl call --yes --json github.get_repo --owner owner --repo repo
```

For flagged complex pipelines, add a comment but leave the original:
```bash
# TODO: migrate to earl (complex pipeline — review manually)
curl ... | jq ... | xargs ...
```

---

## Phase 8: Final Validation

```bash
earl templates validate
```

---

## Phase 9: Report

Summarize what was done:
- Which provider was migrated
- How many call sites were rewritten
- Which were flagged for manual review
- Which secrets were set
- What providers remain for follow-up runs

---

## Next Steps

- To migrate another provider: invoke `migrate-to-earl` again
- To create a template for an unmatched service: invoke `create-template`
- To enforce Earl usage at the platform level: invoke `secure-agent`
- If a migrated call fails: invoke `troubleshoot-earl`
