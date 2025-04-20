mod commands;
mod database;
mod models;

use chrono::TimeDelta;
use database::{create_table_if_nonexistent, delete_reminder_by_id, get_all_reminders};
use poise::{
    samples::create_application_commands,
    serenity_prelude::{self as serenity, CreateMessage},
};
use std::{cmp::Reverse, collections::BinaryHeap, env, sync::Arc};
use tokio::sync::Mutex;
use tokio_rusqlite::Connection;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    db_connection: Arc<Mutex<Connection>>,
    tx: tokio::sync::mpsc::Sender<models::Reminder>,
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
            commands::remind_me_in_1_hour(),
            commands::remind_me_in_3_hours(),
            commands::remind_me_in_6_hours(),
            commands::remind_me_in_12_hours(),
            commands::remind_me_in_24_hours(),
            commands::remind_me_in(),
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
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                println!("Received event: {}", event.snake_case_name());
                Ok(())
            })
        },
        ..Default::default()
    };

    let db_connection = Arc::new(Mutex::new(
        Connection::open("./reminders.db").await.unwrap(),
    ));
    create_table_if_nonexistent(&db_connection).await?;
    let reminders_from_database = get_all_reminders(&db_connection)
        .await?
        .into_iter()
        .filter_map(Result::ok)
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
    mut reminders: BinaryHeap<Reverse<models::Reminder>>,
    db_connection: Arc<Mutex<Connection>>,
    mut rx: tokio::sync::mpsc::Receiver<models::Reminder>,
) -> Result<(), Error> {
    loop {
        println!("{} reminders in the heap.", reminders.len());
        let next_reminder = reminders.pop();

        let sleep_time = next_reminder
            .as_ref()
            .map(|r| *r.0.remind_at() - chrono::Utc::now())
            .map(|duration| duration.max(TimeDelta::zero()))
            .unwrap_or(TimeDelta::nanoseconds(i64::MAX));

        tokio::select! {
            _ = tokio::time::sleep(sleep_time.to_std()?) => {
                if let Some(reminder) = next_reminder {
                    let user_id = serenity::UserId::new(reminder.0.user_id());

                    let message = CreateMessage::default()
                        .embed(reminder.0.to_embed(&http).await);

                    user_id
                        .create_dm_channel(&http)
                        .await?
                        .send_message(&http, message)
                        .await?;

                    delete_reminder_by_id(&db_connection, reminder.0.id())
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
