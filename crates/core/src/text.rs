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

#[cfg(test)]
mod tests {
    use super::{first_non_empty, first_owned};

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
}
