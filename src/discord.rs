use anyhow::Result;
use log::{error, info, warn};
use reqwest;
use serde_json::json;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone)]
pub struct DiscordAlert {
    webhook_url: String,
    enabled: bool,
    client: reqwest::Client,
}

impl DiscordAlert {
    pub fn new(webhook_url: String, enabled: bool) -> Self {
        Self {
            webhook_url,
            enabled,
            client: reqwest::Client::new(),
        }
    }

    pub async fn send_startup_alert(
        &self,
        wallet_address: &str,
        network: &str,
        mode: &str,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let message = json!({
            "embeds": [{
                "title": "🚀 Solana Arbitrage Bot Started",
                "color": 3066993, // Green
                "fields": [
                    {
                        "name": "💳 Wallet",
                        "value": format!("`{}...{}`", &wallet_address[..8], &wallet_address[wallet_address.len()-8..]),
                        "inline": true
                    },
                    {
                        "name": "🌐 Network",
                        "value": network,
                        "inline": true
                    },
                    {
                        "name": "🏃 Mode",
                        "value": mode,
                        "inline": true
                    }
                ],
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "footer": {
                    "text": "Solana Arbitrage Bot v2.0"
                }
            }]
        });

        self.send_webhook(message).await
    }

    pub async fn send_profit_alert(
        &self,
        profit_usd: f64,
        signature: &str,
        buy_dex: &str,
        sell_dex: &str,
        amount_sol: f64,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let color = if profit_usd > 10.0 { 15844367 } else { 3066993 }; // Gold for big profits, green for normal

        let message = json!({
            "embeds": [{
                "title": "💰 Arbitrage Profit!",
                "color": color,
                "fields": [
                    {
                        "name": "💵 Profit",
                        "value": format!("${:.2}", profit_usd),
                        "inline": true
                    },
                    {
                        "name": "📊 Amount",
                        "value": format!("{:.3} SOL", amount_sol),
                        "inline": true
                    },
                    {
                        "name": "🔄 Route",
                        "value": format!("{} → {}", buy_dex, sell_dex),
                        "inline": true
                    },
                    {
                        "name": "🔗 Transaction",
                        "value": format!("[View on Solscan](https://solscan.io/tx/{})", signature),
                        "inline": false
                    }
                ],
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "footer": {
                    "text": "Solana Arbitrage Bot v2.0"
                }
            }]
        });

        self.send_webhook(message).await
    }

    pub async fn send_opportunity_alert(
        &self,
        raydium_price: f64,
        orca_price: f64,
        profit_percent: f64,
        confidence: u8,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let message = json!({
            "embeds": [{
                "title": "🎯 Arbitrage Opportunity Detected",
                "color": 16776960, // Yellow
                "fields": [
                    {
                        "name": "📊 Raydium Price",
                        "value": format!("${:.4}", raydium_price),
                        "inline": true
                    },
                    {
                        "name": "🐋 Orca Price",
                        "value": format!("${:.4}", orca_price),
                        "inline": true
                    },
                    {
                        "name": "📈 Profit %",
                        "value": format!("{:.2}%", profit_percent),
                        "inline": true
                    },
                    {
                        "name": "🎯 Confidence",
                        "value": format!("{}%", confidence),
                        "inline": true
                    }
                ],
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "footer": {
                    "text": "Solana Arbitrage Bot v2.0"
                }
            }]
        });

        self.send_webhook(message).await
    }

    pub async fn send_error_alert(&self, error_msg: &str, context: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Sanitize error message to prevent sensitive data leaks
        let sanitized_error = self.sanitize_error_message(error_msg);

        let message = json!({
            "embeds": [{
                "title": "🚨 Bot Error",
                "color": 15158332, // Red
                "fields": [
                    {
                        "name": "❌ Error",
                        "value": format!("```{}```", sanitized_error),
                        "inline": false
                    },
                    {
                        "name": "📍 Context",
                        "value": context,
                        "inline": false
                    }
                ],
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "footer": {
                    "text": "Solana Arbitrage Bot v2.0"
                }
            }]
        });

        self.send_webhook(message).await
    }

    /// Sanitize error messages to prevent sensitive data leaks
    fn sanitize_error_message(&self, error_msg: &str) -> String {
        let mut sanitized = error_msg.to_string();

        // Remove potential private keys (64 hex chars)
        let private_key_regex = regex::Regex::new(r"[0-9a-fA-F]{64}").unwrap();
        sanitized = private_key_regex.replace_all(&sanitized, "***PRIVATE_KEY***").to_string();

        // Remove potential API keys (common patterns)
        let api_key_regex = regex::Regex::new(r"[A-Za-z0-9]{32,}").unwrap();
        sanitized = api_key_regex.replace_all(&sanitized, "***API_KEY***").to_string();

        // Truncate very long error messages
        if sanitized.len() > 500 {
            sanitized = format!("{}... (truncated)", &sanitized[..500]);
        }

        sanitized
    }

    pub async fn send_price_update(&self, raydium_price: f64, orca_price: f64) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let spread = (orca_price - raydium_price).abs() / raydium_price * 100.0;

        let message = json!({
            "embeds": [{
                "title": "📊 Price Update",
                "color": 3447003, // Blue
                "fields": [
                    {
                        "name": "📊 Raydium",
                        "value": format!("${:.4}", raydium_price),
                        "inline": true
                    },
                    {
                        "name": "🐋 Orca",
                        "value": format!("${:.4}", orca_price),
                        "inline": true
                    },
                    {
                        "name": "📈 Spread",
                        "value": format!("{:.3}%", spread),
                        "inline": true
                    }
                ],
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "footer": {
                    "text": "Live Price Feed"
                }
            }]
        });

        self.send_webhook(message).await
    }

    async fn send_webhook(&self, payload: serde_json::Value) -> Result<()> {
        for attempt in 1..=3 {
            match self
                .client
                .post(&self.webhook_url)
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("📢 Discord alert sent successfully");
                        return Ok(());
                    } else {
                        warn!("Discord webhook failed with status: {}", response.status());
                    }
                }
                Err(e) => {
                    warn!("Discord webhook attempt {} failed: {}", attempt, e);
                }
            }

            if attempt < 3 {
                sleep(Duration::from_secs(2)).await;
            }
        }

        error!("Failed to send Discord alert after 3 attempts");
        Ok(()) // Don't fail the whole bot for Discord issues
    }
}

impl DiscordAlert {
    pub async fn send_dashboard_status(&self, event: &str, info: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let color = match event {
            "connected" => 3066993,
            "disconnected" => 15158332,
            _ => 3447003,
        };
        let message = serde_json::json!({
            "embeds": [{
                "title": "🌐 Dashboard Status",
                "color": color,
                "fields": [
                    { "name": "Event", "value": event, "inline": true },
                    { "name": "Info", "value": info, "inline": false }
                ],
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }]
        });
        self.send_webhook(message).await
    }
}
