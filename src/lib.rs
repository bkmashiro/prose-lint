use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use regex::{Regex, RegexSet};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeSet, HashSet};
use std::fmt;

const RULES_JSON: &str = include_str!("../data/rules.json");
const VOCABULARY_JSON: &str = include_str!("../data/excess-vocabulary.json");
const ABSTRACTION_TERMS: &[&str] = &[
    "boundary",
    "surface",
    "contract",
    "posture",
    "mechanism",
    "authority",
    "invariant",
    "lifecycle",
];

#[derive(Debug)]
pub enum Error {
    Data(String),
    Render(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data(message) => write!(f, "rule data error: {message}"),
            Self::Render(error) => write!(f, "render error: {error}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Render(value)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    #[default]
    Technical,
    Academic,
    Pr,
    Commit,
    Casual,
    Marketing,
}

impl Profile {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "technical" => Some(Self::Technical),
            "academic" => Some(Self::Academic),
            "pr" => Some(Self::Pr),
            "commit" => Some(Self::Commit),
            "casual" => Some(Self::Casual),
            "marketing" => Some(Self::Marketing),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Technical => "technical",
            Self::Academic => "academic",
            Self::Pr => "pr",
            Self::Commit => "commit",
            Self::Casual => "casual",
            Self::Marketing => "marketing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Text,
    Json,
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub profile: Profile,
    pub show_all: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            profile: Profile::Technical,
            show_all: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub line: usize,
    pub column: usize,
    pub start: usize,
    pub end: usize,
    pub matched: String,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub path: String,
    pub profile: Profile,
    pub findings: Vec<Finding>,
    pub suppressed_low_confidence: usize,
    pub vocabulary_candidates: usize,
    pub active_style_vocabulary: usize,
}

impl Report {
    pub fn high_confidence_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::High)
            .count()
    }

