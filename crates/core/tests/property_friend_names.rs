mod support;

use proptest::prelude::*;
use support::{adversarial_text, meaningful_text, user_id};
use vrcx_0_core::friends::meaningful_display_name;

fn is_meaningful(value: &str, user_id: &str) -> bool {
    !value.is_empty() && value != user_id && value != "Unknown" && !value.starts_with("usr_")
}

proptest! {
    #[test]
    fn selected_name_is_a_trimmed_meaningful_candidate(
        display_name in adversarial_text(),
        username in adversarial_text(),
        user_id in user_id(),
    ) {
        if let Some(selected) = meaningful_display_name(&display_name, &username, &user_id) {
            prop_assert!(
                selected == display_name.trim() || selected == username.trim()
            );
            prop_assert!(is_meaningful(&selected, user_id.trim()));
        }
    }

    #[test]
    fn meaningful_display_name_has_priority(
        display_name in meaningful_text(),
        username in adversarial_text(),
        user_id in user_id(),
    ) {
        prop_assert_eq!(
            meaningful_display_name(&display_name, &username, &user_id),
            Some(display_name.trim().to_string())
        );
    }

    #[test]
    fn placeholder_display_name_falls_back_to_meaningful_username(
        placeholder_kind in 0u8..4,
        username in meaningful_text(),
        user_id in user_id(),
    ) {
        let display_name = match placeholder_kind {
            0 => String::new(),
            1 => " \n\t ".to_string(),
            2 => "Unknown".to_string(),
            _ => user_id.clone(),
        };

        prop_assert_eq!(
            meaningful_display_name(&display_name, &username, &user_id),
            Some(username.trim().to_string())
        );
    }

    #[test]
    fn placeholder_candidates_never_produce_a_name(
        display_kind in 0u8..5,
        username_kind in 0u8..5,
        user_id in user_id(),
        placeholder_suffix in adversarial_text(),
    ) {
        let placeholder = |kind| match kind {
            0 => String::new(),
            1 => " \n\t ".to_string(),
            2 => "Unknown".to_string(),
            3 => user_id.clone(),
            _ => format!("usr_{placeholder_suffix}"),
        };

        prop_assert_eq!(
            meaningful_display_name(
                &placeholder(display_kind),
                &placeholder(username_kind),
                &user_id,
            ),
            None
        );
    }
}
