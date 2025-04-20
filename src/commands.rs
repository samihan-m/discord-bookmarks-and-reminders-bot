use crate::{
    database::{get_reminders_for_user, insert_reminder},
    models::{ParseReminderError, Reminder},
};

use poise::{
    serenity_prelude::{self as serenity, CreateButton, CreateEmbed},
    CreateReply,
};

use crate::{Context, Error};

pub const DELETE_MESSAGE_EMOJI: &str = "🗑️";
pub const DELETE_MESSAGE_INTERACTION_CUSTOM_ID: &str = "delete_reminder";

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

fn create_get_reminders_reply(reminders: &[Result<Reminder, ParseReminderError>]) -> CreateReply {
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
                .fields(reminders.iter().filter_map(|reminder| match reminder {
                    Err(_) => None,
                    Ok(reminder) => {
                        let field_name = format!(
                            "{} at: {}",
                            reminder.message().link(),
                            get_discord_relative_timestamp_string(reminder.remind_at())
                        );
                        const MAX_FIELD_NAME_LENGTH: usize = 256;
                        let trimmed_field_name =
                            &field_name[..field_name.len().min(MAX_FIELD_NAME_LENGTH)];
                        Some((trimmed_field_name.to_owned(), "", true))
                    }
                }))
                .colour(serenity::Colour::TEAL),
        )
        .ephemeral(true)
}

pub async fn add_reminder(
    ctx: &Context<'_>,
    user_id: u64,
    message: serenity::Message,
    remind_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), Error> {
    let reminder = insert_reminder(&ctx.data().db_connection, user_id, message, remind_at).await?;

    ctx.data().tx.send(reminder).await?;

    Ok(())
}

#[poise::command(context_menu_command = "Remind me in 10 seconds")]
pub async fn remind_me_in_10_seconds(
    ctx: Context<'_>,
    message: serenity::Message,
) -> Result<(), Error> {
    let remind_at = chrono::Utc::now() + chrono::Duration::seconds(10);

    add_reminder(&ctx, ctx.author().id.get(), message, remind_at).await?;

    ctx.send(get_reminder_created_reply(&remind_at)).await?;

    Ok(())
}

#[poise::command(context_menu_command = "Remind me in 1 hour")]
pub async fn remind_me_in_1_hour(
    ctx: Context<'_>,
    message: serenity::Message,
) -> Result<(), Error> {
    let remind_at = chrono::Utc::now() + chrono::Duration::hours(1);

    add_reminder(&ctx, ctx.author().id.get(), message, remind_at).await?;

    ctx.send(get_reminder_created_reply(&remind_at)).await?;

    Ok(())
}

#[poise::command(context_menu_command = "Remind me in 24 hour")]
pub async fn remind_me_in_24_hours(
    ctx: Context<'_>,
    message: serenity::Message,
) -> Result<(), Error> {
    let remind_at = chrono::Utc::now() + chrono::Duration::hours(24);

    add_reminder(&ctx, ctx.author().id.get(), message, remind_at).await?;

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

    add_reminder(&ctx, ctx.author().id.get(), message, remind_at).await?;

    ctx.send(
        CreateReply::default()
            .content(format!("Reminder set for {} seconds from now!", seconds))
            .reply(true)
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

fn get_discord_relative_timestamp_string(remind_at: &chrono::DateTime<chrono::Utc>) -> String {
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

        let reminders = vec![Ok(Reminder::new(
            1,
            123456789,
            serenity::Message::default(),
            timestamp,
        ))];
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
