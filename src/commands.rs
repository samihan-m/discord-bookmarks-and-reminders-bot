use std::time::Duration;

use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, CreateMessage},
    CreateReply,
};
use tokio::time::sleep;

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

/// Set a reminder
#[poise::command(context_menu_command = "Remind Me")]
pub async fn remind_me_context_menu_command(
    ctx: Context<'_>,
    message: serenity::Message,
) -> Result<(), Error> {
    let wait_duration = Duration::from_secs(10);

    tokio::try_join!(
        async {
            match ctx.guild_channel().await {
                Some(_) => {
                    message.react(ctx.http(), '⏰').await?;
                }
                None => {
                    // If the command was run in a DM, we can't react at the moment (don't have authorization or whatever)
                    // without causing an error, so just skip it
                }
            };
            Ok(())
        },
        async {
            ctx.send(
                CreateReply::default()
                    .content(format!(
                        "Reminder set! I will remind you in {} seconds.",
                        wait_duration.as_secs()
                    ))
                    .reply(true)
                    .ephemeral(true),
            )
            .await
        },
        async {
            sleep(wait_duration).await;
            Ok(())
        }
    )?;

    let title = format!(
        "Reminder from {}",
        ctx.channel_id()
            .name(ctx.http())
            // This will error if we don't have permission to get DM channel information (which we currently do not)
            .await
            .map(|name| format!("[#{}]({})", name, message.link()))
            .unwrap_or("the past!".to_string())
    );
    const MAX_TITLE_LENGTH: usize = 256;
    let trimmed_title = &title[..title.len().min(MAX_TITLE_LENGTH)];

    let description = format!(
        r#"
        # {}
        # {}
    "#,
        message.content,
        message.link()
    );
    const MAX_DESCRIPTION_LENGTH: usize = 4096;
    let trimmed_description = &description[..description.len().min(MAX_DESCRIPTION_LENGTH)];

    ctx.author()
        .create_dm_channel(ctx.http())
        .await?
        .send_message(
            ctx.http(),
            CreateMessage::default().add_embed(
                CreateEmbed::default()
                    .title(trimmed_title)
                    .description(trimmed_description)
                    .timestamp(message.timestamp)
                    .colour(serenity::Colour::TEAL),
            ),
        )
        .await?;

    Ok(())
}
