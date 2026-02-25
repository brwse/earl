use crate::config::{PolicyEffect, PolicyMode, PolicyRule};
use crate::template::schema::CommandMode;

/// Result of a policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny,
}

/// Evaluate policies for a given subject, tool key, and tool mode.
///
/// Deny-overrides model:
/// 1. Collect all matching policies
/// 2. If any deny → deny
/// 3. If any allow → allow
/// 4. No match → deny (default deny)
pub fn evaluate(
    policies: &[PolicyRule],
    subject: &str,
    tool_key: &str,
    tool_mode: CommandMode,
) -> PolicyDecision {
    let mut has_allow = false;

    for policy in policies {
        if !matches_any(&policy.subjects, subject) {
            continue;
        }
        if !matches_any(&policy.tools, tool_key) {
            continue;
        }
        if let Some(modes) = &policy.modes {
            let mode_val = match tool_mode {
                CommandMode::Read => PolicyMode::Read,
                CommandMode::Write => PolicyMode::Write,
            };
            if !modes.contains(&mode_val) {
                continue;
            }
        }

        match policy.effect {
            PolicyEffect::Deny => return PolicyDecision::Deny,
            PolicyEffect::Allow => has_allow = true,
        }
    }

    if has_allow {
        PolicyDecision::Allow
    } else {
        PolicyDecision::Deny
    }
}

/// Filter tool keys that the subject is allowed to access.
#[allow(dead_code)]
pub(crate) fn filter_allowed<'a>(
    policies: &[PolicyRule],
    subject: &str,
    entries: impl Iterator<Item = (&'a str, CommandMode)>,
) -> Vec<&'a str> {
    entries
        .filter(|(key, mode)| evaluate(policies, subject, key, *mode) == PolicyDecision::Allow)
        .map(|(key, _)| key)
        .collect()
}

/// Glob match: `*` matches any characters except `.` (single segment).
/// A lone `*` pattern matches everything.
fn glob_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Guard against pathologically long inputs that could cause exponential backtracking
    if pattern.len() + value.len() > 256 {
        return false;
    }

    let pattern = pattern.to_ascii_lowercase();
    let value = value.to_ascii_lowercase();

    glob_matches_recursive(pattern.as_bytes(), value.as_bytes())
}

fn glob_matches_recursive(pattern: &[u8], value: &[u8]) -> bool {
    match (pattern.first(), value.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            // Try matching zero characters, or one non-dot character
            if glob_matches_recursive(&pattern[1..], value) {
                return true;
            }
            if let Some(&ch) = value.first()
                && ch != b'.'
            {
                return glob_matches_recursive(pattern, &value[1..]);
            }
            false
        }
        (Some(p), Some(v)) if p.eq_ignore_ascii_case(v) => {
            glob_matches_recursive(&pattern[1..], &value[1..])
        }
        _ => false,
    }
}

