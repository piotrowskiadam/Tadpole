//! robots.txt groups, longest matching Allow/Disallow rule, `*` and terminal `$`.
//! Sitemap declarations are global and independent of user-agent rule groups.
#[derive(Clone, Default)]
pub struct RobotsRules {
    rules: Vec<(regex::Regex, usize, bool)>,
    pub sitemaps: Vec<String>,
}

pub fn parse(content: &str, user_agent: &str) -> RobotsRules {
    type Group = (Vec<String>, Vec<(String, bool)>);
    let mut groups: Vec<Group> = Vec::new();
    let mut agents = Vec::new();
    let mut rules = Vec::new();
    let mut has_rules = false;
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "user-agent" => {
                if has_rules {
                    groups.push((std::mem::take(&mut agents), std::mem::take(&mut rules)));
                    has_rules = false;
                }
                agents.push(value.to_ascii_lowercase());
            }
            "allow" | "disallow" if !agents.is_empty() => {
                has_rules = true;
                if !value.is_empty() {
                    rules.push((value.to_string(), key.trim().eq_ignore_ascii_case("allow")));
                }
            }
            _ => {}
        }
    }
    groups.push((agents, rules));
    let agent = user_agent.to_ascii_lowercase();
    let specificity = |agents: &[String]| {
        agents
            .iter()
            .filter_map(|a| {
                if a == "*" {
                    Some(0)
                } else if agent.contains(a.as_str()) {
                    Some(a.len())
                } else {
                    None
                }
            })
            .max()
    };
    let best = groups.iter().filter_map(|(a, _)| specificity(a)).max();
    let mut result = RobotsRules::default();
    for line in content.lines() {
        if let Some((key, value)) = line.split('#').next().unwrap_or("").split_once(':')
            && key.trim().eq_ignore_ascii_case("sitemap")
            && !value.trim().is_empty()
            && !result.sitemaps.iter().any(|s| s == value.trim())
        {
            result.sitemaps.push(value.trim().to_string());
        }
    }
    for (agents, rules) in groups {
        if best.is_none() || specificity(&agents) != best {
            continue;
        }
        for (path, allow) in rules {
            let anchored = path.ends_with('$');
            let pattern = path
                .trim_end_matches('$')
                .split('*')
                .map(regex::escape)
                .collect::<Vec<_>>()
                .join(".*");
            if let Ok(re) =
                regex::Regex::new(&format!("^{pattern}{}", if anchored { "$" } else { "" }))
            {
                result
                    .rules
                    .push((re, path.trim_end_matches('$').replace('*', "").len(), allow));
            }
        }
    }
    result
}

impl RobotsRules {
    pub fn allows(&self, url: &str) -> bool {
        let Ok(url) = url::Url::parse(url) else {
            return false;
        };
        let path = match url.query() {
            Some(q) => format!("{}?{q}", url.path()),
            None => url.path().to_string(),
        };
        self.rules
            .iter()
            .filter(|(re, _, _)| re.is_match(&path))
            .max_by_key(|(_, length, allow)| (*length, *allow))
            .is_none_or(|(_, _, allow)| *allow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn groups_specificity_and_longest_allow() {
        let rules = parse(
            "User-agent: *\nDisallow: /\nUser-agent: Tadpole\nUser-agent: Other\nDisallow: /private\nAllow: /private/public\nDisallow: /*?secret=$",
            "Tadpole/1",
        );
        assert!(rules.allows("https://example.com/"));
        assert!(!rules.allows("https://example.com/private/x"));
        assert!(rules.allows("https://example.com/private/public/x"));
        assert!(!rules.allows("https://example.com/x?secret="));
        assert!(rules.allows("https://example.com/x?secret=yes"));
    }
}
