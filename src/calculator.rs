//! Enhanced Profit Calculator with Real-time Data Support
//! Optimized for high-frequency arbitrage calculations with minimal allocations

use crate::config_manager::BotConfig;
use log::info;
use rust_decimal::prelude::*;
use std::time::Instant;

// Pre-computed constants for performance
const DEX_FEE_MULTIPLIER: f64 = 0.005; // 0.25% * 2 sides
const BASE_GAS_COST_SOL: f64 = 0.00025;
const JUPITER_GAS_MULTIPLIER: f64 = 1.5;
const MAX_USD_POSITION: f64 = 10000.0;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArbitrageOpportunity {
    pub buy_dex: String,
    pub sell_dex: String,
    pub buy_price: f64,
    pub sell_price: f64,
    pub amount_sol: f64,
    pub expected_profit_usd: f64,
    pub profit_after_fees_usd: f64,
    pub profit_percentage: f64,
    pub estimated_gas_sol: f64,
    pub price_impact: f64,
    pub confidence_score: f64, // 0.0 to 1.0
}

#[derive(Debug, Clone)]
pub struct MarketConditions {
    pub network_congestion: NetworkCongestion,
    pub volatility: f64,
    pub recent_success_rate: f64,
}

#[derive(Debug, Clone)]
pub enum NetworkCongestion {
    Low,
    Medium,
    High,
    Extreme,
}

pub struct ProfitCalculator {
    // Pre-computed values for performance (avoid Decimal conversions)
    min_profit_percent_f64: f64,
    max_slippage_percent_f64: f64,
    dex_fee_percent_f64: f64,
    priority_fee_multiplier: f64,
    market_conditions: Option<MarketConditions>,

    // Performance tracking
    calculation_count: u64,
    total_calculation_time_ns: u64,
}

