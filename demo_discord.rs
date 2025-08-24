use solana_arbitrage_bot::discord::DiscordAlert;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Discord Alerts Demo");

    let webhook = std::env::var("DISCORD_WEBHOOK_URL")
        .expect("Set DISCORD_WEBHOOK_URL to test Discord alerts");
    let enabled = std::env::var("DISCORD_ENABLED")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"))
        .unwrap_or(true);

    let discord = DiscordAlert::new(webhook, enabled);

    println!("📢 Sending startup alert...");
    discord
        .send_startup_alert(
            "GedVmbHnUpRoqxWSxLwDMQNY5bmggTjRojoCY6u31VGS",
            "devnet",
            "DEMO MODE",
        )
        .await?;

    sleep(Duration::from_secs(2)).await;

    println!("📊 Sending price update...");
    discord.send_price_update(150.2341, 150.8923).await?;

    sleep(Duration::from_secs(2)).await;

    println!("🎯 Sending opportunity alert...");
    discord
        .send_opportunity_alert(150.2341, 150.8923, 0.43, 75)
        .await?;

    sleep(Duration::from_secs(2)).await;

    println!("💰 Sending profit alert...");
    discord
        .send_profit_alert(
            3.45,
            "5xKr9nQjH8mVfLZGpJxQ2wN3vR7sT1uY6bC4dE8fG9hJ",
            "Raydium",
            "Orca",
            0.1,
        )
        .await?;

    sleep(Duration::from_secs(2)).await;

    println!("🚨 Sending error alert...");
    discord
        .send_error_alert(
            "Transaction simulation failed: insufficient funds",
            "Trade Execution",
        )
        .await?;

    println!("✅ All Discord alerts sent! Check your Discord channel!");

    Ok(())
}
