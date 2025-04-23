use crate::{
    database::{
        bookmark::InsertBookmarkError,
        reminder::{get_reminders_for_user, insert_reminder},
    },
    models::reminder::{PersistedReminder, Reminder},
    InteractionCustomId,
};

use poise::{
    serenity_prelude::{self as serenity, CreateButton, CreateEmbed, CreateMessage},
    CreateReply,
};
use uuid::Uuid;

use crate::{Context, Error};

pub const DELETE_MESSAGE_EMOJI: &str = "🗑️";
pub const DELETE_MESSAGE_INTERACTION_CUSTOM_ID: &str = "delete_message";
pub const SET_REMINDER_INTERACTION_CUSTOM_ID: &str = "set_reminder";

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
        ctx.send(
            CreateReply::default()
                .content("No reminders found in the database.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.send(create_get_reminders_reply(&reminders)).await?;

    Ok(())
}

fn create_get_reminders_reply(reminders: &[PersistedReminder]) -> CreateReply {
    let title = format!(
        "Retrieved up to {} reminder{}.\nThere may be more reminders not shown.",
        reminders.len(),
        if reminders.len() > 1 { "s" } else { "" }
    );
    const MAX_TITLE_LENGTH: usize = 256;
    let trimmed_title = &title[..title.len().min(MAX_TITLE_LENGTH)];

    let description = format!("## Queued Reminders: {}", reminders.len());
    const MAX_DESCRIPTION_LENGTH: usize = 4096;
    let trimmed_description = &description[..description.len().min(MAX_DESCRIPTION_LENGTH)];

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
                    const MAX_FIELD_NAME_LENGTH: usize = 256;
                    let trimmed_field_name =
                        &field_name[..field_name.len().min(MAX_FIELD_NAME_LENGTH)];
                    (trimmed_field_name.to_owned(), "", true)
                }))
                .colour(serenity::Colour::TEAL),
        )
        .ephemeral(true)
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

#[poise::command(context_menu_command = "Remind me in 1 hour")]
pub async fn remind_me_in_1_hour(
    ctx: Context<'_>,
    message: serenity::Message,
) -> Result<(), Error> {
    let remind_at = chrono::Utc::now() + chrono::Duration::hours(1);

    let reminder = Reminder::new(ctx.author().id.get(), message, remind_at);

    add_reminder(&ctx, reminder).await?;

    ctx.send(get_reminder_created_reply(&remind_at)).await?;

    Ok(())
}

