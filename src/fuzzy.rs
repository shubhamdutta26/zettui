//! Small dependency-free fuzzy matcher shared by navigation pickers.

/// Lower scores sort first: exact, prefix, substring, then ordered subsequence.
/// Within each class prefer fewer skipped characters, earlier starts, and
/// shorter candidates. Positions count Unicode scalar values, not UTF-8 bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Score {
    class: u8,
    gaps: usize,
    start: usize,
    length: usize,
}

pub(crate) fn score(candidate: &str, query: &str) -> Option<Score> {
    let query = query.trim().to_lowercase();
    let candidate = candidate.to_lowercase();
    let length = candidate.chars().count();
    if query.is_empty() {
        // Equal scores preserve the caller's original ordering.
        return Some(Score {
            class: 0,
            gaps: 0,
            start: 0,
            length: 0,
        });
    }
    if let Some(byte_start) = candidate.find(query.as_str()) {
        let start = candidate[..byte_start].chars().count();
        let class = if candidate == query {
            0
        } else if start == 0 {
            1
        } else {
            2
        };
        return Some(Score {
            class,
            gaps: 0,
            start,
            length,
        });
    }
    let query: Vec<char> = query.chars().collect();
    if query.len() > length {
        return None;
    }
    let mut matched = 0;
    let mut start = 0;
    for (position, ch) in candidate.chars().enumerate() {
        if ch == query[matched] {
            if matched == 0 {
                start = position;
            }
            matched += 1;
            if matched == query.len() {
                return Some(Score {
                    class: 3,
                    gaps: position - start + 1 - matched,
                    start,
                    length,
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranking_prefers_exact_prefix_substring_then_subsequence() {
        let exact = score("SEC", "sec").unwrap();
        let prefix = score("SEC analysis", "sec").unwrap();
        let substring = score("Analytical SEC", "sec").unwrap();
        let fuzzy = score("Size exclusion chromatography", "sec").unwrap();
        assert!(exact < prefix && prefix < substring && substring < fuzzy);
        assert!(score("Purification", "sec").is_none());
    }

    #[test]
    fn unicode_case_and_order_are_respected() {
        assert!(score("Échantillon α", "éα").is_some());
        assert!(score("β α", "αβ").is_none());
        assert!(score("a", "aa").is_none());
        assert!(score("", "x").is_none());
        assert_eq!(score("Short", "  "), score("Long heading", ""));
    }

    #[test]
    fn compact_subsequences_rank_higher() {
        assert!(score("a-b-c", "abc").unwrap() < score("a---b---c", "abc").unwrap());
    }
}
