pub fn first_non_empty<'a>(values: impl IntoIterator<Item = &'a str>) -> &'a str {
    values
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or("")
        .trim()
}

pub fn first_non_empty_owned<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    first_non_empty(values).to_string()
}

pub fn first_owned(values: impl IntoIterator<Item = String>) -> String {
    let value = values
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or_default();
    if value.trim().len() == value.len() {
        return value;
    }
    value.trim().to_string()
}

pub fn contains_lowercase_query_case_insensitive(value: &str, lowercase_query: &str) -> bool {
    if lowercase_query.is_empty() {
        return true;
    }
    if value.is_ascii() && lowercase_query.is_ascii() {
        return value
            .as_bytes()
            .windows(lowercase_query.len())
            .any(|window| window.eq_ignore_ascii_case(lowercase_query.as_bytes()));
    }
    value.to_lowercase().contains(lowercase_query)
}

#[cfg(test)]
mod tests {
    use super::{contains_lowercase_query_case_insensitive, first_non_empty, first_owned};

    #[test]
    fn first_non_empty_skips_blanks_and_trims() {
        assert_eq!(first_non_empty(["", "   ", " picked ", "later"]), "picked");
        assert_eq!(first_non_empty(["", "  "]), "");
    }

    #[test]
    fn first_owned_skips_blanks_and_trims() {
        assert_eq!(
            first_owned(["".to_string(), " picked ".to_string()]),
            "picked"
        );
        assert_eq!(first_owned([String::new()]), "");
    }

    #[test]
    fn case_insensitive_contains_matches_lowercase_search_semantics() {
        for (value, query) in [
            ("Player One", "player"),
            ("usr_ABC123", "abc"),
            ("İstanbul", "i"),
            ("Straße", "straße"),
            ("anything", ""),
        ] {
            let lowercase_query = query.to_lowercase();
            assert_eq!(
                contains_lowercase_query_case_insensitive(value, &lowercase_query),
                value.to_lowercase().contains(&lowercase_query)
            );
        }
    }
}
