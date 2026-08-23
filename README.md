# Prose Lint ✨🚀

[简体中文](README.zh-CN.md) · **English**

> In today's rapidly evolving AI landscape, where every word carries the
> potential to unlock transformative value, Prose Lint stands as a pivotal,
> robust, and groundbreaking testament to the power of meticulous prose
> orchestration. It doesn't merely scan text—it seamlessly empowers you to
> delve deeper, navigate the intricate tapestry of technical communication,
> and elevate every sentence into a holistic, nuanced, game-changing
> expression of clarity. Importantly, Prose Lint is not merely a linter, but a
> paradigm-shifting gateway to unprecedented linguistic excellence. Together,
> we are not just linting prose; we are redefining what it means to write. 🌐💡

Yes, the paragraph above is a test fixture wearing a README costume. Prose Lint
flags it accordingly.

A fast, deterministic linter for formulaic LLM-shaped English prose, with a
separate skill for evidence-safe revision.

Prose Lint reports editing leads. It does not determine whether a person or a
model wrote a document.

## 📋 One-click copy for your agent to install

Copy the block below into Codex, Claude Code, Hermes, Cursor, or another coding
agent. GitHub puts a copy button in the top-right corner of the block.

```text
Install Prose Lint from https://github.com/bkmashiro/prose-lint for me. Read https://github.com/bkmashiro/prose-lint/blob/main/AGENT_INSTALL.md first, then follow it completely: install the CLI, install the prose-lint skill using this agent's native skill mechanism, and run the documented verification smoke test. Reuse an existing Rust toolchain if present; if a prerequisite is missing, install it through the normal safe mechanism for this environment. Do not stop after cloning or describing commands. Finish with the exact CLI path, skill path, version output, and smoke-test result. Do not modify my current project except for a project-local skill when that is the native convention.
```

For a manual install, continue to [Install](#install). Agents should use
[`AGENT_INSTALL.md`](AGENT_INSTALL.md), which includes platform detection,
verification, update, and uninstall steps.

## Why another prose linter?

Most “AI word” lists make two mistakes: they treat every occurrence as an
error, and they mix writing quality with authorship guesses. Prose Lint keeps
those questions separate:

- curated high-confidence rules target wording that can hide evidence, inflate
  claims, or make technical prose less precise;
- contextual rules identify stock Codex engineering metaphors and repeated
  abstractions without banning valid terms;
- 407 empirically observed style words are available as low-confidence review
  signals;
- all 900 records from the source study remain in the repository, including
  content words that the scanner deliberately does not activate.

Low-confidence vocabulary never makes strict mode fail.

## Performance design

The scanner does not run every expression over every document.

1. Fixed words and phrases are compiled into one case-insensitive
   Aho–Corasick automaton.
2. Regular expressions are compiled as a `RegexSet`; only expressions known to
   match are revisited to obtain positions.
3. Contextual clusters use paragraph-local aggregation rather than combinatorial
   expressions.
4. Markdown masking and newline indexing each happen once per file.
5. Directories are scanned in parallel with Rayon. The compiled scanners are
   shared between workers.

The release binary has six direct runtime dependencies: `aho-corasick`, `glob`,
`regex`, `rayon`, `serde`, and `serde_json`. It does not use a parser framework,
async runtime, network client, or NLP model.

## Install

```bash
git clone https://github.com/bkmashiro/prose-lint.git
cd prose-lint
cargo install --path .
```

The executable contains the rule data, so scans do not need a network
connection or a separate data directory.

## Use

```bash
# Scan one file with the default technical profile
prose-lint scan README.md

# Scan a repository; build and dependency directories are skipped
prose-lint scan docs/

# Scan only top-level Typst files. Quotes make expansion shell-independent.
prose-lint scan '*.typ'

# Scan Typst files recursively
prose-lint scan '**/*.typ'

# Show every low-confidence empirical vocabulary hit
prose-lint scan paper.md --profile academic --all

# JSON for editor or agent integration
prose-lint scan docs/ --format json

# Fail only when a high-confidence finding is present
prose-lint scan docs/ --strict

# Limit file-level parallelism
prose-lint scan docs/ --jobs 4
```

Supported extensions are `.md`, `.mdx`, `.txt`, `.rst`, `.adoc`, `.tex`, and
`.typ`. Fenced code, inline code, and URLs are masked while preserving source
offsets. Positional paths accept `*`, `?`, character classes such as `[ab]`, and
recursive `**` glob patterns. An unmatched pattern is an error instead of a
silent empty scan.

Profiles:

```text
technical  academic  pr  commit  casual  marketing
```

## Confidence levels

### High

Review each result. Current high-confidence categories include:

- empty importance markers;
- ornamental contrast templates;
- stock implication sentences;
- rhetorical inflation;
- unsupported design rationale;
- evidential verbs that may strengthen the original claim;
- meta-writing;
- vague Codex adjectives such as `honest shape` or `clean boundary`;
- chatbot residue.

### Medium

These patterns need context:

- formal signposting and participial tails;
- abstract noun stacks;
- technical artifacts said to “live”, “own”, or “carry” something;
- changes said to “land”;
- dense `boundary / surface / contract / posture` clusters;
- promotional phrasing and high em-dash density.

A valid technical term should remain when it names the correct concept.

### Low

The empirical layer comes from the vocabulary analysis by Kobak et al. It was
measured in biomedical abstracts, so domain transfer is uncertain. Prose Lint
hides individual low-confidence hits by default and reports only their count.
Use `--all` to inspect them.

## Agent skill

`skills/prose-lint/SKILL.md` tells an agent how to run the scanner, interpret
confidence levels, preserve claims and uncertainty, and verify a rewrite.

For self-installing agents, [`AGENT_INSTALL.md`](AGENT_INSTALL.md) gives the
complete procedure: environment detection, CLI installation, native skill
locations for Hermes, Codex, and Claude Code, a required smoke test, updates,
and uninstall steps. The one-click prompt at the top of this README directs an
agent to that guide.

The skill stays short because the executable loads the full catalogue. Agent
context contains only findings that matched the current document.

## Rule data

- `data/rules.json`: curated phrases, expressions, profiles, messages, and
  editing actions.
- `data/excess-vocabulary.json`: 900 research records, 407 of which are active
  low-confidence style signals.
- `THIRD_PARTY_NOTICES.md`: provenance, citation, and source licence.

The curated rules are data rather than Rust code. Adding a phrase normally
requires no engine change.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

Generate a repeatable local corpus for throughput checks:

```bash
python3 scripts/generate_benchmark_corpus.py /tmp/prose-lint-bench
/usr/bin/time -lp target/release/prose-lint scan /tmp/prose-lint-bench >/dev/null
```

The latest checked local smoke result and its environment are recorded in
[`BENCHMARKS.md`](BENCHMARKS.md).

## Limitations

- The scanner currently targets English prose.
- It uses deterministic surface and structural rules, not semantic inference.
- A clean report does not prove that prose is natural, correct, or human-written.
- A finding does not require a rewrite. Context and document purpose remain
  decisive.

## Licence

Project code and curated rules are MIT licensed. The converted research dataset
retains its upstream MIT notice; see `THIRD_PARTY_NOTICES.md`.
