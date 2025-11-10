use std::env;

use anyhow::{Context as AnyhowContext, Result};
use dotenvy::dotenv;
use serenity::{all::*, async_trait};
use tracing::{error, info};

struct Handler {
    channel_id: ChannelId,
    target: String,
    target_user: UserId,
    webhook_url: String,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        // Only process messages from the configured channel
        match msg.channel_id == self.channel_id && msg.author.id == self.target_user {
            true => {
                if msg.content.trim() == self.target {
                    info!(
                        channel = %msg.channel_id,
                        author = %msg.author.name,
                        "Target message detected"
                    );

                    // Send GET request to webhook
                    let client = reqwest::Client::new();
                    match client.get(&self.webhook_url).send().await {
                        Ok(resp) => {
                            info!(
                                status = %resp.status(),
                                "Webhook request sent successfully"
                            );
                            info!(response = ?resp, "Response");
                            let mesg: String = resp.text().await.unwrap_or_default();
                            let response = msg.channel_id.say(&_ctx.http, mesg).await;
                            msg.react(&_ctx.http, ReactionType::Unicode("✅".into()))
                                .await
                                .expect("TODO: panic message");
                            info!(response = ?response, "Sent response");
                        }
                        Err(e) => {
                            msg.react(&_ctx.http, ReactionType::Unicode("❌".into()))
                                .await;
                            error!(?e, "Failed to send webhook request");
                        }
                    }
                }
            }
            false => {
                msg.react(&_ctx.http, ReactionType::Unicode("❌".into()))
                    .await
                    .expect("TODO: panic message");
                error!(?false, "User not matched ebago");
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().context("Failed to load .env file")?;

    let token = env::var("DISCORD_TOKEN")?;
    let channel_id: u64 = env::var("CHANNEL_ID")?.parse()?;
    let target = env::var("TARGET_MESSAGE")?;
    let target_user = env::var("TARGET_USER_ID")?.parse()?;
    let webhook_url = env::var("WEBHOOK_URL")?;

    // Initialize logging: default to INFO if RUST_LOG is not set
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    // Read configuration from environment variables

    let channel_id = ChannelId::new(channel_id);
    let target_user = UserId::new(target_user);

    // Gateway intents: we need MESSAGE_CONTENT to read message text.
    // Ensure you enable the "Message Content Intent" for your bot in the Discord Developer Portal.
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let handler = Handler {
        channel_id,
        target,
        target_user,
        webhook_url,
    };

    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .await
        .context("Error creating Discord client")?;

    info!("Bot starting... listening for target message in the configured channel");

    if let Err(why) = client.start().await {
        error!(?why, "Client ended with error");
    }

    Ok(())
}
