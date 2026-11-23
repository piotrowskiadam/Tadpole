use tadpole::link_opportunities::{
    FilterCombinator, FilterOperator, NgramInfo, OpportunityFilterField, OpportunityFilterRule,
    OpportunityFilterSet, OpportunityInfo,
};

fn sample_opp() -> OpportunityInfo {
    OpportunityInfo {
        source_url: "https://example.com/blog/technical-seo-guide".to_string(),
        target_url: "https://example.com/tag/seo-audit".to_string(),
        keyword: "Technical SEO Audit".to_string(),
        context: "...learn how a <b>Technical SEO Audit</b> improves crawlability...".to_string(),
    }
}

#[test]
fn empty_or_blank_rule_is_inactive_and_passes_all_opportunities() {
    let opp = sample_opp();
    let set = OpportunityFilterSet {
        combinator: FilterCombinator::And,
        rules: vec![OpportunityFilterRule {
            field: OpportunityFilterField::EitherUrl,
            operator: FilterOperator::NotContains,
            raw_value: " ,  , ".to_string(),
        }],
    };
    assert!(set.matches(&opp));
}

#[test]
fn single_field_operators_match_case_insensitively_with_comma_tokens_and_strip_bold_tags() {
    let opp = sample_opp();

    // Contains (any token matches)
    let contains_source = OpportunityFilterRule {
        field: OpportunityFilterField::SourceUrl,
        operator: FilterOperator::Contains,
        raw_value: "/news/, /BLOG/technical".to_string(),
    };
    assert!(contains_source.matches(&opp));

    // NotContains (none of the tokens may match)
    let not_contains_target = OpportunityFilterRule {
        field: OpportunityFilterField::TargetUrl,
        operator: FilterOperator::NotContains,
        raw_value: "/author/, /TAG/".to_string(),
    };
    assert!(!not_contains_target.matches(&opp));

    // StartsWith & EndsWith
    let starts_with_kw = OpportunityFilterRule {
        field: OpportunityFilterField::AnchorKeyword,
        operator: FilterOperator::StartsWith,
        raw_value: "other, technical seo".to_string(),
    };
    assert!(starts_with_kw.matches(&opp));

    let ends_with_kw = OpportunityFilterRule {
        field: OpportunityFilterField::AnchorKeyword,
        operator: FilterOperator::EndsWith,
        raw_value: "SEO AUDIT".to_string(),
    };
    assert!(ends_with_kw.matches(&opp));

    // Equals & NotEquals
    let equals_kw = OpportunityFilterRule {
        field: OpportunityFilterField::AnchorKeyword,
        operator: FilterOperator::Equals,
        raw_value: "technical seo audit".to_string(),
    };
    assert!(equals_kw.matches(&opp));

    let not_equals_kw = OpportunityFilterRule {
        field: OpportunityFilterField::AnchorKeyword,
        operator: FilterOperator::NotEquals,
        raw_value: "other, TECHNICAL SEO AUDIT".to_string(),
    };
    assert!(!not_equals_kw.matches(&opp));

    // ContextSnippet strips <b> and </b> so phrases spanning the bold boundary match cleanly
    let context_across_bold = OpportunityFilterRule {
        field: OpportunityFilterField::ContextSnippet,
        operator: FilterOperator::Contains,
        raw_value: "how a Technical SEO Audit improves".to_string(),
    };
    assert!(context_across_bold.matches(&opp));
}

#[test]
fn either_url_and_combinators_evaluate_ahrefs_style_multi_rules() {
    let clean_opp = OpportunityInfo {
        source_url: "https://example.com/guides/internal-linking".to_string(),
        target_url: "https://example.com/guides/site-architecture".to_string(),
        keyword: "site architecture".to_string(),
        context: "...improve your <b>site architecture</b> with links...".to_string(),
    };
    let tag_target_opp = sample_opp();
    let author_source_opp = OpportunityInfo {
        source_url: "https://example.com/author/adam-piotrowski".to_string(),
        target_url: "https://example.com/guides/site-architecture".to_string(),
        keyword: "adama piotrowskiego".to_string(),
        context: "...written by <b>adama piotrowskiego</b>...".to_string(),
    };

    let exclude_noise_and = OpportunityFilterSet {
        combinator: FilterCombinator::And,
        rules: vec![
            OpportunityFilterRule {
                field: OpportunityFilterField::EitherUrl,
                operator: FilterOperator::NotContains,
                raw_value: "/tag/, /author/, ?page=".to_string(),
            },
            OpportunityFilterRule {
                field: OpportunityFilterField::AnchorKeyword,
                operator: FilterOperator::NotContains,
                raw_value: "adama, piotrowskiego, blog".to_string(),
            },
        ],
    };

    assert!(exclude_noise_and.matches(&clean_opp));
    assert!(!exclude_noise_and.matches(&tag_target_opp));
    assert!(!exclude_noise_and.matches(&author_source_opp));

    let match_any_or = OpportunityFilterSet {
        combinator: FilterCombinator::Or,
        rules: vec![
            OpportunityFilterRule {
                field: OpportunityFilterField::TargetUrl,
                operator: FilterOperator::Contains,
                raw_value: "/tag/".to_string(),
            },
            OpportunityFilterRule {
                field: OpportunityFilterField::AnchorKeyword,
                operator: FilterOperator::Contains,
                raw_value: "adama".to_string(),
            },
            // Inactive empty rule should be ignored in OR evaluation
            OpportunityFilterRule {
                field: OpportunityFilterField::SourceUrl,
                operator: FilterOperator::Contains,
                raw_value: "   ".to_string(),
            },
        ],
    };

    assert!(!match_any_or.matches(&clean_opp));
    assert!(match_any_or.matches(&tag_target_opp));
    assert!(match_any_or.matches(&author_source_opp));
}

#[test]
fn matches_ngram_evaluates_only_anchor_keyword_rules_against_ngram_phrase() {
    let clean_ngram = NgramInfo {
        phrase: "technical seo audit".to_string(),
        length: 3,
        frequency: 12,
    };
    let excluded_ngram = NgramInfo {
        phrase: "adama piotrowskiego blog".to_string(),
        length: 3,
        frequency: 9,
    };

    let set = OpportunityFilterSet {
        combinator: FilterCombinator::And,
        rules: vec![
            // Non-anchor rule should be ignored when filtering N-grams
            OpportunityFilterRule {
                field: OpportunityFilterField::EitherUrl,
                operator: FilterOperator::Contains,
                raw_value: "/only-specific-path/".to_string(),
            },
            OpportunityFilterRule {
                field: OpportunityFilterField::AnchorKeyword,
                operator: FilterOperator::NotContains,
                raw_value: "adama, piotrowskiego".to_string(),
            },
        ],
    };

    assert!(set.matches_ngram(&clean_ngram));
    assert!(!set.matches_ngram(&excluded_ngram));

    // When no active AnchorKeyword rules exist, all N-grams pass
    let url_only_set = OpportunityFilterSet {
        combinator: FilterCombinator::And,
        rules: vec![OpportunityFilterRule {
            field: OpportunityFilterField::SourceUrl,
            operator: FilterOperator::Contains,
            raw_value: "/blog/".to_string(),
        }],
    };
    assert!(url_only_set.matches_ngram(&clean_ngram));
    assert!(url_only_set.matches_ngram(&excluded_ngram));
}
