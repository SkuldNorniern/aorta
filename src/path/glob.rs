use std::path::{Path, PathBuf};

fn pattern_matches(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = name.chars().collect();
    pattern_matches_impl(&p[..], &s[..])
}

fn pattern_matches_impl(p: &[char], s: &[char]) -> bool {
    match (p.first(), s.first()) {
        (None, None) => true,
        (Some(&'*'), None) => pattern_matches_impl(&p[1..], s),
        (Some(&'*'), _) => {
            pattern_matches_impl(&p[1..], s) || pattern_matches_impl(p, &s[1..])
        }
        (Some(&'?'), Some(_)) => pattern_matches_impl(&p[1..], &s[1..]),
        (Some(&a), Some(&b)) if a == b => pattern_matches_impl(&p[1..], &s[1..]),
        _ => false,
    }
}

pub fn expand_glob(pattern: &str) -> Result<Vec<PathBuf>, std::io::Error> {
    if !pattern.contains('*') && !pattern.contains('?') {
        return Ok(vec![PathBuf::from(pattern)]);
    }

    let (dir, pat) = if let Some(slash) = pattern.rfind('/') {
        let (d, p) = pattern.split_at(slash + 1);
        (if d.is_empty() { "." } else { d }, p)
    } else {
        (".", pattern)
    };

    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return Ok(vec![PathBuf::from(pattern)]);
    }

    let mut matches = Vec::new();
    for entry in std::fs::read_dir(dir_path)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if pattern_matches(pat, &name_str) {
            matches.push(entry.path());
        }
    }
    matches.sort();

    if matches.is_empty() {
        Ok(vec![PathBuf::from(pattern)])
    } else {
        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_star_matches_all() {
        assert!(pattern_matches("*", "foo"));
        assert!(pattern_matches("*", ""));
    }

    #[test]
    fn test_pattern_literal() {
        assert!(pattern_matches("foo", "foo"));
        assert!(!pattern_matches("foo", "bar"));
    }

    #[test]
    fn test_pattern_question() {
        assert!(pattern_matches("f?o", "foo"));
        assert!(pattern_matches("?at", "cat"));
        assert!(!pattern_matches("?at", "at"));
    }
}
