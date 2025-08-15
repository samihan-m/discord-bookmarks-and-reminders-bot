use poise::{
    serenity_prelude::{self as serenity, CreateEmbed},
    CreateReply,
};

use crate::{
    components::{trim_embed_description, trim_embed_field_name, trim_embed_title},
    models::bookmark::PersistedBookmarkedMessage,
};

pub fn create_get_bookmarks_reply(
    bookmarks: &[PersistedBookmarkedMessage],
    cache: &serenity::Cache,
) -> CreateReply {
    let title = format!(
        "Retrieved up to {} bookmark{}.\nThere may be more bookmarks not shown.",
        bookmarks.len(),
        if bookmarks.len() > 1 { "s" } else { "" }
    );
    let trimmed_title = trim_embed_title(&title);

    let description = format!("## Retrieved Bookmarks: {}", bookmarks.len());
    let trimmed_description = trim_embed_description(&description);

    const MESSAGE_PREVIEW_LENGTH: usize = 33;
    {
        const EMBED_FIELD_VALUE_MAX_LENGTH: usize = 1024;
        // the hard limit is 1024 but I want to keep it shorter than that for readability
        const _: () = assert!(MESSAGE_PREVIEW_LENGTH < EMBED_FIELD_VALUE_MAX_LENGTH);
    }
    fn get_trimmed_message_preview(content: &str) -> String {
        let preview = content
            .chars()
            .take(MESSAGE_PREVIEW_LENGTH)
            .collect::<String>();
        if content.chars().count() > MESSAGE_PREVIEW_LENGTH {
            format!("{}...", preview.trim_end()) // trimming to prevent previews like "This is a test message ..." (with a space before the ellipsis)
        } else {
            preview
        }
    }

    CreateReply::default()
        .embed(
            CreateEmbed::default()
                .title(trimmed_title)
                .description(trimmed_description)
                .fields(bookmarks.iter().map(|bookmark| {
                    let field_name = {
                        let link_to_message = bookmark.message().link();
                        // it's probably not good to be trimming a link, but that's better than having the embed be rejected for having a too-long field name...?
                        trim_embed_field_name(&link_to_message).to_owned()
                    };
                    let field_value = {
                        let message_contents = &bookmark.message().content_safe(cache);
                        let message_author = bookmark.message().author.name.clone();
                        let untrimmed_message_preview =
                            format!("{}: {}", message_author, message_contents);
                        get_trimmed_message_preview(&untrimmed_message_preview)
                    };
                    let is_inline = true;
                    (field_name, field_value, is_inline)
                }))
                .colour(serenity::Colour::TEAL),
        )
        .ephemeral(true)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::models::bookmark::BookmarkedMessage;

    use super::*;

    #[test]
    fn test_create_get_bookmarks_reply_for_one_bookmark() {
        let guild_id = 1;
        let channel_id = 2;
        let message_id = 3;
        let message_author_name = "TestUser";
        let message_content = "This is a test message content for the bookmarked message.";
        let message = {
            let mut message = serenity::Message::default();
            message.guild_id = Some(guild_id.into());
            message.channel_id = channel_id.into();
            message.id = message_id.into();
            message.author.name = message_author_name.to_string();
            message.content = message_content.to_string();
            message
        };

        let bookmarks = vec![PersistedBookmarkedMessage::from_bookmarked_message(
            BookmarkedMessage::new(
                Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)),
                123456789,
                message,
            ),
            1,
        )];

        let reply = create_get_bookmarks_reply(&bookmarks, &serenity::Cache::new());

        assert_eq!(reply.ephemeral, Some(true));

        assert_eq!(reply.embeds.len(), 1);
        let embed = reply.embeds.get(0).unwrap().to_owned();

        let expected_embed = CreateEmbed::default()
            .title("Retrieved up to 1 bookmark.\nThere may be more bookmarks not shown.")
            .description("## Retrieved Bookmarks: 1")
            .field(
                format!(
                    "https://discord.com/channels/{}/{}/{}",
                    guild_id, channel_id, message_id,
                ),
                "TestUser: This is a test message...",
                true,
            )
            .colour(serenity::Colour::TEAL);

        assert_eq!(embed, expected_embed);
    }

    #[test]
    fn test_create_get_bookmarks_reply_for_multiple_bookmarks() {
        let channel1_id = 1;
        let message1_id = 2;
        let message1_author_name = "User1";
        let message1_content = "Short message.";
        let dm_message = {
            let mut message = serenity::Message::default();
            message.channel_id = channel1_id.into();
            message.id = message1_id.into();
            message.author.name = message1_author_name.to_string();
            message.content = message1_content.to_string();
            message
        };

        let guild_id = 1;
        let channel2_id = 2;
        let message2_id = 3;
        let message2_author_name = "User2";
        let message2_content = "This is a longer test message content to test trimming behavior.";
        let guild_message = {
            let mut message = serenity::Message::default();
            message.guild_id = Some(guild_id.into());
            message.channel_id = channel2_id.into();
            message.id = message2_id.into();
            message.author.name = message2_author_name.to_string();
            message.content = message2_content.to_string();
            message
        };

        let bookmarks = vec![
            PersistedBookmarkedMessage::from_bookmarked_message(
                BookmarkedMessage::new(
                    Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)),
                    123456789,
                    dm_message,
                ),
                1,
            ),
            PersistedBookmarkedMessage::from_bookmarked_message(
                BookmarkedMessage::new(
                    Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)),
                    987654321,
                    guild_message,
                ),
                2,
            ),
        ];

        let reply = create_get_bookmarks_reply(&bookmarks, &serenity::Cache::new());

        assert_eq!(reply.ephemeral, Some(true));

        assert_eq!(reply.embeds.len(), 1);
        let embed = reply.embeds.get(0).unwrap().to_owned();

        let expected_embed = CreateEmbed::default()
            .title("Retrieved up to 2 bookmarks.\nThere may be more bookmarks not shown.")
            .description("## Retrieved Bookmarks: 2")
            .field(
                format!(
                    "https://discord.com/channels/@me/{}/{}",
                    channel1_id, message1_id,
                ),
                "User1: Short message.",
                true,
            )
            .field(
                format!(
                    "https://discord.com/channels/{}/{}/{}",
                    guild_id, channel2_id, message2_id,
                ),
                "User2: This is a longer test mess...",
                true,
            )
            .colour(serenity::Colour::TEAL);

        assert_eq!(embed, expected_embed);
    }
}