impl ProfitCalculator {
    pub fn new(config: &BotConfig) -> Self {
        Self {
            // Pre-convert to f64 for performance
            min_profit_percent_f64: config.trading.min_profit_percent.to_f64().unwrap_or(0.3),
            max_slippage_percent_f64: config.trading.max_slippage_percent.to_f64().unwrap_or(0.5),
            dex_fee_percent_f64: 0.0025, // 0.25% typical DEX fee
            priority_fee_multiplier: 1.0,
            market_conditions: None,
            calculation_count: 0,
            total_calculation_time_ns: 0,
        }
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (u64, f64) {
        let avg_time_ms = if self.calculation_count > 0 {
            (self.total_calculation_time_ns as f64 / self.calculation_count as f64) / 1_000_000.0
        } else {
            0.0
        };
        (self.calculation_count, avg_time_ms)
    }

    pub fn with_market_conditions(mut self, conditions: MarketConditions) -> Self {
        // Adjust parameters based on market conditions
        self.priority_fee_multiplier = match conditions.network_congestion {
            NetworkCongestion::Low => 1.0,
            NetworkCongestion::Medium => 2.0,
            NetworkCongestion::High => 5.0,
            NetworkCongestion::Extreme => 10.0,
        };
        self.market_conditions = Some(conditions);
        self
    }

    pub fn calculate_opportunity(
        &mut self, // Made mutable for performance tracking
        raydium_price: f64,
        orca_price: f64,
        max_position_sol: f64,
    ) -> Option<ArbitrageOpportunity> {
        let start_time = Instant::now();

        // Fast validation with early returns
        if raydium_price <= 0.0 || orca_price <= 0.0 {
            return None;
        }

        // Pre-compute values once
        let price_diff = (raydium_price - orca_price).abs();
        let avg_price = (raydium_price + orca_price) * 0.5; // Faster than division
        let spread_percent = (price_diff / avg_price) * 100.0;

        // Fast minimum spread check
        if spread_percent < self.min_profit_percent_f64 {
            return None;
        }

        // Determine buy/sell direction
        let (buy_dex, buy_price, sell_dex, sell_price) = if raydium_price < orca_price {
            ("Raydium", raydium_price, "Orca", orca_price)
        } else {
            ("Orca", orca_price, "Raydium", raydium_price)
        };

        info!(
            "🎯 Price spread detected: {:.4}% | Buy {} @ ${:.4}, Sell {} @ ${:.4}",
            spread_percent, buy_dex, buy_price, sell_dex, sell_price
        );

        // Calculate optimal position size (optimized)
        let position_sol = self.calculate_optimal_size(spread_percent, max_position_sol, avg_price);

        // Pre-compute common values
        let position_value_usd = position_sol * avg_price;
        let gross_profit_usd = position_sol * price_diff;

        // Calculate all costs in one pass (optimized)
        let gas_cost_sol =
            BASE_GAS_COST_SOL * self.priority_fee_multiplier * JUPITER_GAS_MULTIPLIER;
        let gas_cost_usd = gas_cost_sol * avg_price;

        // DEX fees (pre-computed multiplier)
        let dex_fees_usd = position_value_usd * DEX_FEE_MULTIPLIER;

        // Slippage cost (optimized)
        let slippage_cost_usd = position_value_usd * self.max_slippage_percent_f64 * 0.01;

        // Price impact (simplified for speed)
        let price_impact = self.estimate_price_impact(position_sol, avg_price);
        let price_impact_cost = position_value_usd * price_impact * 0.01;

        // Total costs
        let total_costs = gas_cost_usd + dex_fees_usd + slippage_cost_usd + price_impact_cost;
        let net_profit_usd = gross_profit_usd - total_costs;

        // Fast confidence score (simplified)
        let confidence =
            self.calculate_confidence_score(spread_percent, position_sol, net_profit_usd);

        // Update performance tracking
        self.calculation_count += 1;
        self.total_calculation_time_ns += start_time.elapsed().as_nanos() as u64;

        // Only return if profitable after all costs
        if net_profit_usd > 0.0 && confidence > 0.3 {
            Some(ArbitrageOpportunity {
                buy_dex: buy_dex.to_string(),
                sell_dex: sell_dex.to_string(),
                buy_price,
                sell_price,
                amount_sol: position_sol,
                expected_profit_usd: gross_profit_usd,
                profit_after_fees_usd: net_profit_usd,
                profit_percentage: (net_profit_usd / position_value_usd) * 100.0,
                estimated_gas_sol: gas_cost_sol,
                price_impact,
                confidence_score: confidence,
            })
        } else {
            None // Removed debug log for performance
        }
    }

    fn calculate_optimal_size(
        &self,
        spread_percent: f64,
        max_position: f64,
        avg_price: f64,
    ) -> f64 {
        // Fast branching with pre-computed multipliers
        let base_multiplier = if spread_percent > 2.0 {
            1.0
        } else if spread_percent > 1.0 {
            0.75
        } else if spread_percent > 0.5 {
            0.5
        } else {
            0.25
        };

        // Apply market conditions adjustment
        let market_multiplier = if let Some(conditions) = &self.market_conditions {
            match conditions.network_congestion {
                NetworkCongestion::Low => 1.0,
                NetworkCongestion::Medium => 0.8,
                NetworkCongestion::High => 0.5,
                NetworkCongestion::Extreme => 0.25,
            }
        } else {
            1.0
        };

        let base_size = max_position * base_multiplier * market_multiplier;

        // Apply USD cap
        let max_sol_from_usd = MAX_USD_POSITION / avg_price;
        base_size.min(max_sol_from_usd).min(max_position)
    }

    fn estimate_price_impact(&self, amount_sol: f64, _price: f64) -> f64 {
        // Simplified price impact estimation
        // In reality, would query liquidity depth
        if amount_sol < 10.0 {
            0.01 // 0.01% for small trades
        } else if amount_sol < 50.0 {
            0.05 // 0.05% for medium trades
        } else if amount_sol < 100.0 {
            0.1 // 0.1% for large trades
        } else {
            0.2 // 0.2% for very large trades
        }
    }

    fn calculate_confidence_score(&self, spread: f64, size: f64, profit: f64) -> f64 {
        let mut score: f64 = 0.5; // Base score

        // Spread quality
        if spread > 1.0 {
            score += 0.2;
        } else if spread > 0.5 {
            score += 0.1;
        }

        // Profit quality
        if profit > 50.0 {
            score += 0.2;
        } else if profit > 10.0 {
            score += 0.1;
        }

        // Size appropriateness
        if size < 50.0 {
            score += 0.1; // Smaller is safer
        }

        // Market conditions
        if let Some(conditions) = &self.market_conditions {
            if conditions.recent_success_rate > 0.7 {
                score += 0.1;
            }
            if matches!(conditions.network_congestion, NetworkCongestion::Low) {
                score += 0.1;
            }
        }

        score.min(1.0)
    }

    // Advanced analysis methods
    pub fn analyze_historical_opportunity(
        &mut self,
        prices: &[(f64, f64, u64)],
    ) -> Vec<ArbitrageOpportunity> {
        // Analyze historical price data for backtesting
        let mut opportunities = Vec::new();

        for (raydium_price, orca_price, _timestamp) in prices {
            if let Some(opp) = self.calculate_opportunity(*raydium_price, *orca_price, 10.0) {
                opportunities.push(opp);
            }
        }

        opportunities
    }

    pub fn calculate_daily_metrics(&self, opportunities: &[ArbitrageOpportunity]) -> DailyMetrics {
        let total_profit: f64 = opportunities.iter().map(|o| o.profit_after_fees_usd).sum();

        let avg_profit = if !opportunities.is_empty() {
            total_profit / opportunities.len() as f64
        } else {
            0.0
        };

        let best_trade = opportunities
            .iter()
            .max_by(|a, b| {
                a.profit_after_fees_usd
                    .partial_cmp(&b.profit_after_fees_usd)
                    .unwrap()
            })
            .cloned();

        DailyMetrics {
            total_opportunities: opportunities.len(),
            total_profit_usd: total_profit,
            average_profit_usd: avg_profit,
            best_trade,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DailyMetrics {
    pub total_opportunities: usize,
    pub total_profit_usd: f64,
    pub average_profit_usd: f64,
    pub best_trade: Option<ArbitrageOpportunity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> crate::Config {
        crate::Config {
            rpc: crate::RpcConfig {
                url: "test".to_string(),
                ws_url: "test".to_string(),
            },
            wallet: crate::WalletConfig {
                use_ledger: Some(false),
                ledger_path: None,
                path: "wallet.json".to_string(),
            },
            dex: crate::DexConfig {
                raydium: crate::DexInfo {
                    program_id: "test".to_string(),
                    sol_usdc_pool: "test".to_string(),
                },
                orca: crate::DexInfo {
                    program_id: "test".to_string(),
                    sol_usdc_pool: "test".to_string(),
                },
            },
            limits: crate::LimitsConfig {
                max_position_sol: rust_decimal::Decimal::from_f64_retain(10.0).unwrap(),
                min_profit_percent: rust_decimal::Decimal::from_f64_retain(0.3).unwrap(),
                min_profit_usd: rust_decimal::Decimal::from_f64_retain(1.0).unwrap(),
                max_slippage_percent: rust_decimal::Decimal::from_f64_retain(0.5).unwrap(),
                max_daily_loss_usd: rust_decimal::Decimal::from_f64_retain(100.0).unwrap(),
                max_daily_trades: 30,
            },
            execution: crate::ExecutionConfig {
                priority_fee_lamports: 10000,
                simulation_required: true,
                max_retries: 3,
            },
            discord: None,
            web: None,
        }
    }

    #[test]
    fn test_profitable_arbitrage() {
        let config = test_config();
        let mut calc = ProfitCalculator::new(&config);

        // Ustawiamy nieco szerszy spread (1.2%), by po uwzględnieniu kosztów wynik był dodatni
        let opp = calc.calculate_opportunity(
            150.0, // Raydium price
            151.8, // Orca price (~1.2% higher)
            100.0, // 100 SOL max position to ensure net > 0 after costs
        );

        assert!(
            opp.is_some(),
            "Expected profitable opportunity at ~1.2% spread with sufficient size"
        );
        let opp = opp.unwrap();
        assert_eq!(opp.buy_dex, "Raydium");
        assert_eq!(opp.sell_dex, "Orca");
        assert!(opp.profit_after_fees_usd > 0.0);
        assert!(opp.confidence_score > 0.3);
    }

    #[test]
    fn test_unprofitable_arbitrage() {
        let config = test_config();
        let mut calc = ProfitCalculator::new(&config);

        // 0.1% spread should NOT be profitable after fees
        let opp = calc.calculate_opportunity(
            150.0,  // Raydium price
            150.15, // Orca price (0.1% higher)
            10.0,   // 10 SOL max position
        );

        assert!(opp.is_none());
    }

    #[test]
    fn test_market_conditions_impact() {
        let config = test_config();
        let conditions = MarketConditions {
            network_congestion: NetworkCongestion::High,
            volatility: 0.5,
            recent_success_rate: 0.8,
        };

        let mut calc = ProfitCalculator::new(&config).with_market_conditions(conditions);

        let opp = calc.calculate_opportunity(150.0, 152.0, 100.0);

        assert!(opp.is_some());
        let opp = opp.unwrap();
        // Position should be reduced due to high congestion
        assert!(opp.amount_sol < 100.0);
    }
}
