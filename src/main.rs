mod commands;
mod components;
mod database;
mod models;

use components::{
    interaction_custom_id::{InteractionCustomId, DELETE_MESSAGE_INTERACTION_CUSTOM_ID},
    relative_timestamp_string::get_discord_relative_timestamp_string,
    reminder::{
        reminder_message::get_reminder_message,
        reminder_time_select_menu::menu_value::ReminderSelectMenuValue,
    },
    DELETE_MESSAGE_EMOJI,
};
use database::{
    bookmark::create_bookmarks_table_if_nonexistent,
    reminder::{create_reminders_table_if_nonexistent, delete_reminder_by_id, get_all_reminders},
};
use poise::{
    samples::create_application_commands,
    serenity_prelude::{
        self as serenity, ComponentInteractionDataKind, CreateInteractionResponseMessage, FullEvent,
    },
    FrameworkContext,
};
use std::{cmp::Reverse, collections::BinaryHeap, env, str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use tokio_rusqlite::Connection;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    db_connection: Arc<Mutex<Connection>>,
    tx: tokio::sync::mpsc::Sender<models::reminder::PersistedReminder>,
    uuid_context: Arc<std::sync::Mutex<uuid::ContextV7>>,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            eprintln!("Error in command `{}`: {:?}", ctx.command().name, error,);
            eprintln!("Error: {:?}", error.to_string());
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                eprintln!("Error while handling error: {}", e)
            }
        }
    }
}

