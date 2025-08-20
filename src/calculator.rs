//! Profit Calculator
//! Calculates arbitrage opportunities considering gas, slippage, and fees

use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

pub struct ProfitCalculator {
    gas_cost_sol: Decimal,
    min_profit_percent: Decimal,
    max_slippage_percent: Decimal,
    dex_fee_percent: Decimal,
}

#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    pub buy_dex: String,
    pub sell_dex: String,
    pub buy_price: f64,
    pub sell_price: f64,
    pub amount_sol: f64,
    pub expected_profit_usd: f64,
    pub profit_after_fees_usd: f64,
}

impl ProfitCalculator {
    pub fn new(config: &crate::Config) -> Self {
        Self {
            gas_cost_sol: dec!(0.00025), // ~0.00025 SOL for 2 swaps
            min_profit_percent: Decimal::from_f64(config.limits.min_profit_percent).unwrap(),
            max_slippage_percent: Decimal::from_f64(config.limits.max_slippage_percent).unwrap(),
            dex_fee_percent: dec!(0.0025), // 0.25% typical DEX fee
        }
    }
    
    pub fn calculate_opportunity(
        &self,
        raydium_price: f64,
        orca_price: f64,
        max_position_sol: f64,
    ) -> Option<ArbitrageOpportunity> {
        let price_diff = (raydium_price - orca_price).abs();
        let avg_price = (raydium_price + orca_price) / 2.0;
        let spread_percent = (price_diff / avg_price) * 100.0;
        
        // Need minimum spread to be profitable
        if spread_percent < self.min_profit_percent.to_f64().unwrap() {
            return None;
        }
        
        // Determine buy/sell direction
        let (buy_dex, buy_price, sell_dex, sell_price) = if raydium_price < orca_price {
            ("Raydium", raydium_price, "Orca", orca_price)
        } else {
            ("Orca", orca_price, "Raydium", raydium_price)
        };
        
        // Calculate optimal position size (simplified)
        let position_sol = self.calculate_optimal_size(
            spread_percent,
            max_position_sol,
        );
        
        // Calculate expected profit
        let gross_profit_usd = position_sol * price_diff;
        
        // Subtract fees and costs
        let gas_cost_usd = self.gas_cost_sol.to_f64().unwrap() * avg_price;
        let dex_fees_usd = position_sol * avg_price * self.dex_fee_percent.to_f64().unwrap() * 2.0; // Buy + sell
        let slippage_cost_usd = position_sol * avg_price * self.max_slippage_percent.to_f64().unwrap() / 100.0;
        
        let net_profit_usd = gross_profit_usd - gas_cost_usd - dex_fees_usd - slippage_cost_usd;
        
        // Only return if profitable after all costs
        if net_profit_usd > 0.0 {
            Some(ArbitrageOpportunity {
                buy_dex: buy_dex.to_string(),
                sell_dex: sell_dex.to_string(),
                buy_price,
                sell_price,
                amount_sol: position_sol,
                expected_profit_usd: gross_profit_usd,
                profit_after_fees_usd: net_profit_usd,
            })
        } else {
            None
        }
    }
    
    fn calculate_optimal_size(&self, spread_percent: f64, max_position: f64) -> f64 {
        // Simple sizing: larger position for larger spreads
        // In reality, you'd consider liquidity depth
        
        if spread_percent > 1.0 {
            max_position // Full size for >1% spreads
        } else if spread_percent > 0.5 {
            max_position * 0.5 // Half size for 0.5-1% spreads
        } else {
            max_position * 0.25 // Quarter size for small spreads
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_profitable_arbitrage() {
        let config = crate::Config {
            limits: crate::LimitsConfig {
                min_profit_percent: 0.3,
                max_slippage_percent: 0.5,
                // ... other fields
            },
            // ... other fields
        };
        
        let calc = ProfitCalculator::new(&config);
        
        // 1% spread should be profitable
        let opp = calc.calculate_opportunity(
            150.0,  // Raydium price
            151.5,  // Orca price (1% higher)
            10.0,   // 10 SOL max position
        );
        
        assert!(opp.is_some());
        let opp = opp.unwrap();
        assert_eq!(opp.buy_dex, "Raydium");
        assert_eq!(opp.sell_dex, "Orca");
        assert!(opp.profit_after_fees_usd > 0.0);
    }
    
    #[test]
    fn test_unprofitable_arbitrage() {
        let config = crate::Config {
            limits: crate::LimitsConfig {
                min_profit_percent: 0.3,
                max_slippage_percent: 0.5,
                // ... other fields
            },
            // ... other fields
        };
        
        let calc = ProfitCalculator::new(&config);
        
        // 0.1% spread should NOT be profitable after fees
        let opp = calc.calculate_opportunity(
            150.0,   // Raydium price
            150.15,  // Orca price (0.1% higher)
            10.0,    // 10 SOL max position
        );
        
        assert!(opp.is_none());
    }
}
