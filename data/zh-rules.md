# Chinese rule coverage

Source: [lieflat-less-ai-tone](https://github.com/larashero3-dotcom/lieflat-less-ai-tone/tree/27d29232f10124db904ca9c0536d0b67cb3b2833), MIT.
All 11 current SKILL rule families have review candidates, implemented as 17 `zh.*` rule IDs.
All profiles enable them by default. All are medium severity, visible without `--all`.
No Chinese-only finding fails `--strict`. No text is automatically edited.

## Mapping

1. Reversal: `zh.reversal`, phrase patterns for invented misconceptions and contrasts.
2. Enumeration: `zh.enumeration`, three or more Chinese-list-separator items.
3. Template sentences/case stacking: `zh.sentence-template`, adjacent sentences with the same comma count, colon/parenthesis presence and 15-character length bucket. Requires Chinese, a comma, and >10 characters. These thresholds are engineering heuristics adapted from the upstream script.
4. Em dash: `zh.em-dash`, every Chinese doubled em dash.
5. Preamble + colon: `zh.colon-preamble`; an announcement followed by a Markdown list: `zh.list-announcement`.
6. Numbered headings: `zh.numbered-heading`, runs of at least three Chinese numeral headings, introduced by Markdown hashes or bold markers. Body prose can intervene; an unnumbered heading breaks the run. Plain prose enumerators are not counted as headings.
7. Idealized occupational metaphors: `zh.idealized-persona`, broad profession-metaphor candidates including legitimate uses.
8. Vague summary before details: `zh.vague-summary`, flags the generalizing phrase without claiming that a numeric detail actually follows.
9. Nominalization: `zh.nominalization`.
10. Stock openers: `zh.stock-opener`.
11. Translation-like syntax: `zh.long-modifier`, `zh.when-clause`, `zh.topic-shell`, `zh.transition`, `zh.restatement`.

`zh.zero-subject` additionally adapts the research document's zero-subject paragraph-start observation: a non-first Chinese prose paragraph starts with a comment marker and its first sentence lacks a simple reference marker. Headings, blockquotes and lists do not count as prose paragraphs for this check.

## Interpretation

This is deliberately high-recall review. Legitimate technical contrasts, ordered procedures,
precise lists, causal links, quotations and uncertainty can be flagged and should be preserved
when needed. Ordinary quoted prose remains visible to lexical rules. Markdown code and URLs
remain masked, and findings cannot cross an excluded span. English rules are unchanged.

Sentence fingerprints cannot judge whether examples are actually redundant. Generalization
markers cannot establish whether evidence is missing, and restatement markers cannot tell
repetition from a new deduction. The agent must assess those questions in context. Nothing
in this implementation measures an AI-authorship probability or guarantees complete recall.

The upstream corpus is unpublished. Its reported rates are not used as calibrated scores.
Withdrawn rules (forced sentence-length variation, mandatory first person, generic passive
voice bans and forced synonym substitution) are not activated. Long-modifier thresholds are
heuristics, not statistically validated cutoffs. Upstream script/document inconsistencies
are resolved in favor of the documented heading and paragraph intent, rather than copied.
