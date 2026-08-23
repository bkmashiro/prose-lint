# Agent installation guide

This document is written for coding agents that have been asked to install
Prose Lint. Complete the installation and verification rather than returning
these commands to the user as a plan.

Repository: <https://github.com/bkmashiro/prose-lint>

## Required result

Install both components:

1. the `prose-lint` CLI in the user's normal executable path;
2. `skills/prose-lint/SKILL.md` in the current agent's native skill location.

Do not place build artifacts in the user's current project. Do not request or
handle API keys. Respect the host agent's normal approval rules for network,
package-manager, and toolchain changes.

## 1. Inspect the environment

Check, do not guess:

```bash
command -v prose-lint || true
command -v cargo || true
command -v rustc || true
printf 'HOME=%s\n' "$HOME"
```

If `cargo` is absent, install a stable Rust toolchain through the environment's
normal package mechanism. On an ordinary user-owned Unix machine with no
managed toolchain, the minimal official rustup profile is suitable:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
```

Do not replace a managed Rust installation or silently change a team machine's
system packages. If policy blocks installing Rust, report that exact blocker
instead of pretending installation succeeded.

## 2. Install the CLI

Use Cargo's git installer so no persistent source checkout is required:

```bash
cargo install --locked --git https://github.com/bkmashiro/prose-lint.git prose-lint
```

If an older Prose Lint installation already exists, update it with:

```bash
cargo install --locked --force --git https://github.com/bkmashiro/prose-lint.git prose-lint
```

Resolve the installed executable and verify it:

```bash
command -v prose-lint
prose-lint --version
```

If Cargo installed into `$HOME/.cargo/bin` but that directory is not on `PATH`,
add it using the shell's normal user-level profile rather than moving the
binary into a system directory.

## 3. Install the agent skill

The skill is an [Agent Skills](https://agentskills.io/) `SKILL.md` directory.
Prefer the host's native installer or documented skill root.

### Hermes Agent

Hermes can install the file directly:

```bash
hermes skills install https://raw.githubusercontent.com/bkmashiro/prose-lint/main/skills/prose-lint/SKILL.md
hermes skills list
```

The skill becomes available to new sessions. In an existing interactive
session, use `/reload-skills` when appropriate.

### Codex

For a user-level Codex installation, install under the personal Agent Skills
root:

```bash
mkdir -p "$HOME/.agents/skills/prose-lint"
curl -fsSL https://raw.githubusercontent.com/bkmashiro/prose-lint/main/skills/prose-lint/SKILL.md \
  -o "$HOME/.agents/skills/prose-lint/SKILL.md"
```

For a repository-only installation, use
`.agents/skills/prose-lint/SKILL.md` at the repository root instead. Codex
scans repository `.agents/skills` directories.

### Claude Code

For a personal Claude Code installation:

```bash
mkdir -p "$HOME/.claude/skills/prose-lint"
curl -fsSL https://raw.githubusercontent.com/bkmashiro/prose-lint/main/skills/prose-lint/SKILL.md \
  -o "$HOME/.claude/skills/prose-lint/SKILL.md"
```

For a project-only installation, use
`.claude/skills/prose-lint/SKILL.md` at the project root.

### Other agents

If the host declares a skill directory or installer, use it. Otherwise install
`SKILL.md` under the Agent Skills-compatible user root
`$HOME/.agents/skills/prose-lint/`. Do not invent a path when the host has an
explicit convention.

Before replacing an existing `prose-lint` skill, inspect it. Update it when it
is this repository's skill; do not overwrite an unrelated user-authored skill
with the same name. Verify the installed file starts with:

```yaml
---
name: prose-lint
```

## 4. Run the smoke test

Use a temporary file outside the user's project:

```bash
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/prose-lint-smoke.XXXXXX")"
tmp_file="$tmp_dir/smoke.md"
printf '%s\n' 'It is important to note that this change is not just fast, but transformative.' > "$tmp_file"

set +e
prose-lint scan "$tmp_file" --strict
status=$?
set -e
rm -f "$tmp_file"
rmdir "$tmp_dir"

test "$status" -eq 1
```

Success means:

- the output contains at least one `High` finding;
- strict mode exits with status `1`;
- the command does not fail to load its embedded rules.

An exit status of `1` is expected for this deliberately bad fixture. Any other
status is an installation failure that must be diagnosed.

## 5. Report completion

Return concrete evidence:

```text
CLI path: ...
CLI version: ...
Skill path or Hermes skill ID: ...
Smoke test: passed (High finding observed; strict exit 1)
```

Do not report success after only cloning the repository or installing one of
the two components.

## Update

Update the CLI with the `cargo install --force` command above, then refresh the
skill through the same native installer or destination used during install.
Rerun `prose-lint --version` and the smoke test.

## Uninstall

```bash
cargo uninstall prose-lint
```

Then uninstall the skill through the host's skill manager or remove only the
`prose-lint` skill directory created during installation. For Hermes:

```bash
hermes skills uninstall prose-lint
```