    pub fn render(&self, format: Format) -> Result<String, Error> {
        match format {
            Format::Json => Ok(serde_json::to_string_pretty(self)?),
            Format::Text => {
                let mut output = String::new();
                output.push_str(&format!(
                    "{} [{}]: {} finding(s)",
                    self.path,
                    self.profile.name(),
                    self.findings.len()
                ));
                if self.suppressed_low_confidence > 0 {
                    output.push_str(&format!(
                        ", {} low-confidence signal(s) summarized",
                        self.suppressed_low_confidence
                    ));
                }
                output.push('\n');
                for finding in &self.findings {
                    output.push_str(&format!(
                        "  {:?} {}:{} {}\n    {}\n    matched: {:?}\n    suggestion: {}\n",
                        finding.severity,
                        finding.line,
                        finding.column,
                        finding.rule_id,
                        finding.message,
                        finding.matched,
                        finding.suggestion
                    ));
                }
                Ok(output)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct RuleDef {
    id: String,
    severity: Severity,
    kind: RuleKind,
    patterns: Vec<String>,
    message: String,
    suggestion: String,
    profiles: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum RuleKind {
    Literal,
    Regex,
}

#[derive(Debug, Deserialize)]
struct VocabularyData {
    entries: Vec<VocabularyEntry>,
}

#[derive(Debug, Deserialize)]
struct VocabularyEntry {
    word: String,
    class: String,
}

#[derive(Debug, Clone)]
enum LiteralTarget {
    Rule(usize),
    Vocabulary(String),
}

#[derive(Debug, Clone)]
struct LiteralMeta {
    target: LiteralTarget,
}

#[derive(Debug)]
pub struct Scanner {
    rules: Vec<RuleDef>,
    literal_automaton: AhoCorasick,
    literal_meta: Vec<LiteralMeta>,
    regex_set: RegexSet,
    regexes: Vec<Regex>,
    regex_rule_indices: Vec<usize>,
    abstraction_automaton: AhoCorasick,
    vocabulary_candidates: usize,
    active_style_vocabulary: usize,
}

impl Scanner {
    pub fn builtin() -> Result<Self, Error> {
        let rules: Vec<RuleDef> = serde_json::from_str(RULES_JSON)
            .map_err(|error| Error::Data(format!("cannot parse rules.json: {error}")))?;
        let vocabulary: VocabularyData = serde_json::from_str(VOCABULARY_JSON)
            .map_err(|error| Error::Data(format!("cannot parse vocabulary: {error}")))?;

        let mut literal_meta = Vec::new();
        let mut literal_patterns = Vec::new();
        let mut regex_patterns = Vec::new();
        let mut regex_rule_indices = Vec::new();

        for (rule_index, rule) in rules.iter().enumerate() {
            match rule.kind {
                RuleKind::Literal => {
                    for pattern in &rule.patterns {
                        literal_patterns.push(pattern.clone());
                        literal_meta.push(LiteralMeta {
                            target: LiteralTarget::Rule(rule_index),
                        });
                    }
                }
                RuleKind::Regex => {
                    for pattern in &rule.patterns {
                        regex_patterns.push(pattern.clone());
                        regex_rule_indices.push(rule_index);
                    }
                }
            }
        }

        let mut active_style_vocabulary = 0;
        for entry in &vocabulary.entries {
            if entry.class == "style" && entry.word.chars().all(|c| c.is_alphabetic() || c == '-') {
                active_style_vocabulary += 1;
                literal_patterns.push(entry.word.clone());
                literal_meta.push(LiteralMeta {
                    target: LiteralTarget::Vocabulary(entry.word.clone()),
                });
            }
        }

        let literal_automaton = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::Standard)
            .build(&literal_patterns)
            .map_err(|error| Error::Data(format!("cannot compile literals: {error}")))?;
        let regex_set = RegexSet::new(&regex_patterns)
            .map_err(|error| Error::Data(format!("cannot compile regex set: {error}")))?;
        let regexes = regex_patterns
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| Error::Data(format!("cannot compile regex: {error}")))?;
        let abstraction_automaton = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::Standard)
            .build(ABSTRACTION_TERMS)
            .map_err(|error| Error::Data(format!("cannot compile cluster terms: {error}")))?;

        Ok(Self {
            rules,
            literal_automaton,
            literal_meta,
            regex_set,
            regexes,
            regex_rule_indices,
            abstraction_automaton,
            vocabulary_candidates: vocabulary.entries.len(),
            active_style_vocabulary,
        })
    }

    pub fn scan_text(&self, path: &str, source: &str, options: &ScanOptions) -> Report {
        let masked = mask_markdown(source);
        let newlines = newline_offsets(source);
        let mut findings = Vec::new();
        let mut suppressed_low_confidence = 0;
        let mut seen = HashSet::new();

        for hit in self
            .literal_automaton
            .find_overlapping_iter(masked.as_bytes())
        {
            let meta = &self.literal_meta[hit.pattern().as_usize()];
            if !has_word_boundaries(&masked, hit.start(), hit.end()) {
                continue;
            }
            match &meta.target {
                LiteralTarget::Rule(rule_index) => {
                    let rule = &self.rules[*rule_index];
                    if !profile_enabled(rule, options.profile) {
                        continue;
                    }
                    push_finding(
                        &mut findings,
                        &mut seen,
                        source,
                        &newlines,
                        hit.start(),
                        hit.end(),
                        rule,
                    );
                }
                LiteralTarget::Vocabulary(word) => {
                    if options.show_all {
                        let (line, column) = line_column(source, &newlines, hit.start());
                        let key = (
                            "research.excess-vocabulary".to_owned(),
                            hit.start(),
                            hit.end(),
                        );
                        if seen.insert(key) {
                            findings.push(Finding {
                                rule_id: "research.excess-vocabulary".to_owned(),
                                severity: Severity::Low,
                                line,
                                column,
                                start: hit.start(),
                                end: hit.end(),
                                matched: source[hit.start()..hit.end()].to_owned(),
                                message: format!(
                                    "{word:?} is an empirically observed excess style word in post-LLM PubMed abstracts; this is a weak, domain-specific signal."
                                ),
                                suggestion: "Review only in context; do not replace mechanically.".to_owned(),
                            });
                        }
                    } else {
                        suppressed_low_confidence += 1;
                    }
                }
            }
        }

        let matching_regexes = self.regex_set.matches(&masked);
        for regex_index in matching_regexes.iter() {
            let rule_index = self.regex_rule_indices[regex_index];
            let rule = &self.rules[rule_index];
            if !profile_enabled(rule, options.profile) {
                continue;
            }
            for hit in self.regexes[regex_index].find_iter(&masked) {
                push_finding(
                    &mut findings,
                    &mut seen,
                    source,
                    &newlines,
                    hit.start(),
                    hit.end(),
                    rule,
                );
            }
        }

        self.scan_abstraction_clusters(
            source,
            &masked,
            &newlines,
            options,
            &mut findings,
            &mut seen,
        );
        scan_em_dash_density(
            source,
            &masked,
            &newlines,
            options,
            &mut findings,
            &mut seen,
        );

        findings.sort_by_key(|finding| {
            (
                Reverse(finding.severity),
                finding.start,
                finding.end,
                finding.rule_id.clone(),
            )
        });
        Report {
            path: path.to_owned(),
            profile: options.profile,
            findings,
            suppressed_low_confidence,
            vocabulary_candidates: self.vocabulary_candidates,
            active_style_vocabulary: self.active_style_vocabulary,
        }
    }

    fn scan_abstraction_clusters(
        &self,
        source: &str,
        masked: &str,
        newlines: &[usize],
        options: &ScanOptions,
        findings: &mut Vec<Finding>,
        seen: &mut HashSet<(String, usize, usize)>,
    ) {
        if options.profile == Profile::Casual || options.profile == Profile::Marketing {
            return;
        }
        for (start, end) in paragraph_spans(masked) {
            let paragraph = &masked[start..end];
            let mut terms = BTreeSet::new();
            for hit in self
                .abstraction_automaton
                .find_overlapping_iter(paragraph.as_bytes())
            {
                if has_word_boundaries(paragraph, hit.start(), hit.end()) {
                    terms.insert(ABSTRACTION_TERMS[hit.pattern().as_usize()]);
                }
            }
            if terms.len() >= 3 {
                let key = ("codex.abstraction-cluster".to_owned(), start, end);
                if seen.insert(key) {
                    let (line, column) = line_column(source, newlines, start);
                    findings.push(Finding {
                        rule_id: "codex.abstraction-cluster".to_owned(),
                        severity: Severity::Medium,
                        line,
                        column,
                        start,
                        end,
                        matched: source[start..end].to_owned(),
                        message: format!(
                            "Dense Codex-style abstraction cluster: {}.",
                            terms.into_iter().collect::<Vec<_>>().join(", ")
                        ),
                        suggestion: "Keep canonical technical terms, but replace ornamental abstractions with concrete actors and operations.".to_owned(),
                    });
                }
            }
        }
    }
}

fn profile_enabled(rule: &RuleDef, profile: Profile) -> bool {
    rule.profiles
        .iter()
        .any(|item| item == "all" || item == profile.name())
}

fn push_finding(
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<(String, usize, usize)>,
    source: &str,
    newlines: &[usize],
    start: usize,
    end: usize,
    rule: &RuleDef,
) {
    let key = (rule.id.clone(), start, end);
    if !seen.insert(key) {
        return;
    }
    let (line, column) = line_column(source, newlines, start);
    findings.push(Finding {
        rule_id: rule.id.clone(),
        severity: rule.severity,
        line,
        column,
        start,
        end,
        matched: source[start..end].to_owned(),
        message: rule.message.clone(),
        suggestion: rule.suggestion.clone(),
    });
}

fn scan_em_dash_density(
    source: &str,
    masked: &str,
    newlines: &[usize],
    options: &ScanOptions,
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<(String, usize, usize)>,
) {
    if options.profile == Profile::Casual {
        return;
    }
    let positions: Vec<_> = masked
        .match_indices('—')
        .map(|(offset, _)| offset)
        .collect();
    let words = masked.split_whitespace().count().max(1);
    if positions.len() >= 3 && positions.len() * 120 > words {
        let start = positions[0];
        let end = start + '—'.len_utf8();
        let key = ("technical.em-dash-density".to_owned(), start, end);
        if seen.insert(key) {
            let (line, column) = line_column(source, newlines, start);
            findings.push(Finding {
                rule_id: "technical.em-dash-density".to_owned(),
                severity: Severity::Medium,
                line,
                column,
                start,
                end,
                matched: source[start..end].to_owned(),
                message: format!("High em-dash density: {} in {} words.", positions.len(), words),
                suggestion: "Check whether commas, parentheses, full stops, or deletion better express each relation.".to_owned(),
            });
        }
    }
}

fn newline_offsets(source: &str) -> Vec<usize> {
    source
        .bytes()
        .enumerate()
        .filter_map(|(index, byte)| (byte == b'\n').then_some(index))
        .collect()
}

fn line_column(source: &str, newlines: &[usize], offset: usize) -> (usize, usize) {
    let line_index = newlines.partition_point(|newline| *newline < offset);
    let line_start = if line_index == 0 {
        0
    } else {
        newlines[line_index - 1] + 1
    };
    (
        line_index + 1,
        source[line_start..offset].chars().count() + 1,
    )
}

fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn has_word_boundaries(text: &str, start: usize, end: usize) -> bool {
    text[..start]
        .chars()
        .next_back()
        .is_none_or(|character| !is_word_char(character))
        && text[end..]
            .chars()
            .next()
            .is_none_or(|character| !is_word_char(character))
}

fn paragraph_spans(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\n' {
            let mut next = index + 1;
            while next < bytes.len()
                && (bytes[next] == b' ' || bytes[next] == b'\t' || bytes[next] == b'\r')
            {
                next += 1;
            }
            if next < bytes.len() && bytes[next] == b'\n' {
                if !text[start..index].trim().is_empty() {
                    spans.push((start, index));
                }
                start = next + 1;
                index = start;
                continue;
            }
        }
        index += 1;
    }
    if start < text.len() && !text[start..].trim().is_empty() {
        spans.push((start, text.len()));
    }
    spans
}