async fn on_event<'a>(
    ctx: &serenity::Context,
    event: &FullEvent,
    framework: FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    println!("Received event: {}", event.snake_case_name());

    match event {
        serenity::FullEvent::ReactionAdd { add_reaction } => {
            let is_reactor_not_the_bot = add_reaction.user_id != Some(framework.bot_id);
            let is_message_from_the_bot = add_reaction.message_author_id == Some(framework.bot_id);
            let is_reaction_emoji_delete = add_reaction.emoji.unicode_eq(DELETE_MESSAGE_EMOJI);

            if is_reactor_not_the_bot && is_message_from_the_bot && is_reaction_emoji_delete {
                delete_message_with_audit_log_reason(
                    ctx.http.clone(),
                    add_reaction.channel_id,
                    add_reaction.message_id,
                    &add_reaction
                        .message_author_id
                        .map_or("Unknown".to_string(), |id| id.to_string()),
                )
                .await?;
            }
        }
        serenity::FullEvent::InteractionCreate { interaction } => {
            println!("Received interaction: {:?}", interaction);
            if let Some(component_interaction) = interaction.as_message_component() {
                match InteractionCustomId::try_from(&component_interaction.data.custom_id[..]) {
                    Ok(InteractionCustomId::DeleteMessage) => {
                        delete_message_with_audit_log_reason(
                            ctx.http.clone(),
                            component_interaction.channel_id,
                            component_interaction.message.id,
                            &component_interaction.user.name,
                        )
                        .await?;
                    }
                    Ok(InteractionCustomId::SetReminder(bookmark_id)) => {
                        let db_connection = data.db_connection.clone();
                        let bookmark =
                            database::bookmark::get_bookmark_by_id(&db_connection, bookmark_id)
                                .await?
                                .unwrap_or_else(|| {
                                    panic!(
                                        "Expected bookmark {} to be found in database",
                                        bookmark_id
                                    )
                                });
                        match &component_interaction.data.kind {
                            ComponentInteractionDataKind::StringSelect { values } => {
                                let selected_value = values
                                                .first()
                                                .unwrap_or_else(|| panic!("Expected at least one value to be selected, received: {:?}", values));
                                let reminder_wait_duration = chrono::Duration::from(
                                    ReminderSelectMenuValue::from_str(selected_value.as_str())
                                        .unwrap_or_else(|_| {
                                            panic!(
                                                "Failed to parse selected value: {}",
                                                selected_value
                                            )
                                        }),
                                );
                                assert!(
                                    component_interaction.user.id == bookmark.user_id(),
                                    "Expected user ID to match bookmark user ID"
                                );
                                let remind_at = chrono::Utc::now() + reminder_wait_duration;
                                let reminder = models::reminder::Reminder::new(
                                    bookmark.user_id(),
                                    bookmark.message().clone(),
                                    remind_at,
                                );
                                let persisted_reminder =
                                    database::reminder::insert_reminder(&db_connection, reminder)
                                        .await?;
                                data.tx.send(persisted_reminder).await?;
                                component_interaction
                                    .create_response(
                                        &ctx.http,
                                        serenity::CreateInteractionResponse::Message(
                                            CreateInteractionResponseMessage::new()
                                                .content(format!(
                                                    "Reminder set for {}",
                                                    get_discord_relative_timestamp_string(
                                                        &remind_at
                                                    )
                                                ))
                                                .ephemeral(true),
                                        ),
                                    )
                                    .await?;
                            }
                            _ => {
                                panic!("Received unexpected interaction data kind for custom id {}: {:?}", component_interaction.data.custom_id, component_interaction.data.kind);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = dotenvy::dotenv(); // Am discarding the result because I don't actually care if there isn't a literal .env file as long as the environment variable is set

    let (serenity_commands, all_commands) = {
        let commands = vec![commands::help()];
        let commands_available_in_dms = vec![
            commands::get_reminders(),
            commands::remind_me_in_10_seconds(),
            commands::bookmark(),
            commands::get_bookmarks(),
        ];

        let serenity_commands = [
            create_application_commands(&commands),
            create_application_commands(&commands_available_in_dms)
                .into_iter()
                .map(|command| command.add_context(serenity::InteractionContext::PrivateChannel).add_context(serenity::InteractionContext::BotDm))
                .collect(),
        ]
        .concat();

        (
            serenity_commands,
            commands
                .into_iter()
                .chain(commands_available_in_dms)
                .collect(),
        )
    };

    let options = poise::FrameworkOptions {
        commands: all_commands,
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        skip_checks_for_owners: false,
        event_handler: |ctx, event, framework, data| {
            Box::pin(on_event(ctx, event, framework, data))
        },
        ..Default::default()
    };

    let db_connection = Arc::new(Mutex::new(Connection::open("./data.db").await.unwrap()));
    let _ = tokio::try_join!(
        create_reminders_table_if_nonexistent(&db_connection),
        create_bookmarks_table_if_nonexistent(&db_connection),
    )?;
    let reminders_from_database = get_all_reminders(&db_connection)
        .await?
        .into_iter()
        .map(Reverse)
        .collect::<Vec<_>>();

    let reminders_heap = BinaryHeap::from(reminders_from_database);

    let (tx, rx) = tokio::sync::mpsc::channel(32);

    let db_connection_clone = db_connection.clone();
    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, _framework| {
            Box::pin(async move {
                #[cfg(debug_assertions)]
                {
                    println!("Setting global commands...");
                    serenity::Command::set_global_commands(ctx.http.clone(), serenity_commands.clone())
                        .await?;
                }

                const TEST_GUILD_ID: &str = "TEST_GUILD_ID";

                match env::var(TEST_GUILD_ID) {
                    Ok(guild_id) => {
                        let guild_id = serenity::GuildId::new(
                            guild_id
                                .parse::<u64>()
                                .unwrap_or_else(|_| panic!("Environment variable `{}` must be a valid u64", TEST_GUILD_ID))
                        );
                        // This will make each command appear twice in the command list specifically only for the test guild
                        guild_id
                            .set_commands(ctx.http.clone(), serenity_commands)
                            .await?;
                    }
                    Err(env::VarError::NotPresent) => {
                        println!(
                            "Environment variable `{}` not set, skipping guild command registration",
                            TEST_GUILD_ID
                        );
                    }
                    Err(env::VarError::NotUnicode(_)) => {
                        panic!("Environment variable `{}` must be a valid UTF-8 string", TEST_GUILD_ID);
                    }
                }

                println!("Logged in as {}", ready.user.name);
                Ok(Data {
                    db_connection: db_connection_clone,
                    tx,
                    uuid_context: Arc::new(std::sync::Mutex::new(uuid::ContextV7::new())),
                })
            })
        })
        .options(options)
        .build();

    const DISCORD_TOKEN: &str = "DISCORD_TOKEN";
    let token = env::var(DISCORD_TOKEN)
        .unwrap_or_else(|_| panic!("Environment variable `{}` must be set", DISCORD_TOKEN));
    let intents = serenity::GatewayIntents::non_privileged();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .unwrap();

    tokio::spawn(send_reminders(
        client.http.clone(),
        reminders_heap,
        db_connection,
        rx,
    ));

    client.start().await.unwrap();

    Ok(())
}

async fn send_reminders(
    http: Arc<serenity::Http>,
    mut reminders: BinaryHeap<Reverse<models::reminder::PersistedReminder>>,
    db_connection: Arc<Mutex<Connection>>,
    mut rx: tokio::sync::mpsc::Receiver<models::reminder::PersistedReminder>,
) -> Result<(), Error> {
    loop {
        println!("{} reminders in the heap.", reminders.len());
        let next_reminder = reminders.pop();

        let sleep_time = next_reminder
            .as_ref()
            .map(|r| &r.0)
            .map(models::reminder::PersistedReminder::get_sleep_time_until_reminder_should_trigger)
            .unwrap_or(std::time::Duration::from_secs(u64::MAX));

        tokio::select! {
            _ = tokio::time::sleep(sleep_time) => {
                if let Some(Reverse(reminder)) = next_reminder {
                    let channel_name = &reminder.message().channel_id.name(&http).await.unwrap_or("the past!".to_string());

                    let message = get_reminder_message(
                        &reminder,
                        channel_name,
                        DELETE_MESSAGE_INTERACTION_CUSTOM_ID,
                        DELETE_MESSAGE_EMOJI
                    );

                    let user_id = serenity::UserId::new(reminder.user_id());

                    user_id
                        .create_dm_channel(&http)
                        .await?
                        .send_message(&http, message)
                        .await?;

                    delete_reminder_by_id(&db_connection, reminder.pk())
                        .await?;
                }
            }
            Some(reminder) = rx.recv() => {
                println!("Received reminder: {:?}", reminder);
                if let Some(next_reminder) = next_reminder {
                    reminders.push(next_reminder);
                }
                reminders.push(Reverse(reminder));
            }
        }
    }
}

async fn delete_message_with_audit_log_reason(
    http: Arc<serenity::Http>,
    channel_id: serenity::ChannelId,
    message_id: serenity::MessageId,
    responsible_user_name: &str,
) -> Result<(), Error> {
    http.delete_message(
        channel_id,
        message_id,
        Some(&get_delete_message_audit_log_reason(responsible_user_name)),
    )
    .await?;

    Ok(())
}

fn get_delete_message_audit_log_reason(responsible_user_name: &str) -> String {
    format!("Deletion requested by: {}", responsible_user_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_delete_message_audit_log_reason() {
        let responsible_user_name = "TestUser";
        assert_eq!(
            get_delete_message_audit_log_reason(responsible_user_name),
            "Deletion requested by: TestUser"
        );
    }
}
