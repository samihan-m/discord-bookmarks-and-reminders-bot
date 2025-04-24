use poise::{
    serenity_prelude::{self as serenity, CreateEmbed},
    CreateReply,
};

use crate::{
    components::{
        relative_timestamp_string::get_discord_relative_timestamp_string, trim_embed_description,
        trim_embed_field_name, trim_embed_title,
    },
    models::reminder::PersistedReminder,
};

pub fn create_get_reminders_reply(reminders: &[PersistedReminder]) -> CreateReply {
    let title = format!(
        "Retrieved up to {} reminder{}.\nThere may be more reminders not shown.",
        reminders.len(),
        if reminders.len() > 1 { "s" } else { "" }
    );
    let trimmed_title = trim_embed_title(&title);

    let description = format!("## Queued Reminders: {}", reminders.len());
    let trimmed_description = trim_embed_description(&description);

    CreateReply::default()
        .embed(
            CreateEmbed::default()
                .title(trimmed_title)
                .description(trimmed_description)
                .fields(reminders.iter().map(|reminder| {
                    let field_name = format!(
                        "{} at: {}",
                        reminder.message().link(),
                        get_discord_relative_timestamp_string(reminder.remind_at())
                    );
                    let trimmed_field_name = trim_embed_field_name(&field_name);
                    (trimmed_field_name.to_owned(), "", true)
                }))
                .colour(serenity::Colour::TEAL),
        )
        .ephemeral(true)
}

#[cfg(test)]
mod tests {
    use crate::models::reminder::Reminder;

    use super::*;

    #[test]
    fn test_create_get_reminders_reply_for_one_reminder() {
        let timestamp = chrono::Utc::now();

        let guild_id = 1;
        let channel_id = 2;
        let message_id = 3;
        let message = {
            let mut message = serenity::Message::default();
            message.guild_id = Some(guild_id.into());
            message.channel_id = channel_id.into();
            message.id = message_id.into();
            message
        };

        let reminders = vec![PersistedReminder::from_reminder(
            Reminder::new(123456789, message, timestamp),
            1,
        )];

        let reply = create_get_reminders_reply(&reminders);

        assert_eq!(reply.ephemeral, Some(true));

        assert_eq!(reply.embeds.len(), 1);
        let embed = reply.embeds.get(0).unwrap().to_owned();

        let expected_embed = CreateEmbed::default()
            .title("Retrieved up to 1 reminder.\nThere may be more reminders not shown.")
            .description("## Queued Reminders: 1")
            .field(
                format!(
                    "https://discord.com/channels/{}/{}/{} at: <t:{}:R>",
                    guild_id,
                    channel_id,
                    message_id,
                    timestamp.timestamp()
                ),
                "",
                true,
            )
            .colour(serenity::Colour::TEAL);

        assert_eq!(embed, expected_embed);
    }

    #[test]
    fn test_create_get_reminders_reply_for_multiple_reminders() {
        let timestamp = chrono::Utc::now();

        let channel1_id = 1;
        let message1_id = 2;
        let dm_message = {
            let mut message = serenity::Message::default();
            message.channel_id = channel1_id.into();
            message.id = message1_id.into();
            message
        };

        let guild_id = 1;
        let channel2_id = 2;
        let message2_id = 3;
        let guild_message = {
            let mut message = serenity::Message::default();
            message.guild_id = Some(guild_id.into());
            message.channel_id = channel2_id.into();
            message.id = message2_id.into();
            message
        };

        let reminders = vec![
            PersistedReminder::from_reminder(Reminder::new(123456789, dm_message, timestamp), 1),
            PersistedReminder::from_reminder(Reminder::new(987654321, guild_message, timestamp), 2),
        ];

        let reply = create_get_reminders_reply(&reminders);

        assert_eq!(reply.ephemeral, Some(true));

        assert_eq!(reply.embeds.len(), 1);
        let embed = reply.embeds.get(0).unwrap().to_owned();

        let expected_embed = CreateEmbed::default()
            .title("Retrieved up to 2 reminders.\nThere may be more reminders not shown.")
            .description("## Queued Reminders: 2")
            .field(
                format!(
                    "https://discord.com/channels/@me/{}/{} at: <t:{}:R>",
                    channel1_id,
                    message1_id,
                    timestamp.timestamp()
                ),
                "",
                true,
            )
            .field(
                format!(
                    "https://discord.com/channels/{}/{}/{} at: <t:{}:R>",
                    guild_id,
                    channel2_id,
                    message2_id,
                    timestamp.timestamp()
                ),
                "",
                true,
            )
            .colour(serenity::Colour::TEAL);

        assert_eq!(embed, expected_embed);
    }
}
