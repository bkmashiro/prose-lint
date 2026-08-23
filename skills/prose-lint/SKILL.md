---
name: prose-lint
description: Use when scanning or revising English technical prose for formulaic LLM/Codex wording. Run the deterministic linter, prioritize high-confidence findings, and preserve claims, evidence, terminology, citations, numbers, and uncertainty.
---

# Prose Lint

Use the scanner as an editing aid, not an authorship detector.

## Scan

Run the installed binary on the relevant files:

```bash
prose-lint scan PATH --profile technical
```

Use `--profile academic`, `pr`, `commit`, `casual`, or `marketing` when the
genre is known. Add `--all` only when a broad vocabulary review is useful.
For machine-readable output, use `--format json`.

Paths may be files, directories, or glob patterns. Quote patterns so behavior
does not depend on shell expansion:

```bash
prose-lint scan '*.typ'
prose-lint scan '**/*.typ'
```

The first scans top-level Typst files; the second scans them recursively. An
unmatched pattern is an error, so do not report success without reading the
command's exit status.

## Interpret findings

Apply findings in this order:

1. **High:** inspect every finding. These rules target unsupported rationale,
   semantic inflation, empty importance markers, stock implications, and other
   patterns that often damage technical precision.
2. **Medium:** edit only when the wording is vague, repetitive, or ornamental
   in context. Keep canonical domain terminology.
3. **Low:** treat empirical excess vocabulary as a weak lead. A word's presence
   is not a reason to replace it, and it is not evidence of AI authorship.

A dense abstraction cluster can be useful evidence of formulaic prose, but its
individual terms may still be correct. Prefer removing ornamental uses over
rotating synonyms.

## Rewrite constraints

- Preserve factual claims, numbers, citations, quotations, identifiers, and
  mathematical notation.
- Preserve uncertainty and scope. Do not turn an observation into a result or
  a proposal into an implemented feature.
- Do not invent rationale, evidence, motivations, or author experience.
- Keep good sentences. Do not rewrite every line to make the scan quieter.
- Prefer concrete actors, operations, conditions, and trade-offs.
- Repeat canonical technical terms when consistency matters.

## Verify

After editing, rerun the same command. A clean scan is not required. Confirm
that remaining findings are intentional and that no edit changed the original
meaning.

Use `--strict` only as a repository quality gate. It fails on high-confidence
findings, never on low-confidence vocabulary alone.
