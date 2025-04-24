use poise::{
    serenity_prelude::{self as serenity, CreateEmbed},
    CreateReply,
};

use crate::{
    components::{trim_embed_description, trim_embed_field_name, trim_embed_title},
    models::bookmark::PersistedBookmarkedMessage,
};

pub fn create_get_bookmarks_reply(bookmarks: &[PersistedBookmarkedMessage]) -> CreateReply {
    let title = format!(
        "Retrieved up to {} bookmark{}.\nThere may be more bookmarks not shown.",
        bookmarks.len(),
        if bookmarks.len() > 1 { "s" } else { "" }
    );
    let trimmed_title = trim_embed_title(&title);

    let description = format!("## Retrieved Bookmarks: {}", bookmarks.len());
    let trimmed_description = trim_embed_description(&description);

    CreateReply::default()
        .embed(
            CreateEmbed::default()
                .title(trimmed_title)
                .description(trimmed_description)
                .fields(bookmarks.iter().map(|bookmark| {
                    let field_name = bookmark.message().link();
                    let trimmed_field_name = trim_embed_field_name(&field_name);
                    (trimmed_field_name.to_owned(), "", false)
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
        let message = {
            let mut message = serenity::Message::default();
            message.guild_id = Some(guild_id.into());
            message.channel_id = channel_id.into();
            message.id = message_id.into();
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

        let reply = create_get_bookmarks_reply(&bookmarks);

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
                "",
                false,
            )
            .colour(serenity::Colour::TEAL);

        assert_eq!(embed, expected_embed);
    }

    #[test]
    fn test_create_get_bookmarks_reply_for_multiple_bookmarks() {
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

        let reply = create_get_bookmarks_reply(&bookmarks);

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
                "",
                false,
            )
            .field(
                format!(
                    "https://discord.com/channels/{}/{}/{}",
                    guild_id, channel2_id, message2_id,
                ),
                "",
                false,
            )
            .colour(serenity::Colour::TEAL);

        assert_eq!(embed, expected_embed);
    }
}
