/// Score how well `input` matches `candidate`. Returns 0 if no match.
/// Scoring: exact (1000) > prefix (800) > acronym (600) > substring (400) > no match (0).
/// Empty input returns 400 (matches everything moderately).
/// Case-insensitive.
pub fn match_score(input: &str, candidate: &str) -> u32 {
    if input.is_empty() {
        return 400;
    }

    let input_lower = input.to_lowercase();
    let cand_lower = candidate.to_lowercase();

    if input_lower == cand_lower {
        return 1000;
    }

    if cand_lower.starts_with(&input_lower) {
        return 800;
    }

    // Acronym match: input chars match first char of each underscore-separated word
    let words: Vec<&str> = cand_lower.split('_').filter(|w| !w.is_empty()).collect();
    if words.len() > 1 {
        let acronym: String = words.iter().filter_map(|w| w.chars().next()).collect();
        if acronym.starts_with(&input_lower) {
            return 600;
        }
        // Also try: each input char matches the initial of successive words
        if input_lower.len() <= words.len() {
            let chars: Vec<char> = input_lower.chars().collect();
            if chars.iter().zip(words.iter()).all(|(c, w)| w.starts_with(*c)) {
                return 600;
            }
        }
    }

    if cand_lower.contains(&input_lower) {
        return 400;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_highest() {
        assert_eq!(match_score("users", "users"), 1000);
    }

    #[test]
    fn exact_match_case_insensitive() {
        assert_eq!(match_score("USERS", "users"), 1000);
    }

    #[test]
    fn prefix_match() {
        let score = match_score("use", "users");
        assert!(score > 0, "prefix should score > 0");
        assert!(score < match_score("users", "users"), "prefix < exact");
        assert_eq!(score, 800);
    }

    #[test]
    fn prefix_case_insensitive() {
        assert!(match_score("USE", "users") > 0);
    }

    #[test]
    fn acronym_match_underscored() {
        assert!(match_score("ui", "user_id") > 0, "ui should match user_id");
        assert!(match_score("oi", "order_id") > 0, "oi should match order_id");
        assert_eq!(match_score("ui", "user_id"), 600);
    }

    #[test]
    fn acronym_no_match_for_single_word() {
        // "us" should NOT match "users" via acronym (no underscore), but may match as prefix/substring
        assert!(match_score("us", "users") > 0); // prefix/substring match
        assert_ne!(match_score("us", "users"), 600); // not acronym
    }

    #[test]
    fn substring_match() {
        let score = match_score("ser", "users");
        assert!(score > 0, "substring should score > 0");
        assert_eq!(score, 400);
    }

    #[test]
    fn no_match_returns_zero() {
        assert_eq!(match_score("xyz", "users"), 0);
        assert_eq!(match_score("zzz", "order_id"), 0);
    }

    #[test]
    fn prefix_beats_substring() {
        assert!(match_score("use", "users") > match_score("ser", "users"));
    }

    #[test]
    fn empty_input_returns_moderate_score() {
        assert_eq!(match_score("", "users"), 400);
    }

    #[test]
    fn empty_candidate() {
        // empty candidate can't match non-empty input
        assert_eq!(match_score("abc", ""), 0);
    }

    #[test]
    fn multi_word_acronym_partial() {
        // "ci" matches "created_id" via acronym
        assert_eq!(match_score("ci", "created_id"), 600);
    }
}
