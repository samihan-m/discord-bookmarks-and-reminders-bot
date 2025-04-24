use poise::serenity_prelude::{Colour, CreateEmbed, CreateMessage};

use crate::{
    components::{
        delete_message_button::get_delete_button,
        interaction_custom_id::DELETE_MESSAGE_INTERACTION_CUSTOM_ID,
        reminder::reminder_time_select_menu::select_menu::get_reminder_select_menu,
        trim_embed_description, trim_embed_title,
    },
    models::bookmark::PersistedBookmarkedMessage,
};

pub fn get_bookmark_message(
    bookmark: &PersistedBookmarkedMessage,
    bookmark_channel_name: &str,
    delete_interaction_emoji: impl Into<String>,
) -> CreateMessage {
    CreateMessage::default()
        .embed(get_bookmark_embed(bookmark, bookmark_channel_name))
        .select_menu(get_reminder_select_menu(
            crate::components::interaction_custom_id::InteractionCustomId::SetReminder(
                bookmark.bookmark_id(),
            ),
        ))
        .button(get_delete_button(
            DELETE_MESSAGE_INTERACTION_CUSTOM_ID,
            delete_interaction_emoji,
        ))
}

pub fn get_bookmark_embed(
    bookmark: &PersistedBookmarkedMessage,
    bookmark_channel_name: &str,
) -> CreateEmbed {
    let title = format!("Bookmarked message from {}", bookmark_channel_name);
    let trimmed_title = trim_embed_title(&title);

    let description = format!(
        "# {} \n # {}",
        bookmark.message().content,
        bookmark.message().link()
    );
    let trimmed_description = trim_embed_description(&description);

    CreateEmbed::default()
        .title(trimmed_title)
        .description(trimmed_description)
        .timestamp(bookmark.message().timestamp)
        .colour(Colour::TEAL)
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude::Message;
    use uuid::Uuid;

    use super::*;
    use crate::models::bookmark::BookmarkedMessage;

    /// [`CreateMessage`] doesn't impl [`PartialEq`] nor does it
    /// expose any of it's fields, so I can't actually test any values of type
    /// [`CreateMessage`] (e.g. the return value of [`get_bookmark_message`]).
    /// The best I can do is test the [`CreateEmbed`] returned by [`get_bookmark_embed`].
    #[test]
    fn test_get_bookmark_embed() {
        let bookmark = PersistedBookmarkedMessage::from_bookmarked_message(
            BookmarkedMessage::new(
                Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)),
                123456789,
                Message::default(),
            ),
            1,
        );
        let bookmark_channel_name = "test_channel";

        let embed = get_bookmark_embed(&bookmark, bookmark_channel_name);

        let expected_embed = CreateEmbed::default()
            .title(format!("Bookmarked message from {}", bookmark_channel_name))
            .description(format!(
                "# {} \n # {}",
                bookmark.message().content,
                bookmark.message().link()
            ))
            .timestamp(bookmark.message().timestamp)
            .colour(Colour::TEAL);
        assert_eq!(embed, expected_embed);
    }
}
