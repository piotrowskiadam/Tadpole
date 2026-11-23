#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpportunityInfo {
    pub source_url: String,
    pub target_url: String,
    pub keyword: String,
    pub context: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NgramInfo {
    pub phrase: String,
    pub length: u8,
    pub frequency: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpportunityFilterField {
    #[default]
    EitherUrl,
    SourceUrl,
    TargetUrl,
    AnchorKeyword,
    ContextSnippet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterOperator {
    #[default]
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    Equals,
    NotEquals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterCombinator {
    #[default]
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpportunityFilterRule {
    pub field: OpportunityFilterField,
    pub operator: FilterOperator,
    pub raw_value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpportunityFilterSet {
    pub combinator: FilterCombinator,
    pub rules: Vec<OpportunityFilterRule>,
}

impl OpportunityFilterField {
    pub const ALL: [Self; 5] = [
        Self::EitherUrl,
        Self::SourceUrl,
        Self::TargetUrl,
        Self::AnchorKeyword,
        Self::ContextSnippet,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::EitherUrl => "Either URL (Source or Target)",
            Self::SourceUrl => "Source Page",
            Self::TargetUrl => "Target Page",
            Self::AnchorKeyword => "Anchor Keyword",
            Self::ContextSnippet => "Context Snippet",
        }
    }

    pub fn from_index(idx: u32) -> Self {
        Self::ALL.get(idx as usize).copied().unwrap_or_default()
    }

    pub fn to_index(self) -> u32 {
        Self::ALL.iter().position(|&f| f == self).unwrap_or(0) as u32
    }
}

impl FilterOperator {
    pub const ALL: [Self; 6] = [
        Self::Contains,
        Self::NotContains,
        Self::StartsWith,
        Self::EndsWith,
        Self::Equals,
        Self::NotEquals,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Contains => "Contains",
            Self::NotContains => "Does not contain",
            Self::StartsWith => "Starts with",
            Self::EndsWith => "Ends with",
            Self::Equals => "Equals",
            Self::NotEquals => "Does not equal",
        }
    }

    pub fn from_index(idx: u32) -> Self {
        Self::ALL.get(idx as usize).copied().unwrap_or_default()
    }

    pub fn to_index(self) -> u32 {
        Self::ALL.iter().position(|&op| op == self).unwrap_or(0) as u32
    }
}

impl FilterCombinator {
    pub const ALL: [Self; 2] = [Self::And, Self::Or];

    pub fn label(self) -> &'static str {
        match self {
            Self::And => "Match all (AND)",
            Self::Or => "Match any (OR)",
        }
    }

    pub fn from_index(idx: u32) -> Self {
        Self::ALL.get(idx as usize).copied().unwrap_or_default()
    }

    pub fn to_index(self) -> u32 {
        Self::ALL.iter().position(|&c| c == self).unwrap_or(0) as u32
    }
}

impl OpportunityFilterRule {
    pub fn tokens(&self) -> Vec<String> {
        self.raw_value
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(|t| t.to_lowercase())
            .collect()
    }

    pub fn is_active(&self) -> bool {
        self.raw_value.split(',').any(|t| !t.trim().is_empty())
    }

    pub fn matches(&self, opp: &OpportunityInfo) -> bool {
        let tokens = self.tokens();
        if tokens.is_empty() {
            return true;
        }
        match self.field {
            OpportunityFilterField::EitherUrl => {
                let source = opp.source_url.to_lowercase();
                let target = opp.target_url.to_lowercase();
                match self.operator {
                    FilterOperator::Contains => tokens
                        .iter()
                        .any(|t| source.contains(t) || target.contains(t)),
                    FilterOperator::NotContains => !tokens
                        .iter()
                        .any(|t| source.contains(t) || target.contains(t)),
                    FilterOperator::StartsWith => tokens
                        .iter()
                        .any(|t| source.starts_with(t) || target.starts_with(t)),
                    FilterOperator::EndsWith => tokens
                        .iter()
                        .any(|t| source.ends_with(t) || target.ends_with(t)),
                    FilterOperator::Equals => tokens.iter().any(|t| source == *t || target == *t),
                    FilterOperator::NotEquals => {
                        !tokens.iter().any(|t| source == *t || target == *t)
                    }
                }
            }
            OpportunityFilterField::SourceUrl => {
                Self::match_text(&opp.source_url.to_lowercase(), self.operator, &tokens)
            }
            OpportunityFilterField::TargetUrl => {
                Self::match_text(&opp.target_url.to_lowercase(), self.operator, &tokens)
            }
            OpportunityFilterField::AnchorKeyword => {
                Self::match_text(&opp.keyword.to_lowercase(), self.operator, &tokens)
            }
            OpportunityFilterField::ContextSnippet => {
                let stripped = strip_context_markup(&opp.context).to_lowercase();
                Self::match_text(&stripped, self.operator, &tokens)
            }
        }
    }

    pub fn matches_ngram(&self, ngram: &NgramInfo) -> bool {
        if self.field != OpportunityFilterField::AnchorKeyword {
            return true;
        }
        let tokens = self.tokens();
        if tokens.is_empty() {
            return true;
        }
        let phrase = ngram.phrase.to_lowercase();
        Self::match_text(&phrase, self.operator, &tokens)
    }

    fn match_text(haystack_lower: &str, operator: FilterOperator, tokens_lower: &[String]) -> bool {
        match operator {
            FilterOperator::Contains => tokens_lower.iter().any(|t| haystack_lower.contains(t)),
            FilterOperator::NotContains => !tokens_lower.iter().any(|t| haystack_lower.contains(t)),
            FilterOperator::StartsWith => {
                tokens_lower.iter().any(|t| haystack_lower.starts_with(t))
            }
            FilterOperator::EndsWith => tokens_lower.iter().any(|t| haystack_lower.ends_with(t)),
            FilterOperator::Equals => tokens_lower.iter().any(|t| haystack_lower == t),
            FilterOperator::NotEquals => !tokens_lower.iter().any(|t| haystack_lower == t),
        }
    }
}

pub fn strip_context_markup(context: &str) -> String {
    context.replace("<b>", "").replace("</b>", "")
}

impl OpportunityFilterSet {
    pub fn matches(&self, opp: &OpportunityInfo) -> bool {
        let mut active = self.rules.iter().filter(|r| r.is_active()).peekable();
        if active.peek().is_none() {
            return true;
        }
        match self.combinator {
            FilterCombinator::And => active.all(|r| r.matches(opp)),
            FilterCombinator::Or => active.any(|r| r.matches(opp)),
        }
    }

    pub fn matches_ngram(&self, ngram: &NgramInfo) -> bool {
        let mut active = self
            .rules
            .iter()
            .filter(|r| r.field == OpportunityFilterField::AnchorKeyword && r.is_active())
            .peekable();
        if active.peek().is_none() {
            return true;
        }
        match self.combinator {
            FilterCombinator::And => active.all(|r| r.matches_ngram(ngram)),
            FilterCombinator::Or => active.any(|r| r.matches_ngram(ngram)),
        }
    }
}