#[poise::command(context_menu_command = "Remind me in 24 hours")]
pub async fn remind_me_in_24_hours(
    ctx: Context<'_>,
    message: serenity::Message,
) -> Result<(), Error> {
    let remind_at = chrono::Utc::now() + chrono::Duration::hours(24);

    let reminder = Reminder::new(ctx.author().id.get(), message, remind_at);
    add_reminder(&ctx, reminder).await?;

    ctx.send(get_reminder_created_reply(&remind_at)).await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn remind_me_in(
    ctx: Context<'_>,
    #[description = "Time to wait before sending the reminder"]
    #[min = 1]
    #[max = 60]
    seconds: u64,
    #[description = "Message to remind you of"] message: serenity::Message,
) -> Result<(), Error> {
    let remind_at = chrono::Utc::now() + chrono::Duration::seconds(seconds as i64);

    let reminder = Reminder::new(ctx.author().id.get(), message, remind_at);
    add_reminder(&ctx, reminder).await?;

    ctx.send(
        CreateReply::default()
            .content(format!("Reminder set for {} seconds from now!", seconds))
            .reply(true)
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

#[poise::command(context_menu_command = "Bookmark")]
pub async fn bookmark(ctx: Context<'_>, message: serenity::Message) -> Result<(), Error> {
    let bookmark = crate::models::bookmark::BookmarkedMessage::new(
        uuid::Uuid::new_v4(),
        ctx.author().id.get(),
        message,
    );

    let inserted_bookmark =
        crate::database::bookmark::insert_bookmark(&ctx.data().db_connection, bookmark).await;

    let (dm_message, message_reply) = match inserted_bookmark {
        Ok(bookmark) => {
            let embed = bookmark.to_embed(ctx.http()).await;
            let reminder_select_menu = get_reminder_select_menu(bookmark.bookmark_id());

            (
                Some(
                    CreateMessage::default()
                        .embed(embed)
                        .select_menu(reminder_select_menu),
                ),
                CreateReply::default()
                    .content("Bookmark created!")
                    .ephemeral(true)
                    .reply(true),
            )
        }
        Err(InsertBookmarkError::BookmarkAlreadyExists(bookmark)) => {
            let embed = bookmark.to_embed(ctx.http()).await;
            let reminder_select_menu = get_reminder_select_menu(bookmark.bookmark_id());

            (
                Some(
                    CreateMessage::default()
                        .embed(embed)
                        .select_menu(reminder_select_menu),
                ),
                CreateReply::default()
                    .content("Bookmark already exists!")
                    .ephemeral(true)
                    .reply(true),
            )
        }
        Err(other) => {
            eprintln!("Failed to insert bookmark: {:?}", other);

            (
                None,
                CreateReply::default()
                    .content("Failed to create bookmark.")
                    .ephemeral(true),
            )
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

pub fn get_discord_relative_timestamp_string(remind_at: &chrono::DateTime<chrono::Utc>) -> String {
    format!("<t:{}:R>", remind_at.timestamp())
}

fn get_reminder_created_reply(remind_at: &chrono::DateTime<chrono::Utc>) -> CreateReply {
    CreateReply::default()
        .content(format!(
            "Reminder set for {}",
            get_discord_relative_timestamp_string(remind_at)
        ))
        .reply(true)
        .ephemeral(true)
}

pub fn get_delete_button() -> CreateButton {
    CreateButton::new(DELETE_MESSAGE_INTERACTION_CUSTOM_ID)
        .label("Delete")
        .emoji(serenity::ReactionType::Unicode(
            DELETE_MESSAGE_EMOJI.to_string(),
        ))
        .style(serenity::ButtonStyle::Danger)
}

pub enum ReminderSelectMenuValue {
    TenSeconds,
    OneHour,
    TwentyFourHours,
}

// TODO: Write tests for this
impl From<ReminderSelectMenuValue> for String {
    fn from(value: ReminderSelectMenuValue) -> Self {
        match value {
            ReminderSelectMenuValue::TenSeconds => "10",
            ReminderSelectMenuValue::OneHour => "3600",
            ReminderSelectMenuValue::TwentyFourHours => "86400",
        }
        .to_string()
    }
}

// TODO: Write tests for this
impl TryFrom<&str> for ReminderSelectMenuValue {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "10" => Ok(Self::TenSeconds),
            "3600" => Ok(Self::OneHour),
            "86400" => Ok(Self::TwentyFourHours),
            _ => Err(format!(
                "Invalid value for ReminderSelectMenuValue: {}",
                value
            )),
        }
    }
}

impl From<ReminderSelectMenuValue> for chrono::Duration {
    fn from(value: ReminderSelectMenuValue) -> Self {
        match value {
            ReminderSelectMenuValue::TenSeconds => chrono::Duration::seconds(10),
            ReminderSelectMenuValue::OneHour => chrono::Duration::hours(1),
            ReminderSelectMenuValue::TwentyFourHours => chrono::Duration::hours(24),
        }
    }
}

// TODO: Write tests for this
fn get_reminder_select_menu(bookmark_id: Uuid) -> serenity::CreateSelectMenu {
    serenity::CreateSelectMenu::new(
        InteractionCustomId::SetReminder(bookmark_id),
        serenity::CreateSelectMenuKind::String {
            options: vec![
                serenity::CreateSelectMenuOption::new(
                    "In 10 seconds",
                    ReminderSelectMenuValue::TenSeconds,
                ),
                serenity::CreateSelectMenuOption::new(
                    "In 1 hour",
                    ReminderSelectMenuValue::OneHour,
                ),
                serenity::CreateSelectMenuOption::new(
                    "In 24 hours",
                    ReminderSelectMenuValue::TwentyFourHours,
                ),
            ],
        },
    )
    .min_values(1)
    .max_values(1)
    .placeholder("When would you like to be reminded?")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_discord_relative_timestamp_string() {
        let remind_at = chrono::Utc::now();
        let result = get_discord_relative_timestamp_string(&remind_at);
        assert_eq!(result, format!("<t:{}:R>", remind_at.timestamp()));
    }

    #[test]
    fn test_get_reminder_created_reply() {
        let remind_at = chrono::Utc::now();
        let result = get_reminder_created_reply(&remind_at);
        assert_eq!(
            result.content.unwrap(),
            format!("Reminder set for <t:{}:R>", remind_at.timestamp())
        );
        assert_eq!(result.ephemeral, Some(true));
        assert_eq!(result.reply, true);
    }

    #[test]
    fn test_create_get_reminders_reply() {
        let timestamp = chrono::Utc::now();

        let reminders = vec![PersistedReminder::from_reminder(
            Reminder::new(123456789, serenity::Message::default(), timestamp),
            1,
        )];
        let result = create_get_reminders_reply(&reminders);
        assert_eq!(result.ephemeral, Some(true));

        let embed = result.embeds.get(0).unwrap().to_owned();
        let expected_embed = CreateEmbed::default()
            .title("Retrieved up to 1 reminder.\nThere may be more reminders not shown.")
            .description("## Queued Reminders: 1")
            .field(
                format!(
                    "https://discord.com/channels/@me/1/1 at: <t:{}:R>",
                    timestamp.timestamp()
                ),
                "",
                true,
            )
            .colour(serenity::Colour::TEAL);
        assert_eq!(embed, expected_embed);
    }
}
