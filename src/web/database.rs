//! Database module for storing transaction history and bot statistics

use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{info, error};
use rust_decimal::Decimal;
use sqlx::{SqlitePool, Row};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteJournalMode};
use std::path::Path;
use std::str::FromStr;

use super::TransactionRecord;

/// Database connection and operations
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create a new database connection
    pub async fn new(database_path: &str) -> Result<Self> {
        // Create directory if it doesn't exist
        if let Some(parent) = Path::new(database_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Prefer explicit options to ensure the file is created and WAL is enabled
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", database_path))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        
        let db = Self { pool };
        db.init_schema().await?;
        
        info!("📊 Database initialized at: {}", database_path);
        Ok(db)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                signature TEXT NOT NULL UNIQUE,
                buy_dex TEXT NOT NULL,
                sell_dex TEXT NOT NULL,
                amount_sol TEXT NOT NULL,
                profit_usd TEXT NOT NULL,
                raydium_price TEXT NOT NULL,
                orca_price TEXT NOT NULL,
                spread_percent TEXT NOT NULL,
                gas_fee TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS bot_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL UNIQUE,
                trades_count INTEGER DEFAULT 0,
                total_profit_usd TEXT DEFAULT '0',
                total_volume_sol TEXT DEFAULT '0',
                avg_profit_per_trade TEXT DEFAULT '0',
                win_rate_percent TEXT DEFAULT '0',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        info!("✅ Database schema initialized");
        Ok(())
    }

    /// Insert a new transaction record
    pub async fn insert_transaction(&self, tx: &TransactionRecord) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO transactions (
                timestamp, signature, buy_dex, sell_dex, amount_sol,
                profit_usd, raydium_price, orca_price, spread_percent, gas_fee
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(tx.timestamp.to_rfc3339())
        .bind(&tx.signature)
        .bind(&tx.buy_dex)
        .bind(&tx.sell_dex)
        .bind(tx.amount_sol.to_string())
        .bind(tx.profit_usd.to_string())
        .bind(tx.raydium_price.to_string())
        .bind(tx.orca_price.to_string())
        .bind(tx.spread_percent.to_string())
        .bind(tx.gas_fee.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get recent transactions with pagination
    pub async fn get_transactions(&self, limit: i64, offset: i64) -> Result<Vec<TransactionRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, signature, buy_dex, sell_dex, amount_sol,
                   profit_usd, raydium_price, orca_price, spread_percent, gas_fee
            FROM transactions
            ORDER BY timestamp DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut transactions = Vec::new();
        for row in rows {
            let tx = TransactionRecord {
                id: Some(row.get::<i64, _>("id")),
                timestamp: DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))?.with_timezone(&Utc),
                signature: row.get("signature"),
                buy_dex: row.get("buy_dex"),
                sell_dex: row.get("sell_dex"),
                amount_sol: row.get::<String, _>("amount_sol").parse()?,
                profit_usd: row.get::<String, _>("profit_usd").parse()?,
                raydium_price: row.get::<String, _>("raydium_price").parse()?,
                orca_price: row.get::<String, _>("orca_price").parse()?,
                spread_percent: row.get::<String, _>("spread_percent").parse()?,
                gas_fee: row.get::<String, _>("gas_fee").parse()?,
            };
            transactions.push(tx);
        }

        Ok(transactions)
    }

    /// Get daily statistics
    pub async fn get_daily_stats(&self, date: &str) -> Result<Option<DailyStats>> {
        let row = sqlx::query(
            r#"
            SELECT trades_count, total_profit_usd, total_volume_sol,
                   avg_profit_per_trade, win_rate_percent
            FROM bot_stats
            WHERE date = ?
            "#,
        )
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(DailyStats {
                date: date.to_string(),
                trades_count: row.get("trades_count"),
                total_profit_usd: row.get::<String, _>("total_profit_usd").parse()?,
                total_volume_sol: row.get::<String, _>("total_volume_sol").parse()?,
                avg_profit_per_trade: row.get::<String, _>("avg_profit_per_trade").parse()?,
                win_rate_percent: row.get::<String, _>("win_rate_percent").parse()?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update daily statistics
    pub async fn update_daily_stats(&self, date: &str, stats: &DailyStats) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO bot_stats (
                date, trades_count, total_profit_usd, total_volume_sol,
                avg_profit_per_trade, win_rate_percent, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(date)
        .bind(stats.trades_count)
        .bind(stats.total_profit_usd.to_string())
        .bind(stats.total_volume_sol.to_string())
        .bind(stats.avg_profit_per_trade.to_string())
        .bind(stats.win_rate_percent.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get transaction count
    pub async fn get_transaction_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM transactions")
            .fetch_one(&self.pool)
            .await?;
        
        Ok(row.get("count"))
    }
}

/// Daily statistics structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub trades_count: i64,
    pub total_profit_usd: Decimal,
    pub total_volume_sol: Decimal,
    pub avg_profit_per_trade: Decimal,
    pub win_rate_percent: Decimal,
}