fn mask_markdown(source: &str) -> String {
    let mut masked = source.as_bytes().to_vec();
    let mut in_fence = false;
    let mut offset = 0;

    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if in_fence || fence {
            for byte in &mut masked[offset..offset + line.len()] {
                if *byte != b'\n' {
                    *byte = b' ';
                }
            }
        }
        if fence {
            in_fence = !in_fence;
        }
        offset += line.len();
    }

    let mut index = 0;
    while index < masked.len() {
        if masked[index] == b'`' {
            let start = index;
            index += 1;
            while index < masked.len() && masked[index] != b'`' && masked[index] != b'\n' {
                index += 1;
            }
            if index < masked.len() && masked[index] == b'`' {
                index += 1;
                masked[start..index].fill(b' ');
            }
            continue;
        }
        if !masked[index].is_ascii_whitespace()
            && (source.as_bytes()[index..].starts_with(b"http://")
                || source.as_bytes()[index..].starts_with(b"https://"))
        {
            let start = index;
            while index < masked.len() && !masked[index].is_ascii_whitespace() {
                index += 1;
            }
            masked[start..index].fill(b' ');
            continue;
        }
        index += 1;
    }

    String::from_utf8(masked).expect("masking valid UTF-8 with ASCII spaces remains valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_lookup_handles_first_and_later_lines() {
        let source = "éone\ntwo\nthree";
        let offsets = newline_offsets(source);
        assert_eq!(line_column(source, &offsets, 2), (1, 2));
        assert_eq!(line_column(source, &offsets, 7), (2, 2));
    }

    #[test]
    fn masking_preserves_byte_offsets() {
        let source = "é `delve https://example.com` — plain";
        let masked = mask_markdown(source);
        assert_eq!(source.len(), masked.len());
        assert_eq!(masked.find('—'), source.find('—'));
        assert!(!masked.contains("delve"));
        assert!(!masked.contains("example"));
    }
}
