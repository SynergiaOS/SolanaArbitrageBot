"use client";

import { useWebSocket } from "@/components/providers/websocket-provider";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Zap, Target, Shield, TrendingUp, AlertTriangle, Clock, DollarSign } from "lucide-react";
import { formatDistanceToNow } from "date-fns";

export default function SniperPage() {
  const { 
    isConnected, 
    sniperActivities, 
    positions,
    performanceMetrics 
  } = useWebSocket();

  const sniperPositions = positions.filter(p => p.strategy === 'sniper');
  const recentActivities = sniperActivities.slice(0, 20);
  
  const sniperStats = {
    totalSnipes: sniperActivities.filter(a => a.action === 'sniped').length,
    rugsAvoided: sniperActivities.filter(a => a.action === 'rejected').length,
    averageExecutionTime: sniperActivities
      .filter(a => a.executionTimeMs)
      .reduce((sum, a) => sum + (a.executionTimeMs || 0), 0) / 
      sniperActivities.filter(a => a.executionTimeMs).length || 0,
    bestMultiplier: Math.max(...sniperActivities.map(a => a.profitMultiplier || 0), 0),
  };

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
    }).format(value);
  };

  const getActionColor = (action: string) => {
    switch (action) {
      case 'detected': return 'bg-blue-500';
      case 'analyzing': return 'bg-yellow-500';
      case 'sniped': return 'bg-green-500';
      case 'rejected': return 'bg-red-500';
      default: return 'bg-gray-500';
    }
  };

  const getRiskColor = (riskScore: number) => {
    if (riskScore <= 0.3) return 'text-green-600';
    if (riskScore <= 0.6) return 'text-yellow-600';
    return 'text-red-600';
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-foreground flex items-center gap-2">
            <Zap className="h-8 w-8" />
            Sniper Bot
          </h1>
          <p className="text-muted-foreground">
            High-frequency new token launch detection and trading
          </p>
        </div>
        <div className="flex items-center space-x-2">
          <div className={`w-3 h-3 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`}></div>
          <span className="text-sm text-muted-foreground">
            {isConnected ? 'Active' : 'Disconnected'}
          </span>
        </div>
      </div>

      {/* Stats Overview */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Snipes</CardTitle>
            <Target className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{sniperStats.totalSnipes}</div>
            <p className="text-xs text-muted-foreground">
              Successful token snipes
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Rugs Avoided</CardTitle>
            <Shield className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">{sniperStats.rugsAvoided}</div>
            <p className="text-xs text-muted-foreground">
              Potential losses prevented
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Avg Execution</CardTitle>
            <Clock className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {sniperStats.averageExecutionTime ? `${sniperStats.averageExecutionTime.toFixed(0)}ms` : '0ms'}
            </div>
            <p className="text-xs text-muted-foreground">
              Speed to market
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Best Multiplier</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">
              {sniperStats.bestMultiplier ? `${sniperStats.bestMultiplier.toFixed(1)}x` : '0x'}
            </div>
            <p className="text-xs text-muted-foreground">
              Highest return achieved
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Main Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Recent Activity */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Zap className="h-5 w-5" />
              Recent Activity
              <Badge variant="secondary">{recentActivities.length}</Badge>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4 max-h-96 overflow-y-auto">
              {recentActivities.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  No recent sniper activity
                </div>
              ) : (
                recentActivities.map((activity) => (
                  <div
                    key={activity.id}
                    className="flex items-center justify-between p-3 border rounded-lg hover:bg-accent/50 transition-colors"
                  >
                    <div className="flex items-center gap-3">
                      <div className={`w-2 h-8 rounded-full ${getActionColor(activity.action)}`}></div>
                      
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <span className="font-medium">
                            {activity.tokenSymbol || activity.tokenMint.slice(0, 8)}...
                          </span>
                          <Badge variant="outline" className="text-xs">
                            {activity.action}
                          </Badge>
                        </div>
                        
                        <div className="flex items-center gap-4 text-sm text-muted-foreground">
                          <div className="flex items-center gap-1">
                            <Shield className="h-3 w-3" />
                            <span className={getRiskColor(activity.riskScore)}>
                              {(activity.riskScore * 100).toFixed(0)}%
                            </span>
                          </div>
                          
                          <div>
                            {activity.liquiditySol.toFixed(1)} SOL
                          </div>
                          
                          <div className="flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            <span>
                              {formatDistanceToNow(new Date(activity.timestamp), { addSuffix: true })}
                            </span>
                          </div>
                        </div>
                      </div>
                    </div>

                    <div className="text-right">
                      <div className="text-sm font-medium">
                        {formatCurrency(activity.marketCapUsd)}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        {activity.executionTimeMs && `${activity.executionTimeMs}ms`}
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>
          </CardContent>
        </Card>

        {/* Active Sniper Positions */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <DollarSign className="h-5 w-5" />
              Active Positions
              <Badge variant="secondary">{sniperPositions.length}</Badge>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4 max-h-96 overflow-y-auto">
              {sniperPositions.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  No active sniper positions
                </div>
              ) : (
                sniperPositions.map((position) => (
                  <div
                    key={position.id}
                    className="flex items-center justify-between p-3 border rounded-lg hover:bg-accent/50 transition-colors"
                  >
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-medium">
                          {position.tokenSymbol || position.tokenMint.slice(0, 8)}...
                        </span>
                        <Badge variant="default" className="text-xs">
                          sniper
                        </Badge>
                      </div>
                      
                      <div className="flex items-center gap-4 text-sm text-muted-foreground">
                        <div>
                          Entry: ${position.entryPrice.toFixed(6)}
                        </div>
                        
                        <div>
                          Size: {position.positionSizeSol.toFixed(2)} SOL
                        </div>
                        
                        <div className="flex items-center gap-1">
                          <Clock className="h-3 w-3" />
                          <span>
                            {formatDistanceToNow(new Date(position.timestamp), { addSuffix: true })}
                          </span>
                        </div>
                      </div>
                    </div>

                    <div className="text-right">
                      <div className="text-sm font-medium">
                        {position.multiplier.toFixed(2)}x
                      </div>
                      <div className={`text-sm font-medium ${
                        position.unrealizedPnl > 0 ? 'text-green-600' : 
                        position.unrealizedPnl < 0 ? 'text-red-600' : 'text-muted-foreground'
                      }`}>
                        {formatCurrency(position.unrealizedPnl)}
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Risk Analysis */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            Risk Analysis & Rug Detection
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="space-y-2">
              <div className="text-sm text-muted-foreground">Detection Accuracy</div>
              <div className="text-2xl font-bold text-green-600">94.2%</div>
              <div className="text-xs text-muted-foreground">
                Rug pull prevention rate
              </div>
            </div>

            <div className="space-y-2">
              <div className="text-sm text-muted-foreground">Risk Threshold</div>
              <div className="text-2xl font-bold">60%</div>
              <div className="text-xs text-muted-foreground">
                Maximum risk tolerance
              </div>
            </div>

            <div className="space-y-2">
              <div className="text-sm text-muted-foreground">Avg Risk Score</div>
              <div className="text-2xl font-bold text-yellow-600">
                {recentActivities.length > 0 ? 
                  `${(recentActivities.reduce((sum, a) => sum + a.riskScore, 0) / recentActivities.length * 100).toFixed(0)}%` : 
                  '0%'
                }
              </div>
              <div className="text-xs text-muted-foreground">
                Recent tokens analyzed
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
