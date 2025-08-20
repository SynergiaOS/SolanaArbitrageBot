use solana_arbitrage_bot::discord::DiscordAlert;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Discord Alerts Demo");
    
    let discord = DiscordAlert::new(
        "https://discordapp.com/api/webhooks/1407094459493122158/9Ju35aOQQk5MrqRe4Qj4hx6Y3lPmuq7zfswplBqtB34WBJ-2fXnRSq8C1kaP0VCY8nqG".to_string(),
        true
    );
    
    println!("📢 Sending startup alert...");
    discord.send_startup_alert(
        "GedVmbHnUpRoqxWSxLwDMQNY5bmggTjRojoCY6u31VGS",
        "devnet",
        "DEMO MODE"
    ).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    println!("📊 Sending price update...");
    discord.send_price_update(150.2341, 150.8923).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    println!("🎯 Sending opportunity alert...");
    discord.send_opportunity_alert(
        150.2341,
        150.8923,
        0.43,
        75
    ).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    println!("💰 Sending profit alert...");
    discord.send_profit_alert(
        3.45,
        "5xKr9nQjH8mVfLZGpJxQ2wN3vR7sT1uY6bC4dE8fG9hJ",
        "Raydium",
        "Orca",
        0.1
    ).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    println!("🚨 Sending error alert...");
    discord.send_error_alert(
        "Transaction simulation failed: insufficient funds",
        "Trade Execution"
    ).await?;
    
    println!("✅ All Discord alerts sent! Check your Discord channel!");
    
    Ok(())
}
