// Mock data for development when Enhanced Bot is not running

import { 
  ArbitrageOpportunity, 
  SniperActivity, 
  Position, 
  PerformanceMetrics, 
  GepaMetrics, 
  KestraMetrics, 
  SystemHealth 
} from "@/components/providers/websocket-provider";

export const mockArbitrageOpportunities: ArbitrageOpportunity[] = [
  {
    id: "arb-1",
    tokenA: "SOL",
    tokenB: "USDC",
    dexA: "Raydium",
    dexB: "Orca",
    priceA: 98.45,
    priceB: 99.12,
    spread: 0.68,
    profitUsd: 15.23,
    timestamp: Date.now() - 30000,
  },
  {
    id: "arb-2",
    tokenA: "RAY",
    tokenB: "USDC",
    dexA: "Orca",
    dexB: "Raydium",
    priceA: 2.34,
    priceB: 2.38,
    spread: 1.71,
    profitUsd: 8.45,
    timestamp: Date.now() - 45000,
  },
];

export const mockSniperActivities: SniperActivity[] = [
  {
    id: "snipe-1",
    tokenMint: "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
    tokenName: "BONK",
    tokenSymbol: "BONK",
    action: "sniped",
    riskScore: 0.25,
    liquiditySol: 45.2,
    marketCapUsd: 125000,
    timestamp: Date.now() - 120000,
    executionTimeMs: 87,
    profitMultiplier: 3.2,
  },
  {
    id: "snipe-2",
    tokenMint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    tokenName: "Pepe Coin",
    tokenSymbol: "PEPE",
    action: "rejected",
    riskScore: 0.85,
    liquiditySol: 12.1,
    marketCapUsd: 75000,
    timestamp: Date.now() - 180000,
  },
];

export const mockPositions: Position[] = [
  {
    id: "pos-1",
    tokenMint: "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
    tokenName: "BONK",
    tokenSymbol: "BONK",
    strategy: "sniper",
    entryPrice: 0.000012,
    currentPrice: 0.000038,
    positionSizeSol: 2.5,
    unrealizedPnl: 45.23,
    realizedPnl: 0,
    multiplier: 3.17,
    timestamp: Date.now() - 300000,
  },
  {
    id: "pos-2",
    tokenMint: "So11111111111111111111111111111111111111112",
    tokenName: "Solana",
    tokenSymbol: "SOL",
    strategy: "arbitrage",
    entryPrice: 98.45,
    currentPrice: 99.12,
    positionSizeSol: 5.0,
    unrealizedPnl: 3.35,
    realizedPnl: 12.45,
    multiplier: 1.007,
    timestamp: Date.now() - 600000,
  },
];

export const mockPerformanceMetrics: PerformanceMetrics = {
  totalTrades: 47,
  successfulTrades: 38,
  totalProfitUsd: 234.56,
  totalLossUsd: 45.23,
  winRate: 0.808,
  sharpeRatio: 2.34,
  maxDrawdown: -8.5,
  dailyRoi: 12.3,
  monthlyRoi: 245.7,
};

export const mockGepaMetrics: GepaMetrics = {
  currentGeneration: 23,
  maxGenerations: 100,
  bestFitnessScore: 0.847,
  populationDiversity: 0.65,
  isOptimizing: false,
  lastOptimization: Date.now() - 86400000, // 24 hours ago
  improvementPercent: 28.5,
};

export const mockKestraMetrics: KestraMetrics = {
  activeWorkflows: 3,
  completedWorkflows: 15,
  failedWorkflows: 1,
  successRate: 0.9375,
  lastExecution: Date.now() - 3600000, // 1 hour ago
  uptime: 0.995,
};

export const mockSystemHealth: SystemHealth = {
  botStatus: "online",
  rpcStatus: "connected",
  walletBalance: 12.45,
  systemLoad: 0.65,
  memoryUsage: 72.3,
  uptime: 86400, // 24 hours
  lastUpdate: Date.now(),
};

// Function to generate random variations of mock data
export function generateMockData() {
  const now = Date.now();
  
  // Add some randomness to make it feel more realistic
  const randomSpread = () => Math.random() * 2 + 0.1;
  const randomProfit = () => Math.random() * 50 + 5;
  
  return {
    arbitrageOpportunities: [
      {
        ...mockArbitrageOpportunities[0],
        id: `arb-${now}`,
        spread: randomSpread(),
        profitUsd: randomProfit(),
        timestamp: now,
      }
    ],
    sniperActivities: [
      {
        ...mockSniperActivities[0],
        id: `snipe-${now}`,
        action: Math.random() > 0.7 ? "sniped" : "analyzing" as any,
        riskScore: Math.random() * 0.6 + 0.1,
        timestamp: now,
      }
    ],
    performanceMetrics: {
      ...mockPerformanceMetrics,
      totalProfitUsd: mockPerformanceMetrics.totalProfitUsd + Math.random() * 10,
      dailyRoi: mockPerformanceMetrics.dailyRoi + (Math.random() - 0.5) * 2,
    },
    systemHealth: {
      ...mockSystemHealth,
      systemLoad: Math.random() * 0.3 + 0.4,
      memoryUsage: Math.random() * 20 + 60,
      lastUpdate: now,
    }
  };
}
