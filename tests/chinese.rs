use prose_lint::{Profile, ScanOptions, Scanner, Severity};

#[test]
fn covers_every_current_upstream_rule_family() {
    let scanner = Scanner::builtin().unwrap();
    let cases = [
        ("zh.reversal", "这不是工具的问题，而是判断的问题。"),
        ("zh.enumeration", "需要采集、存储、展示这些数据。"),
        (
            "zh.sentence-template",
            "团队完成了本轮测试，结果符合预期。用户完成了本轮操作，反馈符合预期。",
        ),
        ("zh.em-dash", "答案很简单——专注。"),
        ("zh.colon-preamble", "一句话总结：成本太高。"),
        (
            "zh.list-announcement",
            "基础检查包括：\n- 检查配置\n- 检查日志",
        ),
        (
            "zh.numbered-heading",
            "## 一、配置\n\n内容。\n\n## 二、运行\n\n内容。\n\n## 三、验证\n",
        ),
        ("zh.idealized-persona", "它像一位智慧的导师，提供建议。"),
        (
            "zh.vague-summary",
            "效率显著提升，耗时从两小时缩短到四十分钟。",
        ),
        ("zh.nominalization", "团队完成了对流程的优化。"),
        ("zh.stock-opener", "先说结论，这个项目预算不足。"),
        (
            "zh.long-modifier",
            "这是一个能够让整个团队在不增加任何人力的情况下提升审核速度的工具。",
        ),
        ("zh.when-clause", "当所有人都能写代码时，判断更重要。"),
        ("zh.topic-shell", "对于早期团队来说，招人很难。"),
        ("zh.transition", "然而，这个方案仍需测试。"),
        ("zh.restatement", "留存率涨了。这意味着产品更受欢迎。"),
        (
            "zh.zero-subject",
            "配置按用户隔离。\n\n值得注意的是，权限还需检查。",
        ),
    ];
    for (id, text) in cases {
        let report = scanner.scan_text("zh.md", text, &ScanOptions::default());
        assert!(
            report.findings.iter().any(|f| f.rule_id == id),
            "missing {id}: {text}"
        );
        for f in report
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("zh."))
        {
            assert_eq!(f.severity, Severity::Medium);
            assert_eq!(&text[f.start..f.end], f.matched);
        }
    }
}

#[test]
fn chinese_masks_and_unicode_locations_work_in_all_profiles() {
    let scanner = Scanner::builtin().unwrap();
    let text = "Use `先说结论` literally.\n~~~text\n先说结论：结果如下。\n~~~\nhttps://example.com/先说结论\n中文：先说结论，先测试。\n";
    for profile in [
        Profile::Technical,
        Profile::Academic,
        Profile::Pr,
        Profile::Commit,
        Profile::Casual,
        Profile::Marketing,
    ] {
        let report = scanner.scan_text(
            "mixed.md",
            text,
            &ScanOptions {
                profile,
                show_all: false,
            },
        );
        let hits: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule_id == "zh.stock-opener")
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].line, hits[0].column), (6, 4));
        assert_eq!(report.high_confidence_count(), 0);
    }
}

#[test]
fn structural_rules_respect_paragraphs_headings_and_reference_words() {
    let scanner = Scanner::builtin().unwrap();
    for text in [
        "## 一、配置\n\n## 二、运行\n",
        "首先，读取文件。其次，解析数据。最后，输出结果。",
        "值得注意的是，这是第一段。",
        "配置按用户隔离。\n\n值得注意的是，这还需要权限检查。",
        "团队完成了本轮测试，结果符合预期。\n\n用户完成了本轮操作，反馈符合预期。",
        "```\n## 一、配置\n## 二、运行\n## 三、验证\n```",
    ] {
        let report = scanner.scan_text("legitimate.md", text, &ScanOptions::default());
        assert!(
            !report.findings.iter().any(|f| matches!(
                f.rule_id.as_str(),
                "zh.numbered-heading" | "zh.zero-subject" | "zh.sentence-template"
            )),
            "{text}"
        );
    }
}

#[test]
fn chinese_regexes_do_not_cross_code_or_sentence_boundaries() {
    let scanner = Scanner::builtin().unwrap();
    for text in [
        "不是字符偏移 `opaque` 而是字节偏移。",
        "不是字符偏移\n```text\nopaque\n```\n而是字节偏移。",
        "不是字符偏移 https://example.com 而是字节偏移。",
        "不是字符偏移。后续还有多种情况，而是字节偏移只是一种。",
    ] {
        let report = scanner.scan_text("boundaries.md", text, &ScanOptions::default());
        assert!(
            !report.findings.iter().any(|f| f.rule_id == "zh.reversal"),
            "{text}"
        );
    }
    let report = scanner.scan_text(
        "contrast.md",
        "不是字符偏移。而是字节偏移。",
        &ScanOptions::default(),
    );
    assert!(report.findings.iter().any(|f| f.rule_id == "zh.reversal"));
}

#[test]
fn aggressive_review_keeps_legitimate_contrasts_advisory() {
    let report = Scanner::builtin().unwrap().scan_text(
        "legitimate.md",
        "这里不是字符偏移，而是字节偏移。",
        &ScanOptions::default(),
    );
    assert!(report.findings.iter().any(|f| f.rule_id == "zh.reversal"));
    assert_eq!(report.high_confidence_count(), 0);
}
