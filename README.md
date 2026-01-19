# gh-pr-sync

sync github prs to local yaml files for code review and analysis.

## what it does

- fetches pr metadata via github cli
- writes each pr to `.prs/<number>-<slug>.yaml`
- includes changed files with per-file additions/deletions
- designed for loading pr context into claude for flexible analysis

## requirements

- [github cli](https://cli.github.com/) (`gh`) authenticated
- rust (for building)

## quick start

```bash
# build and install
cargo install --path .

# install claude skill
gh-pr-sync skill claude

# pull prs from current repo
gh-pr-sync pull

# pull from external repo
gh-pr-sync pull --repo owner/repo --limit 20

# read prs
cat .prs/*.yaml
```

## commands

### pull

```bash
# current repo, open prs
gh-pr-sync pull

# external repo
gh-pr-sync pull --repo anthropics/claude-code

# limit count
gh-pr-sync pull --limit 50

# include closed/merged
gh-pr-sync pull --all
```

options:

- `--repo`: target repository (owner/repo format)
- `--limit`: max prs to fetch (default: 100)
- `--all`: include closed and merged prs

### skill

```bash
# install for claude
gh-pr-sync skill claude
```

installs the skill for the specified AI assistant. currently supports `claude` (`~/.claude/skills/gh-pr-sync/`).

## output format

`.prs/42-fix-login-bug.yaml`:

```yaml
number: 42
title: Fix login bug
state: open
author: alice
head: fix-login
base: main
labels: [bug]
created_at: 2024-01-15T10:00:00Z
updated_at: 2024-01-16T12:00:00Z
additions: 50
deletions: 10
is_draft: false
files:
  - path: src/auth.rs
    additions: 40
    deletions: 5
  - path: src/main.rs
    additions: 10
    deletions: 5
body: |
  pr description here...
```

## use case

load prs into context, then ask claude about:

- low-hanging optimization opportunities in recently changed code
- architectural changes across prs
- areas with high churn
- code quality in recent changes
- similar patterns that could be applied elsewhere

## license

MIT OR Apache-2.0
