"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { TrendingUp, TrendingDown, DollarSign, Activity, Target, Shield } from "lucide-react";
import { PerformanceMetrics, SystemHealth } from "@/components/providers/websocket-provider";

interface DashboardOverviewProps {
  performanceMetrics: PerformanceMetrics | null;
  systemHealth: SystemHealth | null;
  positionsCount: number;
  opportunitiesCount: number;
}

export function DashboardOverview({ 
  performanceMetrics, 
  systemHealth, 
  positionsCount, 
  opportunitiesCount 
}: DashboardOverviewProps) {
  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
    }).format(value);
  };

  const formatPercentage = (value: number) => {
    return `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      {/* Total Profit */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Total Profit (24h)</CardTitle>
          <DollarSign className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {performanceMetrics ? formatCurrency(performanceMetrics.totalProfitUsd) : '$0.00'}
          </div>
          <p className="text-xs text-muted-foreground">
            {performanceMetrics && (
              <span className={performanceMetrics.dailyRoi >= 0 ? 'text-green-600' : 'text-red-600'}>
                {formatPercentage(performanceMetrics.dailyRoi)} from yesterday
              </span>
            )}
          </p>
        </CardContent>
      </Card>

      {/* Win Rate */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Win Rate</CardTitle>
          <TrendingUp className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {performanceMetrics ? `${(performanceMetrics.winRate * 100).toFixed(1)}%` : '0%'}
          </div>
          <p className="text-xs text-muted-foreground">
            {performanceMetrics && (
              <>
                {performanceMetrics.successfulTrades} / {performanceMetrics.totalTrades} trades
              </>
            )}
          </p>
        </CardContent>
      </Card>

      {/* Active Positions */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Active Positions</CardTitle>
          <Activity className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">{positionsCount}</div>
          <p className="text-xs text-muted-foreground">
            {opportunitiesCount} opportunities detected
          </p>
        </CardContent>
      </Card>

      {/* System Health */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">System Health</CardTitle>
          <Shield className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {systemHealth?.botStatus === 'online' ? (
              <span className="text-green-600">Online</span>
            ) : systemHealth?.botStatus === 'error' ? (
              <span className="text-red-600">Error</span>
            ) : (
              <span className="text-yellow-600">Offline</span>
            )}
          </div>
          <p className="text-xs text-muted-foreground">
            {systemHealth && (
              <>
                {systemHealth.walletBalance.toFixed(2)} SOL • {Math.floor(systemHealth.uptime / 3600)}h uptime
              </>
            )}
          </p>
        </CardContent>
      </Card>

      {/* Sharpe Ratio */}
      <Card className="md:col-span-2 lg:col-span-2">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Risk Metrics</CardTitle>
          <Target className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <div className="text-lg font-bold">
                {performanceMetrics ? performanceMetrics.sharpeRatio.toFixed(2) : '0.00'}
              </div>
              <p className="text-xs text-muted-foreground">Sharpe Ratio</p>
            </div>
            <div>
              <div className="text-lg font-bold">
                {performanceMetrics ? formatPercentage(performanceMetrics.maxDrawdown) : '0%'}
              </div>
              <p className="text-xs text-muted-foreground">Max Drawdown</p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Monthly Performance */}
      <Card className="md:col-span-2 lg:col-span-2">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Monthly Performance</CardTitle>
          {performanceMetrics && performanceMetrics.monthlyRoi >= 0 ? (
            <TrendingUp className="h-4 w-4 text-green-600" />
          ) : (
            <TrendingDown className="h-4 w-4 text-red-600" />
          )}
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <div className="text-lg font-bold">
                {performanceMetrics ? formatPercentage(performanceMetrics.monthlyRoi) : '0%'}
              </div>
              <p className="text-xs text-muted-foreground">Monthly ROI</p>
            </div>
            <div>
              <div className="text-lg font-bold">
                {performanceMetrics ? formatCurrency(performanceMetrics.totalProfitUsd - performanceMetrics.totalLossUsd) : '$0.00'}
              </div>
              <p className="text-xs text-muted-foreground">Net P&L</p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
