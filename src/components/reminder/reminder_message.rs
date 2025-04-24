use poise::serenity_prelude::{Colour, CreateEmbed, CreateMessage};

use crate::{
    components::{
        delete_message_button::get_delete_button, trim_embed_description, trim_embed_title,
    },
    models::reminder::PersistedReminder,
};

pub fn get_reminder_message(
    reminder: &PersistedReminder,
    reminder_channel_name: &str,
    delete_interaction_custom_id: impl Into<String>,
    delete_interaction_emoji: impl Into<String>,
) -> CreateMessage {
    CreateMessage::default()
        .embed(get_reminder_embed(reminder, reminder_channel_name))
        .button(get_delete_button(
            delete_interaction_custom_id,
            delete_interaction_emoji,
        ))
}

fn get_reminder_embed(reminder: &PersistedReminder, reminder_channel_name: &str) -> CreateEmbed {
    let title = format!("Reminder from {}", reminder_channel_name);
    let trimmed_title = trim_embed_title(&title);

    let description = format!(
        "# {} \n # {}",
        reminder.message().content,
        reminder.message().link()
    );
    let trimmed_description = trim_embed_description(&description);

    CreateEmbed::default()
        .title(trimmed_title)
        .description(trimmed_description)
        .timestamp(reminder.message().timestamp)
        .colour(Colour::TEAL)
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude::Message;

    use super::*;
    use crate::models::reminder::Reminder;

    /// [`CreateMessage`] doesn't impl [`PartialEq`] nor does it
    /// expose any of it's fields, so I can't actually test any values of type
    /// [`CreateMessage`] (e.g. the return value of [`get_reminder_message`]).
    /// The best I can do is test the [`CreateEmbed`] returned by [`get_reminder_embed`].
    #[test]
    fn test_get_reminder_embed() {
        let timestamp = chrono::Utc::now();
        let reminder = PersistedReminder::from_reminder(
            Reminder::new(123456789, Message::default(), timestamp),
            1,
        );
        let reminder_channel_name = "test_channel";

        let embed = get_reminder_embed(&reminder, reminder_channel_name);

        let expected_embed = CreateEmbed::default()
            .title(format!("Reminder from {}", reminder_channel_name))
            .description(format!(
                "# {} \n # {}",
                reminder.message().content,
                reminder.message().link()
            ))
            .timestamp(reminder.message().timestamp)
            .colour(Colour::TEAL);
        assert_eq!(embed, expected_embed);
    }
}
