mod commands;

use poise::{samples::create_application_commands, serenity_prelude as serenity};
use std::{env, sync::Arc, time::Duration};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv(); // Am discarding the result because I don't actually care if there isn't a literal .env file as long as the environment variable is set

    let (serenity_commands, all_commands) = {
        let commands = vec![commands::help()];
        let commands_available_in_dms = vec![commands::remind_me_context_menu_command()];

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
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            ..Default::default()
        },
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
        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.author().id == 123456789 {
                    return Ok(false);
                }
                Ok(true)
            })
        }),
        skip_checks_for_owners: false,
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                println!(
                    "Got an event in event handler: {:?}",
                    event.snake_case_name()
                );
                Ok(())
            })
        },
        ..Default::default()
    };

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
                Ok(Data {})
            })
        })
        .options(options)
        .build();

    const DISCORD_TOKEN: &str = "DISCORD_TOKEN";
    let token = env::var(DISCORD_TOKEN)
        .unwrap_or_else(|_| panic!("Environment variable `{}` must be set", DISCORD_TOKEN));
    let intents = serenity::GatewayIntents::non_privileged();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap()
}
