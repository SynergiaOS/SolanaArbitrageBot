//! 🛡️ Advanced Rug Pull Detection System
//! 
//! Sophisticated algorithms to detect and prevent rug pull scams

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};
use log::{info, warn, error, debug};

use super::enhanced_detector::{EnhancedTokenLaunch, TokenMetadata, TokenDistribution, LiquidityLockInfo, ContractVerification};

/// Rug pull detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugDetectorConfig {
    /// Sensitivity level (0.0 - 1.0, higher = more sensitive)
    pub sensitivity: f64,
    
    /// Minimum holder count threshold
    pub min_holder_count: u32,
    
    /// Maximum team allocation percentage
    pub max_team_allocation: f64,
    
    /// Maximum top 10 concentration percentage
    pub max_top10_concentration: f64,
    
    /// Minimum liquidity lock percentage
    pub min_liquidity_lock_percent: f64,
    
    /// Minimum liquidity lock duration (hours)
    pub min_lock_duration_hours: u64,
    
    /// Honeypot detection threshold
    pub honeypot_threshold: f64,
    
    /// Enable social media analysis
    pub enable_social_analysis: bool,
    
    /// Enable contract pattern analysis
    pub enable_contract_analysis: bool,
    
    /// Enable creator history analysis
    pub enable_creator_analysis: bool,
}

impl Default for RugDetectorConfig {
    fn default() -> Self {
        Self {
            sensitivity: 0.8,
            min_holder_count: 50,
            max_team_allocation: 0.3, // 30%
            max_top10_concentration: 0.7, // 70%
            min_liquidity_lock_percent: 0.8, // 80%
            min_lock_duration_hours: 24 * 30, // 30 days
            honeypot_threshold: 0.3,
            enable_social_analysis: true,
            enable_contract_analysis: true,
            enable_creator_analysis: true,
        }
    }
}

/// Rug pull risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugRiskAssessment {
    /// Overall risk score (0.0 - 1.0, higher = riskier)
    pub overall_risk_score: f64,
    
    /// Individual risk factors
    pub risk_factors: Vec<RiskFactor>,
    
    /// Recommendation
    pub recommendation: RiskRecommendation,
    
    /// Detailed analysis
    pub analysis: DetailedAnalysis,
    
    /// Confidence level in assessment
    pub confidence: f64,
    
    /// Analysis timestamp
    pub timestamp: SystemTime,
}

/// Individual risk factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub category: RiskCategory,
    pub description: String,
    pub severity: RiskSeverity,
    pub score: f64,
    pub evidence: Vec<String>,
}

/// Risk categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskCategory {
    TokenDistribution,
    LiquidityLock,
    ContractSecurity,
    CreatorHistory,
    SocialPresence,
    MarketBehavior,
    TechnicalIndicators,
}

/// Risk severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Risk recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskRecommendation {
    SafeToTrade,
    TradeWithCaution,
    HighRisk,
    DoNotTrade,
}

/// Detailed analysis breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedAnalysis {
    pub distribution_analysis: DistributionAnalysis,
    pub liquidity_analysis: LiquidityAnalysis,
    pub contract_analysis: ContractAnalysis,
    pub creator_analysis: CreatorAnalysis,
    pub social_analysis: SocialAnalysis,
    pub market_analysis: MarketAnalysis,
}

/// Token distribution analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionAnalysis {
    pub holder_count: u32,
    pub top10_concentration: f64,
    pub team_allocation: f64,
    pub pool_allocation: f64,
    pub distribution_score: f64,
    pub suspicious_patterns: Vec<String>,
}

/// Liquidity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityAnalysis {
    pub is_locked: bool,
    pub lock_percentage: f64,
    pub lock_duration_hours: Option<u64>,
    pub lock_contract_verified: bool,
    pub liquidity_score: f64,
    pub warnings: Vec<String>,
}

/// Contract security analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAnalysis {
    pub is_verified: bool,
    pub has_mint_authority: bool,
    pub has_freeze_authority: bool,
    pub honeypot_risk: f64,
    pub malicious_patterns: Vec<String>,
    pub security_score: f64,
}

