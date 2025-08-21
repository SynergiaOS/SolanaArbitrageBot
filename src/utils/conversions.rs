//! Type conversion utilities for Decimal ↔ f64

use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};

/// Convert Decimal to f64 with fallback to 0.0
pub fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}

/// Convert f64 to Decimal with fallback to ZERO
pub fn f64_to_decimal(f: f64) -> Decimal {
    Decimal::from_f64(f).unwrap_or(Decimal::ZERO)
}

/// Multiply Decimal by f64 (converts f64 to Decimal first)
pub fn decimal_multiply_f64(d: Decimal, f: f64) -> Decimal {
    d * f64_to_decimal(f)
}

/// Add f64 to Decimal (converts f64 to Decimal first)
pub fn decimal_add_f64(d: Decimal, f: f64) -> Decimal {
    d + f64_to_decimal(f)
}

/// Subtract f64 from Decimal (converts f64 to Decimal first)
pub fn decimal_sub_f64(d: Decimal, f: f64) -> Decimal {
    d - f64_to_decimal(f)
}

/// Compare Decimal with f64 (converts f64 to Decimal first)
pub fn decimal_gt_f64(d: Decimal, f: f64) -> bool {
    d > f64_to_decimal(f)
}

/// Compare Decimal with f64 (converts f64 to Decimal first)
pub fn decimal_lt_f64(d: Decimal, f: f64) -> bool {
    d < f64_to_decimal(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_to_f64() {
        let d = Decimal::from_f64(123.456).unwrap();
        let f = decimal_to_f64(d);
        assert!((f - 123.456).abs() < 0.001);
    }

    #[test]
    fn test_f64_to_decimal() {
        let f = 123.456;
        let d = f64_to_decimal(f);
        assert!((decimal_to_f64(d) - f).abs() < 0.001);
    }

    #[test]
    fn test_decimal_multiply_f64() {
        let d = Decimal::from_f64(10.0).unwrap();
        let result = decimal_multiply_f64(d, 2.5);
        assert_eq!(decimal_to_f64(result), 25.0);
    }
}
