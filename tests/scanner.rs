use prose_lint::{CustomTerm, Format, Profile, ScanOptions, Scanner, Severity};

#[test]
fn flags_high_confidence_technical_prose_patterns() {
    let scanner = Scanner::builtin().unwrap();
    let report = scanner.scan_text(
        "draft.md",
        "Importantly, this is not just a cache, but a paradigm shift. This ensures robust operation.",
        &ScanOptions::default(),
    );

    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id == "technical.empty-importance" && f.severity == Severity::High)
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id == "technical.contrast-template")
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id == "technical.stock-implication")
    );
}

#[test]
fn flags_standalone_deictic_corrections_without_banning_ordinary_not() {
    let scanner = Scanner::builtin().unwrap();
    let text = concat!(
        "This case study uses the shared-backing mechanism, not the early-source latency path.\n",
        "This is a host-owned path rather than a guest fallback.\n",
        "The parser accepts JSON, not YAML.\n",
        "This case uses byte offsets, not character offsets. The distinction matters.\n",
        "A lead-in makes this case use the shared mechanism, not the fallback.\n",
        "This case uses the shared mechanism,\nnot the fallback.\n",
    );
    let report = scanner.scan_text("contrast.md", text, &ScanOptions::default());
    let hits: Vec<_> = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == "codex.standalone-correction")
        .collect();

    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].line, 1);
    assert_eq!(hits[1].line, 2);
    assert!(
        hits.iter()
            .all(|finding| finding.severity == Severity::Medium)
    );
}

#[test]
fn masks_markdown_code_and_inline_code() {
    let scanner = Scanner::builtin().unwrap();
    let text = "Use `It is worth noting that` as a test value.\n\n```text\nImportantly, delve into it.\n```\n\nPlain prose delves into the issue.";
    let report = scanner.scan_text(
        "docs.md",
        text,
        &ScanOptions {
            show_all: true,
            ..Default::default()
        },
    );

    assert_eq!(
        report
            .findings
            .iter()
            .filter(|f| f.matched.to_lowercase().contains("delv"))
            .count(),
        1
    );
    assert!(!report.findings.iter().any(|f| f.matched == "Importantly"));
}

#[test]
fn regexes_do_not_bridge_masked_regions() {
    let scanner = Scanner::builtin().unwrap();
    for text in [
        "This is not just\n```text\nopaque code\n```\nbut a direct claim.",
        "This is not just https://example.com but a direct claim.",
    ] {
        let report = scanner.scan_text("masked.md", text, &ScanOptions::default());
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| finding.rule_id == "technical.contrast-template"),
            "unexpected cross-mask match in {text:?}"
        );
    }
}

#[test]
fn weak_vocabulary_is_summarized_unless_all_is_requested() {
    let scanner = Scanner::builtin().unwrap();
    let normal = scanner.scan_text(
        "draft.md",
        "The method accentuates the trend.",
        &ScanOptions::default(),
    );
    assert!(normal.findings.iter().all(|f| f.severity != Severity::Low));
    assert_eq!(normal.suppressed_low_confidence, 1);

    let all = scanner.scan_text(
        "draft.md",
        "The method accentuates the trend.",
        &ScanOptions {
            show_all: true,
            ..Default::default()
        },
    );
    assert!(
        all.findings
            .iter()
            .any(|f| f.rule_id == "research.excess-vocabulary" && f.severity == Severity::Low)
    );
}

#[test]
fn technical_terms_need_a_cluster_before_reporting() {
    let scanner = Scanner::builtin().unwrap();
    let single = scanner.scan_text(
        "design.md",
        "The trust boundary is explicit.",
        &ScanOptions::default(),
    );
    assert!(
        !single
            .findings
            .iter()
            .any(|f| f.rule_id == "codex.abstraction-cluster")
    );

    let clustered = scanner.scan_text(
        "design.md",
        "The boundary defines the execution surface, authority contract, and runtime posture.",
        &ScanOptions::default(),
    );
    assert!(
        clustered
            .findings
            .iter()
            .any(|f| f.rule_id == "codex.abstraction-cluster")
    );
}

#[test]
fn profile_can_suppress_contextual_rules() {
    let scanner = Scanner::builtin().unwrap();
    let report = scanner.scan_text(
        "message.txt",
        "The config lives in the project root.",
        &ScanOptions {
            profile: Profile::Casual,
            show_all: true,
        },
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.rule_id == "codex.artifact-residence")
    );
}

#[test]
fn vocabulary_uses_unicode_word_boundaries_and_character_columns() {
    let scanner = Scanner::builtin().unwrap();
    let report = scanner.scan_text(
        "unicode.md",
        "é delves but édelves does not count twice.",
        &ScanOptions {
            show_all: true,
            ..Default::default()
        },
    );
    let hits: Vec<_> = report
        .findings
        .iter()
        .filter(|finding| finding.matched.eq_ignore_ascii_case("delves"))
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].column, 3);
}

#[test]
fn embeds_the_full_research_dataset_but_only_activates_style_entries() {
    let scanner = Scanner::builtin().unwrap();
    let report = scanner.scan_text("empty.md", "", &ScanOptions::default());
    assert_eq!(report.vocabulary_candidates, 900);
    assert_eq!(report.active_style_vocabulary, 407);
}

#[test]
fn report_serializes_as_json() {
    let scanner = Scanner::builtin().unwrap();
    let report = scanner.scan_text(
        "draft.md",
        "It is worth noting that tests pass.",
        &ScanOptions::default(),
    );
    let output = report.render(Format::Json).unwrap();
    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(value["path"], "draft.md");
    assert!(!value["findings"].as_array().unwrap().is_empty());
}

#[test]
fn custom_terms_are_case_insensitive_but_still_respect_masking_and_boundaries() {
    let scanner = Scanner::builtin_with_custom_terms(&[CustomTerm {
        term: "Magic Surface".to_owned(),
        severity: Severity::Medium,
        message: "Repository-specific abstraction.".to_owned(),
        suggestion: "Name the concrete API.".to_owned(),
    }])
    .unwrap();
    let report = scanner.scan_text(
        "custom.md",
        "The MAGIC SURFACE is vague. `magic surface` and magic surfaces are excluded.",
        &ScanOptions::default(),
    );
    let hits: Vec<_> = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == "custom.repo-term")
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].matched, "MAGIC SURFACE");
    assert_eq!(hits[0].message, "Repository-specific abstraction.");
}