/// Creator history analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorAnalysis {
    pub previous_tokens: u32,
    pub previous_rugs: u32,
    pub reputation_score: f64,
    pub wallet_age_days: u32,
    pub suspicious_activity: Vec<String>,
}

/// Social media presence analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialAnalysis {
    pub has_website: bool,
    pub has_twitter: bool,
    pub has_telegram: bool,
    pub has_discord: bool,
    pub social_score: f64,
    pub red_flags: Vec<String>,
}

/// Market behavior analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketAnalysis {
    pub price_volatility: f64,
    pub volume_pattern: String,
    pub whale_activity: f64,
    pub market_score: f64,
    pub anomalies: Vec<String>,
}

/// Advanced rug pull detector
pub struct RugDetector {
    config: RugDetectorConfig,
    rpc_client: Arc<RpcClient>,
    
    /// Known rug pull patterns
    known_rug_patterns: Arc<RwLock<HashSet<String>>>,
    
    /// Creator blacklist
    creator_blacklist: Arc<RwLock<HashSet<Pubkey>>>,
    
    /// Historical data
    creator_history: Arc<RwLock<HashMap<Pubkey, CreatorHistory>>>,
    
    /// Analysis cache
    analysis_cache: Arc<RwLock<HashMap<Pubkey, (RugRiskAssessment, SystemTime)>>>,
    
    /// Performance metrics
    analysis_count: Arc<RwLock<u64>>,
    detection_count: Arc<RwLock<u64>>,
}

/// Creator historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreatorHistory {
    tokens_created: Vec<Pubkey>,
    rug_pulls: Vec<Pubkey>,
    first_seen: SystemTime,
    reputation_score: f64,
    last_updated: SystemTime,
}

