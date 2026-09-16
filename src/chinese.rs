//! High-recall structural review leads. These heuristics do not infer semantics.
use regex::Regex;
use std::sync::OnceLock;

struct Patterns {
    heading: Regex,
    comment: Regex,
    reference: Regex,
}

fn patterns() -> &'static Patterns {
    static PATTERNS: OnceLock<Patterns> = OnceLock::new();
    PATTERNS.get_or_init(|| Patterns {
        heading: Regex::new(r"(?m)^[ \t]*(?:#{1,6}[ \t]+|\*\*)[ \t]*(?:第?[一二三四五六七八九十百]+)[、．.][^\n]+").unwrap(),
        comment: Regex::new(r"^(?:听起来|看起来|看上去|听上去|说白了|说到底|换句话说|意味着|值得注意的是|不难看出|问题在于|原因在于|结果是|有意思的是|更重要的是|关键在于|真正的)").unwrap(),
        reference: Regex::new(r"这|那|其|此|上面|前面|刚才|以上|该|它|他|她").unwrap(),
    })
}

pub(super) fn scan(text: &str) -> Vec<(&'static str, usize, usize)> {
    if !text.chars().any(is_han) {
        return Vec::new();
    }
    let patterns = patterns();
    let mut hits = Vec::new();
    // A non-numbered heading breaks a heading sequence; intervening body prose does not.
    let mut run = Vec::new();
    let flush = |run: &mut Vec<(usize, usize)>, hits: &mut Vec<_>| {
        if run.len() >= 3 {
            hits.extend(run.iter().map(|&(s, e)| ("zh.numbered-heading", s, e)));
        }
        run.clear();
    };
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if let Some(hit) = patterns.heading.find(line) {
            run.push((offset + hit.start(), offset + hit.end()));
        } else if trimmed.starts_with('#') || trimmed.starts_with("**") {
            flush(&mut run, &mut hits);
        }
        offset += line.len();
    }
    flush(&mut run, &mut hits);

    let mut prose_paragraphs = 0;
    for (start, end) in super::paragraph_spans(text) {
        let raw = &text[start..end];
        let paragraph = raw.trim();
        if paragraph.is_empty()
            || paragraph.starts_with(['#', '>', '|'])
            || paragraph.starts_with("**")
            || paragraph.starts_with("- ")
            || paragraph.starts_with("* ")
            || paragraph.starts_with("+ ")
            || paragraph.starts_with(|c: char| c.is_ascii_digit())
            || !paragraph.chars().any(is_han)
        {
            continue;
        }
        let base = start + raw.len() - raw.trim_start().len();
        let first = paragraph
            .split(['。', '！', '？', '\n'])
            .next()
            .unwrap_or("");
        if prose_paragraphs > 0
            && patterns.comment.is_match(first)
            && !patterns.reference.is_match(first)
        {
            hits.push(("zh.zero-subject", base, base + first.len()));
        }
        prose_paragraphs += 1;

        let mut previous = None;
        let mut position = base;
        for raw_sentence in paragraph.split_inclusive(['。', '！', '？']) {
            let sentence = raw_sentence.trim();
            let begin = position + raw_sentence.len() - raw_sentence.trim_start().len();
            let len = sentence.chars().count();
            let commas = sentence.matches('，').count();
            let signature = (
                commas,
                sentence.contains('：'),
                sentence.contains(['（', '(']),
                len / 15,
            );
            if len > 10 && commas > 0 && sentence.chars().any(is_han) {
                if let Some((old_signature, old_begin)) = previous
                    && old_signature == signature
                {
                    hits.push(("zh.sentence-template", old_begin, begin + sentence.len()));
                }
                previous = Some((signature, begin));
            } else {
                previous = None;
            }
            position += raw_sentence.len();
        }
    }
    hits
}

fn is_han(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{20000}'..='\u{3134f}')
}
