# Rule data

## `excess-vocabulary.json`

This file preserves all 900 records published by Kobak et al. The scanner
indexes the 407 entries whose source annotation is exactly `style`. It retains
the content, mixed, and other annotations so that the evidence dataset is
complete and future analysis can distinguish topical shifts from style shifts.

These words are weak review signals. They do not establish AI authorship, and
they never make `--strict` fail.

## `rules.json`

The curated catalogue contains fixed phrases and regular expressions. Each
rule records:

- a stable ID;
- a severity;
- one or more patterns;
- applicable writing profiles;
- a diagnostic message;
- a suggested editing action.

`high` means the pattern is likely to damage precision, evidence, or technical
clarity. `medium` means it needs context. The empirical vocabulary layer is
always `low`.

The scanner compiles fixed phrases into one Aho–Corasick automaton and regular
expressions into a `RegexSet`. Regexes that cannot match the document are not
run again for positions.
