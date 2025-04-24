use poise::serenity_prelude::{ButtonStyle, CreateButton, ReactionType};

pub fn get_delete_button(custom_id: impl Into<String>, emoji: impl Into<String>) -> CreateButton {
    CreateButton::new(custom_id)
        .label("Delete")
        .emoji(ReactionType::Unicode(emoji.into()))
        .style(ButtonStyle::Danger)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_delete_button() {
        let custom_id = "delete_message";
        let emoji = "🗑️";
        let button = get_delete_button(custom_id, emoji);

        let expected_button = CreateButton::new(custom_id)
            .label("Delete")
            .emoji(ReactionType::Unicode(emoji.to_string()))
            .style(ButtonStyle::Danger);

        assert_eq!(button, expected_button);
    }
}
