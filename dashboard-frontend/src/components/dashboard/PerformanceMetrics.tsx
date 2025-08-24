"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { TrendingUp, TrendingDown, BarChart3 } from "lucide-react";
import { PerformanceMetrics as PerformanceMetricsType } from "@/components/providers/websocket-provider";

interface PerformanceMetricsProps {
  metrics: PerformanceMetricsType | null;
}

export function PerformanceMetrics({ metrics }: PerformanceMetricsProps) {
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

  // Mock data for the chart - in real implementation, this would come from historical data
  const chartData = [
    { time: '00:00', profit: 0 },
    { time: '04:00', profit: metrics ? metrics.totalProfitUsd * 0.2 : 0 },
    { time: '08:00', profit: metrics ? metrics.totalProfitUsd * 0.4 : 0 },
    { time: '12:00', profit: metrics ? metrics.totalProfitUsd * 0.7 : 0 },
    { time: '16:00', profit: metrics ? metrics.totalProfitUsd * 0.85 : 0 },
    { time: '20:00', profit: metrics ? metrics.totalProfitUsd * 0.95 : 0 },
    { time: '24:00', profit: metrics ? metrics.totalProfitUsd : 0 },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <BarChart3 className="h-5 w-5" />
          Performance Metrics
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          {/* Key Metrics Grid */}
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <div className="text-sm text-muted-foreground">Total Trades</div>
              <div className="text-2xl font-bold">
                {metrics ? metrics.totalTrades : 0}
              </div>
              <div className="text-xs text-muted-foreground">
                {metrics && (
                  <span className="text-green-600">
                    {metrics.successfulTrades} successful
                  </span>
                )}
              </div>
            </div>

            <div className="space-y-2">
              <div className="text-sm text-muted-foreground">Win Rate</div>
              <div className="text-2xl font-bold">
                {metrics ? `${(metrics.winRate * 100).toFixed(1)}%` : '0%'}
              </div>
              <div className="text-xs text-muted-foreground">
                {metrics && (
                  <span className={metrics.winRate >= 0.7 ? 'text-green-600' : 'text-yellow-600'}>
                    {metrics.winRate >= 0.7 ? 'Excellent' : 'Good'}
                  </span>
                )}
              </div>
            </div>

            <div className="space-y-2">
              <div className="text-sm text-muted-foreground">Sharpe Ratio</div>
              <div className="text-2xl font-bold">
                {metrics ? metrics.sharpeRatio.toFixed(2) : '0.00'}
              </div>
              <div className="text-xs text-muted-foreground">
                {metrics && (
                  <span className={metrics.sharpeRatio >= 1 ? 'text-green-600' : 'text-yellow-600'}>
                    {metrics.sharpeRatio >= 1 ? 'Good' : 'Fair'}
                  </span>
                )}
              </div>
            </div>

            <div className="space-y-2">
              <div className="text-sm text-muted-foreground">Max Drawdown</div>
              <div className="text-2xl font-bold">
                {metrics ? formatPercentage(metrics.maxDrawdown) : '0%'}
              </div>
              <div className="text-xs text-muted-foreground">
                {metrics && (
                  <span className={Math.abs(metrics.maxDrawdown) <= 10 ? 'text-green-600' : 'text-red-600'}>
                    {Math.abs(metrics.maxDrawdown) <= 10 ? 'Low risk' : 'High risk'}
                  </span>
                )}
              </div>
            </div>
          </div>

          {/* Profit Chart */}
          <div className="space-y-2">
            <div className="text-sm text-muted-foreground">24h Profit Progression</div>
            <div className="h-48">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis 
                    dataKey="time" 
                    className="text-xs fill-muted-foreground"
                  />
                  <YAxis 
                    className="text-xs fill-muted-foreground"
                    tickFormatter={(value) => `$${value.toFixed(0)}`}
                  />
                  <Tooltip 
                    formatter={(value: number) => [formatCurrency(value), 'Profit']}
                    labelStyle={{ color: 'hsl(var(--foreground))' }}
                    contentStyle={{ 
                      backgroundColor: 'hsl(var(--background))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '6px'
                    }}
                  />
                  <Line 
                    type="monotone" 
                    dataKey="profit" 
                    stroke="hsl(var(--primary))" 
                    strokeWidth={2}
                    dot={{ fill: 'hsl(var(--primary))', strokeWidth: 2, r: 4 }}
                    activeDot={{ r: 6, stroke: 'hsl(var(--primary))', strokeWidth: 2 }}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* P&L Summary */}
          <div className="grid grid-cols-2 gap-4 pt-4 border-t">
            <div className="flex items-center gap-2">
              <TrendingUp className="h-4 w-4 text-green-600" />
              <div>
                <div className="text-sm text-muted-foreground">Total Profit</div>
                <div className="font-semibold text-green-600">
                  {metrics ? formatCurrency(metrics.totalProfitUsd) : '$0.00'}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <TrendingDown className="h-4 w-4 text-red-600" />
              <div>
                <div className="text-sm text-muted-foreground">Total Loss</div>
                <div className="font-semibold text-red-600">
                  {metrics ? formatCurrency(metrics.totalLossUsd) : '$0.00'}
                </div>
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
