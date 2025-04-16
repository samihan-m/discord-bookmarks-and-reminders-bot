use std::env;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(error) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {error:?}");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv(); // I don't actually care if there isn't a literal .env file as long as the environment variable is set
    
    let token = env::var("DISCORD_TOKEN").expect("Expected `DISCORD_TOKEN` in the environment");
    
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    if let Err(error) = client.start().await {
        println!("Client error: {error:?}");
    }
}