impl RugDetector {
    /// Create new rug detector
    pub fn new(config: RugDetectorConfig, rpc_client: Arc<RpcClient>) -> Self {
        Self {
            config,
            rpc_client,
            known_rug_patterns: Arc::new(RwLock::new(HashSet::new())),
            creator_blacklist: Arc::new(RwLock::new(HashSet::new())),
            creator_history: Arc::new(RwLock::new(HashMap::new())),
            analysis_cache: Arc::new(RwLock::new(HashMap::new())),
            analysis_count: Arc::new(RwLock::new(0)),
            detection_count: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Analyze token launch for rug pull risk
    pub async fn analyze_rug_risk(
        &self,
        launch: &EnhancedTokenLaunch,
    ) -> Result<RugRiskAssessment> {
        // Check cache first
        if let Some((cached_assessment, timestamp)) = self.analysis_cache.read().await.get(&launch.token_mint) {
            if timestamp.elapsed().unwrap_or(Duration::MAX) < Duration::from_secs(300) { // 5 min cache
                return Ok(cached_assessment.clone());
            }
        }
        
        debug!("🔍 Analyzing rug risk for token: {}", launch.token_mint);
        
        let mut risk_factors = Vec::new();
        let mut total_risk_score = 0.0;
        let mut confidence = 1.0;
        
        // 1. Token Distribution Analysis
        let distribution_analysis = self.analyze_token_distribution(&launch.token_distribution).await;
        if distribution_analysis.distribution_score > 0.5 {
            risk_factors.push(RiskFactor {
                category: RiskCategory::TokenDistribution,
                description: "Suspicious token distribution detected".to_string(),
                severity: if distribution_analysis.distribution_score > 0.8 { 
                    RiskSeverity::Critical 
                } else { 
                    RiskSeverity::High 
                },
                score: distribution_analysis.distribution_score,
                evidence: distribution_analysis.suspicious_patterns.clone(),
            });
        }
        total_risk_score += distribution_analysis.distribution_score * 0.25;
        
        // 2. Liquidity Lock Analysis
        let liquidity_analysis = self.analyze_liquidity_lock(&launch.liquidity_lock).await;
        if liquidity_analysis.liquidity_score > 0.5 {
            risk_factors.push(RiskFactor {
                category: RiskCategory::LiquidityLock,
                description: "Liquidity lock concerns detected".to_string(),
                severity: if liquidity_analysis.liquidity_score > 0.8 { 
                    RiskSeverity::Critical 
                } else { 
                    RiskSeverity::High 
                },
                score: liquidity_analysis.liquidity_score,
                evidence: liquidity_analysis.warnings.clone(),
            });
        }
        total_risk_score += liquidity_analysis.liquidity_score * 0.3;
        
        // 3. Contract Security Analysis
        let contract_analysis = self.analyze_contract_security(&launch.contract_verification).await;
        if contract_analysis.security_score > 0.5 {
            risk_factors.push(RiskFactor {
                category: RiskCategory::ContractSecurity,
                description: "Contract security issues detected".to_string(),
                severity: if contract_analysis.security_score > 0.8 { 
                    RiskSeverity::Critical 
                } else { 
                    RiskSeverity::High 
                },
                score: contract_analysis.security_score,
                evidence: contract_analysis.malicious_patterns.clone(),
            });
        }
        total_risk_score += contract_analysis.security_score * 0.25;
        
        // 4. Creator History Analysis
        let creator_analysis = if let Some(creator) = launch.creator_address {
            self.analyze_creator_history(&creator).await
        } else {
            CreatorAnalysis {
                previous_tokens: 0,
                previous_rugs: 0,
                reputation_score: 0.5, // Unknown creator = medium risk
                wallet_age_days: 0,
                suspicious_activity: vec!["Unknown creator".to_string()],
            }
        };
        
        if creator_analysis.reputation_score > 0.5 {
            risk_factors.push(RiskFactor {
                category: RiskCategory::CreatorHistory,
                description: "Creator history concerns".to_string(),
                severity: if creator_analysis.reputation_score > 0.8 { 
                    RiskSeverity::High 
                } else { 
                    RiskSeverity::Medium 
                },
                score: creator_analysis.reputation_score,
                evidence: creator_analysis.suspicious_activity.clone(),
            });
        }
        total_risk_score += creator_analysis.reputation_score * 0.2;
        
        // 5. Social Media Analysis (if enabled)
        let social_analysis = if self.config.enable_social_analysis {
            self.analyze_social_presence(&launch.metadata).await
        } else {
            SocialAnalysis {
                has_website: false,
                has_twitter: false,
                has_telegram: false,
                has_discord: false,
                social_score: 0.3, // Neutral if disabled
                red_flags: vec![],
            }
        };
        
        if social_analysis.social_score > 0.5 {
            risk_factors.push(RiskFactor {
                category: RiskCategory::SocialPresence,
                description: "Social media red flags detected".to_string(),
                severity: RiskSeverity::Medium,
                score: social_analysis.social_score,
                evidence: social_analysis.red_flags.clone(),
            });
        }
        
        // 6. Market Behavior Analysis
        let market_analysis = self.analyze_market_behavior(launch).await;
        total_risk_score += market_analysis.market_score * 0.1;
        
        // Determine recommendation
        let recommendation = match total_risk_score {
            score if score < 0.3 => RiskRecommendation::SafeToTrade,
            score if score < 0.5 => RiskRecommendation::TradeWithCaution,
            score if score < 0.8 => RiskRecommendation::HighRisk,
            _ => RiskRecommendation::DoNotTrade,
        };
        
        // Adjust confidence based on data availability
        if launch.creator_address.is_none() { confidence *= 0.8; }
        if launch.metadata.uri.is_none() { confidence *= 0.9; }
        
        let assessment = RugRiskAssessment {
            overall_risk_score: total_risk_score,
            risk_factors,
            recommendation,
            analysis: DetailedAnalysis {
                distribution_analysis,
                liquidity_analysis,
                contract_analysis,
                creator_analysis,
                social_analysis,
                market_analysis,
            },
            confidence,
            timestamp: SystemTime::now(),
        };
        
        // Cache the result
        self.analysis_cache.write().await.insert(launch.token_mint, (assessment.clone(), SystemTime::now()));
        
        // Update metrics
        *self.analysis_count.write().await += 1;
        if matches!(assessment.recommendation, RiskRecommendation::HighRisk | RiskRecommendation::DoNotTrade) {
            *self.detection_count.write().await += 1;
        }
        
        info!("🛡️ Rug risk analysis completed: {:.2} risk score, {:?}", 
              total_risk_score, assessment.recommendation);
        
        Ok(assessment)
    }
    
    /// Analyze token distribution for suspicious patterns
    async fn analyze_token_distribution(&self, distribution: &TokenDistribution) -> DistributionAnalysis {
        let mut score = 0.0;
        let mut suspicious_patterns = Vec::new();
        
        // Check holder count
        if distribution.holder_count < self.config.min_holder_count {
            score += 0.3;
            suspicious_patterns.push(format!("Low holder count: {}", distribution.holder_count));
        }
        
        // Check top 10 concentration
        if distribution.top_10_concentration > self.config.max_top10_concentration {
            score += 0.4;
            suspicious_patterns.push(format!("High top 10 concentration: {:.1}%", 
                                            distribution.top_10_concentration * 100.0));
        }
        
        // Check team allocation
        if distribution.team_allocation > self.config.max_team_allocation {
            score += 0.3;
            suspicious_patterns.push(format!("High team allocation: {:.1}%", 
                                            distribution.team_allocation * 100.0));
        }
        
        // Add existing suspicious patterns
        suspicious_patterns.extend(distribution.suspicious_patterns.clone());
        
        DistributionAnalysis {
            holder_count: distribution.holder_count,
            top10_concentration: distribution.top_10_concentration,
            team_allocation: distribution.team_allocation,
            pool_allocation: distribution.pool_allocation,
            distribution_score: score.min(1.0),
            suspicious_patterns,
        }
    }
    
    /// Analyze liquidity lock for security
    async fn analyze_liquidity_lock(&self, lock_info: &LiquidityLockInfo) -> LiquidityAnalysis {
        let mut score = 0.0;
        let mut warnings = Vec::new();
        
        // Check if liquidity is locked
        if !lock_info.is_locked {
            score += 0.6;
            warnings.push("Liquidity is not locked".to_string());
        } else {
            // Check lock percentage
            if lock_info.locked_percentage < self.config.min_liquidity_lock_percent {
                score += 0.3;
                warnings.push(format!("Low liquidity lock percentage: {:.1}%", 
                                     lock_info.locked_percentage * 100.0));
            }
            
            // Check lock duration
            if let Some(duration) = lock_info.lock_duration {
                let duration_hours = duration / 3600;
                if duration_hours < self.config.min_lock_duration_hours {
                    score += 0.2;
                    warnings.push(format!("Short lock duration: {} hours", duration_hours));
                }
            } else {
                score += 0.1;
                warnings.push("Lock duration unknown".to_string());
            }
        }
        
        LiquidityAnalysis {
            is_locked: lock_info.is_locked,
            lock_percentage: lock_info.locked_percentage,
            lock_duration_hours: lock_info.lock_duration.map(|d| d / 3600),
            lock_contract_verified: lock_info.lock_contract.is_some(),
            liquidity_score: score.min(1.0),
            warnings,
        }
    }
    
    /// Analyze contract security
    async fn analyze_contract_security(&self, verification: &ContractVerification) -> ContractAnalysis {
        let mut score = 0.0;
        let mut malicious_patterns = Vec::new();
        
        // Check verification status
        if !verification.is_verified {
            score += 0.2;
            malicious_patterns.push("Contract not verified".to_string());
        }
        
        // Check honeypot risk
        if verification.honeypot_risk > self.config.honeypot_threshold {
            score += verification.honeypot_risk;
            malicious_patterns.push(format!("High honeypot risk: {:.1}%", 
                                           verification.honeypot_risk * 100.0));
        }
        
        // Add known malicious patterns
        malicious_patterns.extend(verification.contract_patterns.clone());
        
        ContractAnalysis {
            is_verified: verification.is_verified,
            has_mint_authority: false, // Would be extracted from metadata
            has_freeze_authority: false, // Would be extracted from metadata
            honeypot_risk: verification.honeypot_risk,
            malicious_patterns,
            security_score: score.min(1.0),
        }
    }
    
    // Additional analysis methods would be implemented here...
    // analyze_creator_history, analyze_social_presence, analyze_market_behavior
}