fn matches_any(patterns: &[String], value: &str) -> bool {
    patterns.iter().any(|pattern| glob_matches(pattern, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow_rule(subjects: &[&str], tools: &[&str]) -> PolicyRule {
        PolicyRule {
            subjects: subjects.iter().map(|s| s.to_string()).collect(),
            tools: tools.iter().map(|s| s.to_string()).collect(),
            modes: None,
            effect: PolicyEffect::Allow,
        }
    }

    fn deny_rule(subjects: &[&str], tools: &[&str]) -> PolicyRule {
        PolicyRule {
            subjects: subjects.iter().map(|s| s.to_string()).collect(),
            tools: tools.iter().map(|s| s.to_string()).collect(),
            modes: None,
            effect: PolicyEffect::Deny,
        }
    }

    fn allow_rule_with_modes(
        subjects: &[&str],
        tools: &[&str],
        modes: &[PolicyMode],
    ) -> PolicyRule {
        PolicyRule {
            subjects: subjects.iter().map(|s| s.to_string()).collect(),
            tools: tools.iter().map(|s| s.to_string()).collect(),
            modes: Some(modes.to_vec()),
            effect: PolicyEffect::Allow,
        }
    }

    // --- Glob matching tests ---

    #[test]
    fn exact_pattern_matches_identical_string() {
        assert!(glob_matches("github.create_issue", "github.create_issue"));
    }

    #[test]
    fn exact_pattern_does_not_match_different_string() {
        assert!(!glob_matches("github.create_issue", "github.delete_issue"));
    }

    #[test]
    fn star_matches_within_single_segment() {
        assert!(glob_matches("github.*", "github.create_issue"));
    }

    #[test]
    fn star_matches_different_suffix_within_single_segment() {
        assert!(glob_matches("github.*", "github.delete_repo"));
    }

    #[test]
    fn star_does_not_match_across_dots() {
        assert!(!glob_matches("github.*", "github.admin.delete"));
    }

    #[test]
    fn star_in_pattern_matches_within_segment() {
        assert!(glob_matches("*.delete_*", "github.delete_repo"));
    }

    #[test]
    fn star_in_pattern_matches_different_provider_prefix() {
        assert!(glob_matches("*.delete_*", "slack.delete_message"));
    }

    #[test]
    fn star_does_not_match_three_segment_key() {
        assert!(!glob_matches("*.delete_*", "github.admin.delete_repo"));
    }

    #[test]
    fn lone_star_matches_dotted_key() {
        assert!(glob_matches("*", "github.create_issue"));
    }

    #[test]
    fn lone_star_matches_undotted_value() {
        assert!(glob_matches("*", "anything"));
    }

    #[test]
    fn uppercase_pattern_matches_lowercase_value() {
        assert!(glob_matches("GitHub.*", "github.create_issue"));
    }

    #[test]
    fn lowercase_pattern_matches_uppercase_value() {
        assert!(glob_matches("github.*", "GitHub.Create_Issue"));
    }

    // --- Policy evaluation tests ---

    #[test]
    fn default_deny_when_no_policies() {
        let result = evaluate(&[], "alice", "github.create_issue", CommandMode::Read);
        assert_eq!(result, PolicyDecision::Deny);
    }

    #[test]
    fn allow_when_matching_allow_policy() {
        let policies = vec![allow_rule(&["alice"], &["github.*"])];
        let result = evaluate(&policies, "alice", "github.create_issue", CommandMode::Read);
        assert_eq!(result, PolicyDecision::Allow);
    }

    #[test]
    fn deny_when_subject_not_matched() {
        let policies = vec![allow_rule(&["alice"], &["github.*"])];
        let result = evaluate(&policies, "bob", "github.create_issue", CommandMode::Read);
        assert_eq!(result, PolicyDecision::Deny);
    }

    #[test]
    fn deny_when_tool_not_matched() {
        let policies = vec![allow_rule(&["alice"], &["github.*"])];
        let result = evaluate(&policies, "alice", "slack.send_message", CommandMode::Read);
        assert_eq!(result, PolicyDecision::Deny);
    }

    #[test]
    fn deny_overrides_allow() {
        let policies = vec![
            allow_rule(&["alice"], &["github.*"]),
            deny_rule(&["alice"], &["github.delete_*"]),
        ];
        let result = evaluate(&policies, "alice", "github.delete_repo", CommandMode::Write);
        assert_eq!(result, PolicyDecision::Deny);
    }

    #[test]
    fn wildcard_subject_matches_any_authenticated() {
        let policies = vec![allow_rule(&["*"], &["github.*"])];
        let result = evaluate(
            &policies,
            "anyone",
            "github.search_issues",
            CommandMode::Read,
        );
        assert_eq!(result, PolicyDecision::Allow);
    }

    #[test]
    fn read_mode_is_allowed_when_policy_restricts_to_read() {
        let policies = vec![allow_rule_with_modes(&["*"], &["*"], &[PolicyMode::Read])];
        assert_eq!(
            evaluate(&policies, "alice", "github.search", CommandMode::Read),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn write_mode_is_denied_when_policy_restricts_to_read() {
        let policies = vec![allow_rule_with_modes(&["*"], &["*"], &[PolicyMode::Read])];
        assert_eq!(
            evaluate(&policies, "alice", "github.create", CommandMode::Write),
            PolicyDecision::Deny
        );
    }

    #[test]
    fn filter_allowed_returns_only_explicitly_allowed_tools() {
        let policies = vec![
            allow_rule(&["alice"], &["github.*"]),
            deny_rule(&["alice"], &["github.delete_*"]),
        ];
        let entries = vec![
            ("github.search_issues", CommandMode::Read),
            ("github.create_issue", CommandMode::Write),
            ("github.delete_repo", CommandMode::Write),
            ("slack.send_message", CommandMode::Write),
        ];
        let allowed = filter_allowed(&policies, "alice", entries.into_iter());
        assert_eq!(allowed, vec!["github.search_issues", "github.create_issue"]);
    }
}
