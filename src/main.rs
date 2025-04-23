mod commands;
mod database;
mod models;

use chrono::TimeDelta;
use commands::{
    get_delete_button, get_discord_relative_timestamp_string, ReminderSelectMenuValue,
    DELETE_MESSAGE_EMOJI, DELETE_MESSAGE_INTERACTION_CUSTOM_ID, SET_REMINDER_INTERACTION_CUSTOM_ID,
};
use database::{
    bookmark::create_bookmarks_table_if_nonexistent,
    reminder::{create_reminders_table_if_nonexistent, delete_reminder_by_id, get_all_reminders},
};
use poise::{
    samples::create_application_commands,
    serenity_prelude::{
        self as serenity, ComponentInteractionDataKind, CreateInteractionResponseMessage,
    },
};
use std::{cmp::Reverse, collections::BinaryHeap, env, sync::Arc};
use tokio::sync::Mutex;
use tokio_rusqlite::Connection;
use uuid::Uuid;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    db_connection: Arc<Mutex<Connection>>,
    tx: tokio::sync::mpsc::Sender<models::reminder::PersistedReminder>,
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = dotenvy::dotenv(); // Am discarding the result because I don't actually care if there isn't a literal .env file as long as the environment variable is set

    let (serenity_commands, all_commands) = {
        let commands = vec![commands::help()];
        let commands_available_in_dms = vec![
            commands::get_reminders(),
            commands::remind_me_in_10_seconds(),
            commands::remind_me_in(),
            commands::bookmark(),
        ];

        let serenity_commands = [
            create_application_commands(&commands),
            create_application_commands(&commands_available_in_dms)
                .into_iter()
                .map(|command| command.add_context(serenity::InteractionContext::PrivateChannel))
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
            Box::pin(async move {
                println!("Received event: {}", event.snake_case_name());

                match event {
                    serenity::FullEvent::ReactionAdd { add_reaction } => {
                        let is_reactor_not_the_bot = add_reaction.user_id != Some(framework.bot_id);
                        let is_message_from_the_bot =
                            add_reaction.message_author_id == Some(framework.bot_id);
                        let is_reaction_emoji_delete =
                            add_reaction.emoji.unicode_eq(DELETE_MESSAGE_EMOJI);

                        if is_reactor_not_the_bot
                            && is_message_from_the_bot
                            && is_reaction_emoji_delete
                        {
                            ctx.http
                                .delete_message(
                                    add_reaction.channel_id,
                                    add_reaction.message_id,
                                    Some(&format!(
                                        "Deletion requested by: {}",
                                        add_reaction
                                            .message_author_id
                                            .map_or("Unknown".to_string(), |id| id.to_string())
                                    )),
                                )
                                .await?;
                        }
                    }
                    serenity::FullEvent::InteractionCreate { interaction } => {
                        println!("Received interaction: {:?}", interaction);
                        if let Some(component_interaction) = interaction.as_message_component() {
                            match InteractionCustomId::try_from(
                                &component_interaction.data.custom_id[..],
                            ) {
                                Ok(InteractionCustomId::DeleteMessage) => {
                                    ctx.http
                                        .delete_message(
                                            component_interaction.channel_id,
                                            component_interaction.message.id,
                                            Some(&format!(
                                                "Deletion requested by: {}",
                                                component_interaction.user.id
                                            )),
                                        )
                                        .await?;
                                }
                                Ok(InteractionCustomId::SetReminder(bookmark_id)) => {
                                    let db_connection = data.db_connection.clone();
                                    let bookmark = database::bookmark::get_bookmark_by_id(
                                        &db_connection,
                                        bookmark_id,
                                    )
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
                                                ReminderSelectMenuValue::try_from(
                                                    selected_value.as_str(),
                                                )
                                                .unwrap_or_else(|err| {
                                                    panic!(
                                                        "Failed to parse selected value: {}",
                                                        err
                                                    )
                                                }),
                                            );
                                            assert!(
                                                component_interaction.user.id == bookmark.user_id(),
                                                "Expected user ID to match bookmark user ID"
                                            );
                                            let remind_at =
                                                chrono::Utc::now() + reminder_wait_duration;
                                            let reminder = models::reminder::Reminder::new(
                                                bookmark.user_id(),
                                                bookmark.message().clone(),
                                                remind_at,
                                            );
                                            let persisted_reminder =
                                                database::reminder::insert_reminder(
                                                    &db_connection,
                                                    reminder,
                                                )
                                                .await?;
                                            data.tx.send(persisted_reminder).await?;
                                            component_interaction
                                                .create_response(
                                                    &ctx.http,
                                                    serenity::CreateInteractionResponse::Message(
                                                        CreateInteractionResponseMessage::new()
                                                            .content(format!(
                                                                "Reminder set for {}",
                                                                get_discord_relative_timestamp_string(&remind_at)
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
            })
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
                serenity::Command::set_global_commands(ctx.http.clone(), serenity_commands.clone())
                    .await?;

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
            .map(|r| *r.0.remind_at() - chrono::Utc::now())
            .map(|duration: TimeDelta| duration.max(TimeDelta::zero()))
            .unwrap_or(TimeDelta::nanoseconds(i64::MAX))
            .to_std()?;

        tokio::select! {
            _ = tokio::time::sleep(sleep_time) => {
                if let Some(reminder) = next_reminder {
                    let user_id = serenity::UserId::new(reminder.0.user_id());

                    let message = serenity::CreateMessage::default()
                        .embed(reminder.0.to_embed(&http).await)
                        .button(get_delete_button());

                    user_id
                        .create_dm_channel(&http)
                        .await?
                        .send_message(&http, message)
                        .await?;

                    delete_reminder_by_id(&db_connection, reminder.0.pk())
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

/*
Things I need to do
1. Create 'Bookmarks' table - autoinc int pk, text (uuid) bookmark_id, text (u64) user_id, text (json) message
2. Create context menu command 'Bookmark' - generates random bookmark_id + adds bookmark to table; sends user dm with link to message (looks like the current reminder embed) + select menu asking if they want to be reminded.
custom_data_id should contain the bookmark_id
3. Upon receiving an interaction event, get the bookmark_id from the custom_data_id and get the bookmark from the database, create reminder, and insert it into table + heap
*/

enum InteractionCustomId {
    DeleteMessage,
    SetReminder(Uuid),
}

impl From<InteractionCustomId> for String {
    fn from(value: InteractionCustomId) -> Self {
        match value {
            InteractionCustomId::DeleteMessage => DELETE_MESSAGE_INTERACTION_CUSTOM_ID.to_string(),
            InteractionCustomId::SetReminder(uuid) => {
                format!("{}:{}", SET_REMINDER_INTERACTION_CUSTOM_ID, uuid)
            }
        }
    }
}

impl TryFrom<&str> for InteractionCustomId {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = value.split(':').collect();
        match parts.as_slice() {
            [DELETE_MESSAGE_INTERACTION_CUSTOM_ID] => Ok(Self::DeleteMessage),
            [SET_REMINDER_INTERACTION_CUSTOM_ID, uuid] => {
                let uuid = Uuid::parse_str(uuid).map_err(|_| {
                    format!(
                        "Received invalid UUID for {}: {}",
                        SET_REMINDER_INTERACTION_CUSTOM_ID, uuid
                    )
                })?;
                Ok(Self::SetReminder(uuid))
            }
            _ => Err(format!("Received invalid custom ID: {}", value)),
        }
    }
}
