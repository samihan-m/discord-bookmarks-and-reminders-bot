use crate::{
    components::{
        bookmark::{
            bookmark_created_reply::{
                get_bookmark_already_exists_reply, get_bookmark_created_reply,
                get_failed_to_create_bookmark_reply,
            },
            bookmark_message::get_bookmark_message,
            get_bookmark_reply::create_get_bookmarks_reply,
            no_bookmarks_found_reply::get_no_bookmarks_found_reply,
        },
        reminder::{
            get_reminders_reply::create_get_reminders_reply,
            no_reminders_found_reply::get_no_reminders_found_reply,
            reminder_created_reply::get_reminder_created_reply,
        },
        DELETE_MESSAGE_EMOJI,
    },
    database::{
        bookmark::InsertBookmarkError,
        reminder::{get_reminders_for_user, insert_reminder},
    },
    models::reminder::Reminder,
};

use poise::{
    serenity_prelude::{self as serenity, CreateMessage},
    CreateReply,
};

use crate::{Context, Error};

/// A slightly modified version of [`poise::builtins::autocomplete_command`] that trims the input string
/// to enable something kinda like a fuzzy search - I wanted this because I found myself inputting
/// an extra space at the beginning of the input term and breaking the autocomplete functionality,
/// so I wanted to fix that.
#[expect(clippy::unused_async)]
pub async fn autocomplete_command<'a, U, E>(
    ctx: poise::Context<'a, U, E>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    ctx.framework()
        .options()
        .commands
        .iter()
        .filter(move |cmd| cmd.name.starts_with(partial.trim()))
        .map(|cmd| cmd.name.to_string())
}

/// Show this help menu
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "Hope this helped :)",
            show_context_menu_commands: true,
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

/// Get a subset of your pending reminders
#[poise::command(slash_command)]
pub async fn get_reminders(
    ctx: Context<'_>,
    #[description = "Maximum quantity of reminders to fetch. Defaults to 20."]
    #[min = 1]
    #[max = 100]
    maximum_quantity: Option<u64>,
) -> Result<(), Error> {
    let quantity_to_retrieve = maximum_quantity.unwrap_or(20);

    let reminders = get_reminders_for_user(
        &ctx.data().db_connection,
        ctx.author().id.get(),
        quantity_to_retrieve,
    )
    .await?;

    if reminders.is_empty() {
        ctx.send(get_no_reminders_found_reply()).await?;
        return Ok(());
    }

    ctx.send(create_get_reminders_reply(&reminders)).await?;

    Ok(())
}

pub async fn add_reminder(ctx: &Context<'_>, reminder: Reminder) -> Result<(), Error> {
    let reminder = insert_reminder(&ctx.data().db_connection, reminder).await?;

    ctx.data().tx.send(reminder).await?;

    Ok(())
}

#[poise::command(context_menu_command = "Remind me in 10 seconds")]
pub async fn remind_me_in_10_seconds(
    ctx: Context<'_>,
    message: serenity::Message,
) -> Result<(), Error> {
    let remind_at = chrono::Utc::now() + chrono::Duration::seconds(10);

    let reminder = Reminder::new(ctx.author().id.get(), message, remind_at);

    add_reminder(&ctx, reminder).await?;

    ctx.send(get_reminder_created_reply(&remind_at)).await?;

    Ok(())
}

/// Get a subset of your bookmarks
#[poise::command(slash_command)]
pub async fn get_bookmarks(
    ctx: Context<'_>,
    #[description = "Offset to start fetching bookmarks from. Defaults to 0."]
    #[min = 0]
    offset: Option<u64>,
    #[description = "Maximum quantity of bookmarks to fetch. Defaults to 20."]
    #[min = 1]
    #[max = 100]
    maximum_quantity: Option<u64>,
) -> Result<(), Error> {
    let offset = offset.unwrap_or(0);
    let quantity_to_retrieve = maximum_quantity.unwrap_or(20);

    let bookmarks = crate::database::bookmark::get_bookmarks_for_user(
        &ctx.data().db_connection,
        ctx.author().id.get(),
        quantity_to_retrieve,
        offset,
    )
    .await?;

    if bookmarks.is_empty() {
        ctx.send(get_no_bookmarks_found_reply()).await?;
        return Ok(());
    }

    ctx.send(create_get_bookmarks_reply(&bookmarks)).await?;

    Ok(())
}

#[poise::command(context_menu_command = "Bookmark")]
pub async fn bookmark(ctx: Context<'_>, message: serenity::Message) -> Result<(), Error> {
    let bookmark = crate::models::bookmark::BookmarkedMessage::new(
        uuid::Uuid::new_v7(uuid::Timestamp::now(ctx.data().uuid_context.as_ref())),
        ctx.author().id.get(),
        message,
    );

    let inserted_bookmark =
        crate::database::bookmark::insert_bookmark(&ctx.data().db_connection, bookmark).await;

    let (dm_message, message_reply) = match inserted_bookmark {
        Ok(bookmark) => {
            let channel_name = bookmark.message().channel_id.name(ctx.http()).await?;

            (
                Some(get_bookmark_message(
                    &bookmark,
                    &channel_name,
                    DELETE_MESSAGE_EMOJI,
                )),
                get_bookmark_created_reply(),
            )
        }
        Err(InsertBookmarkError::BookmarkAlreadyExists(bookmark)) => {
            let channel_name = bookmark.message().channel_id.name(ctx.http()).await?;

            (
                Some(get_bookmark_message(
                    &bookmark,
                    &channel_name,
                    DELETE_MESSAGE_EMOJI,
                )),
                get_bookmark_already_exists_reply(),
            )
        }
        Err(other) => {
            eprintln!("Failed to insert bookmark: {:?}", other);

            (None, get_failed_to_create_bookmark_reply())
        }
    };

    async fn send_message(
        ctx: &Context<'_>,
        dm_message: Option<CreateMessage>,
    ) -> Result<(), Error> {
        if let Some(dm_message) = dm_message {
            ctx.author()
                .create_dm_channel(&ctx.http())
                .await?
                .send_message(&ctx.http(), dm_message)
                .await?;
        }
        Ok(())
    }

    async fn send_reply(ctx: &Context<'_>, message_reply: CreateReply) -> Result<(), Error> {
        ctx.send(message_reply).await?;
        Ok(())
    }

    let _ = tokio::try_join!(
        send_message(&ctx, dm_message),
        send_reply(&ctx, message_reply)
    );

    Ok(())
}
