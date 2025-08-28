//! Advanced Slippage Prediction using Machine Learning
//! Uses polynomial regression and historical data

use anyhow::Result;
use nalgebra::{DMatrix, DVector};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlippageDataPoint {
    pub timestamp: DateTime<Utc>,
    pub volume: f64,
    pub liquidity: f64,
    pub volatility: f64,
    pub time_of_day: u32, // minutes since midnight
    pub actual_slippage: f64,
    pub predicted_slippage: f64,
}

pub struct SlippagePredictor {
    /// Historical data points for training
    history: VecDeque<SlippageDataPoint>,
    /// Maximum history size
    max_history_size: usize,
    /// Trained model coefficients
    model_coefficients: Option<DVector<f64>>,
    /// Feature scaling parameters
    feature_means: Vec<f64>,
    feature_stds: Vec<f64>,
}

impl SlippagePredictor {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_history_size),
            max_history_size,
            model_coefficients: None,
            feature_means: vec![0.0; 10],
            feature_stds: vec![1.0; 10],
        }
    }

    /// Add new data point and retrain if needed
    pub fn add_data_point(&mut self, data: SlippageDataPoint) {
        self.history.push_back(data);
        
        if self.history.len() > self.max_history_size {
            self.history.pop_front();
        }

        // Retrain every 100 new points
        if self.history.len() % 100 == 0 && self.history.len() >= 100 {
            if let Err(e) = self.train_model() {
                log::warn!("Failed to retrain slippage model: {}", e);
            }
        }
    }

    /// Extract features from raw inputs
    fn extract_features(&self, volume: f64, liquidity: f64, volatility: f64, time_of_day: u32) -> Vec<f64> {
        vec![
            // Basic features
            volume,
            liquidity,
            volatility,
            time_of_day as f64,
            
            // Engineered features
            volume / liquidity,                // Impact ratio
            (volume / liquidity).powi(2),      // Quadratic impact
            volume.ln(),                        // Log volume
            liquidity.ln(),                     // Log liquidity
            volatility * (volume / liquidity),  // Volatility-adjusted impact
            (time_of_day as f64 / 1440.0).sin() * 2.0 * std::f64::consts::PI, // Time cyclical feature
        ]
    }

    /// Normalize features using z-score normalization
    fn normalize_features(&self, features: &[f64]) -> Vec<f64> {
        features
            .iter()
            .zip(&self.feature_means)
            .zip(&self.feature_stds)
            .map(|((f, mean), std)| {
                if *std > 0.0 {
                    (f - mean) / std
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// Train polynomial regression model
    pub fn train_model(&mut self) -> Result<()> {
        if self.history.len() < 50 {
            return Err(anyhow::anyhow!("Not enough data for training"));
        }

        let n_samples = self.history.len();
        let n_features = 10;

        // Prepare feature matrix
        let mut x_data = Vec::with_capacity(n_samples * n_features);
        let mut y_data = Vec::with_capacity(n_samples);

        // Calculate feature statistics for normalization
        let mut feature_sums = vec![0.0; n_features];
        let mut feature_sq_sums = vec![0.0; n_features];

        // First pass: collect data and calculate means
        for point in &self.history {
            let features = self.extract_features(
                point.volume,
                point.liquidity,
                point.volatility,
                point.time_of_day,
            );
            
            for (i, &f) in features.iter().enumerate() {
                feature_sums[i] += f;
                feature_sq_sums[i] += f * f;
            }
            
            y_data.push(point.actual_slippage);
        }

        // Calculate means and standard deviations
        for i in 0..n_features {
            self.feature_means[i] = feature_sums[i] / n_samples as f64;
            let variance = (feature_sq_sums[i] / n_samples as f64) - self.feature_means[i].powi(2);
            self.feature_stds[i] = variance.max(0.0).sqrt();
        }

        // Second pass: normalize and build matrix
        for point in &self.history {
            let features = self.extract_features(
                point.volume,
                point.liquidity,
                point.volatility,
                point.time_of_day,
            );
            let normalized = self.normalize_features(&features);
            x_data.extend(normalized);
        }

        // Create matrices
        let x = DMatrix::from_row_slice(n_samples, n_features, &x_data);
        let y = DVector::from_vec(y_data);

        // Add bias term
        let ones = DVector::from_element(n_samples, 1.0);
        let x_with_bias = DMatrix::from_columns(&[
            &ones.column(0),
            &x.column(0),
            &x.column(1),
            &x.column(2),
            &x.column(3),
            &x.column(4),
            &x.column(5),
            &x.column(6),
            &x.column(7),
            &x.column(8),
            &x.column(9),
        ]);

        // Solve using least squares (X^T * X)^-1 * X^T * y
        let xt = x_with_bias.transpose();
        let xtx = &xt * &x_with_bias;
        
        // Add regularization to prevent overfitting
        let lambda = 0.01;
        let regularization = DMatrix::identity(n_features + 1, n_features + 1) * lambda;
        let xtx_reg = xtx + regularization;
        
        match xtx_reg.try_inverse() {
            Some(xtx_inv) => {
                self.model_coefficients = Some(xtx_inv * xt * y);
                log::info!("Slippage model trained successfully with {} samples", n_samples);
                Ok(())
            }
            None => {
                Err(anyhow::anyhow!("Matrix is singular, cannot train model"))
            }
        }
    }

    /// Predict slippage for given conditions
    pub fn predict(&self, volume: f64, liquidity: f64, volatility: f64) -> f64 {
        // If no model trained, use simple heuristic
        if self.model_coefficients.is_none() {
            return self.simple_heuristic(volume, liquidity, volatility);
        }

        // Get current time of day
        let now = Utc::now();
        let time_of_day = (now.hour() * 60 + now.minute()) as u32;

        // Extract and normalize features
        let features = self.extract_features(volume, liquidity, volatility, time_of_day);
        let normalized = self.normalize_features(&features);

        // Add bias term
        let mut x_vec = vec![1.0];
        x_vec.extend(normalized);

        // Predict using model
        let x = DVector::from_vec(x_vec);
        let coeffs = self.model_coefficients.as_ref().unwrap();
        let prediction = coeffs.dot(&x);

        // Ensure prediction is within reasonable bounds
        prediction.max(0.001).min(0.20) // Between 0.1% and 20%
    }

    /// Simple heuristic for when model is not trained
    fn simple_heuristic(&self, volume: f64, liquidity: f64, volatility: f64) -> f64 {
        let base_slippage = 0.002; // 0.2% base
        let impact_factor = (volume / liquidity).min(0.1); // Max 10% impact
        let volatility_factor = volatility * 0.5;
        
        base_slippage + impact_factor + volatility_factor
    }

    /// Get prediction confidence based on similar historical conditions
    pub fn get_confidence(&self, volume: f64, liquidity: f64, volatility: f64) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }

        // Find similar historical conditions
        let mut similar_count = 0;
        let tolerance = 0.2; // 20% tolerance

        for point in &self.history {
            let volume_diff = (point.volume - volume).abs() / volume;
            let liquidity_diff = (point.liquidity - liquidity).abs() / liquidity;
            let volatility_diff = (point.volatility - volatility).abs() / volatility.max(0.01);

            if volume_diff < tolerance && liquidity_diff < tolerance && volatility_diff < tolerance {
                similar_count += 1;
            }
        }

        // Confidence based on number of similar conditions seen
        (similar_count as f64 / self.history.len().min(50) as f64).min(1.0)
    }

    /// Calculate adaptive slippage tolerance based on market conditions
    pub fn get_adaptive_tolerance(&self, base_tolerance: f64, volatility: f64) -> f64 {
        // Increase tolerance during high volatility
        let volatility_multiplier = 1.0 + volatility.min(0.5);
        
        // Time-based adjustment (wider tolerance during off-hours)
        let now = Utc::now();
        let hour = now.hour();
        let time_multiplier = if hour < 9 || hour > 21 {
            1.5 // 50% higher tolerance during off-hours
        } else {
            1.0
        };

        base_tolerance * volatility_multiplier * time_multiplier
    }

    /// Export model for persistence
    pub fn export_model(&self) -> Option<String> {
        self.model_coefficients.as_ref().map(|coeffs| {
            serde_json::to_string(&(
                coeffs.as_slice(),
                &self.feature_means,
                &self.feature_stds,
            )).unwrap()
        })
    }

    /// Import model from saved state
    pub fn import_model(&mut self, model_str: &str) -> Result<()> {
        let (coeffs, means, stds): (Vec<f64>, Vec<f64>, Vec<f64>) = 
            serde_json::from_str(model_str)?;
        
        self.model_coefficients = Some(DVector::from_vec(coeffs));
        self.feature_means = means;
        self.feature_stds = stds;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slippage_prediction() {
        let mut predictor = SlippagePredictor::new(1000);
        
        // Add sample data
        for i in 0..100 {
            let data = SlippageDataPoint {
                timestamp: Utc::now(),
                volume: 1000.0 + i as f64 * 10.0,
                liquidity: 100000.0,
                volatility: 0.02,
                time_of_day: 720,
                actual_slippage: 0.005 + (i as f64 * 0.0001),
                predicted_slippage: 0.005,
            };
            predictor.add_data_point(data);
        }

        // Test prediction
        let slippage = predictor.predict(5000.0, 100000.0, 0.02);
        assert!(slippage > 0.0 && slippage < 0.2);
        
        // Test confidence
        let confidence = predictor.get_confidence(5000.0, 100000.0, 0.02);
        assert!(confidence >= 0.0 && confidence <= 1.0);
    }
}
