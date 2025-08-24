"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Activity, TrendingUp, TrendingDown, Clock } from "lucide-react";
import { Position } from "@/components/providers/websocket-provider";
import { formatDistanceToNow } from "date-fns";

interface ActivePositionsProps {
  positions: Position[];
}

export function ActivePositions({ positions }: ActivePositionsProps) {
  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
    }).format(value);
  };

  const formatMultiplier = (value: number) => {
    return `${value.toFixed(2)}x`;
  };

  const getStrategyColor = (strategy: string) => {
    switch (strategy) {
      case 'arbitrage':
        return 'bg-blue-500';
      case 'sniper':
        return 'bg-purple-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getStrategyBadgeVariant = (strategy: string) => {
    switch (strategy) {
      case 'arbitrage':
        return 'secondary';
      case 'sniper':
        return 'default';
      default:
        return 'outline';
    }
  };

  const getPnlColor = (pnl: number) => {
    if (pnl > 0) return 'text-green-600';
    if (pnl < 0) return 'text-red-600';
    return 'text-muted-foreground';
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Activity className="h-5 w-5" />
          Active Positions
          <Badge variant="secondary">{positions.length}</Badge>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {positions.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No active positions
            </div>
          ) : (
            positions.map((position) => (
              <div
                key={position.id}
                className="flex items-center justify-between p-3 border rounded-lg hover:bg-accent/50 transition-colors"
              >
                <div className="flex items-center gap-3">
                  <div className={`w-2 h-8 rounded-full ${getStrategyColor(position.strategy)}`}></div>
                  
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-medium">
                        {position.tokenSymbol || position.tokenMint.slice(0, 8)}...
                      </span>
                      <Badge variant={getStrategyBadgeVariant(position.strategy)} className="text-xs">
                        {position.strategy}
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
                </div>

                <div className="text-right space-y-1">
                  <div className="flex items-center gap-2">
                    <div className="text-sm font-medium">
                      {formatMultiplier(position.multiplier)}
                    </div>
                    {position.multiplier >= 1 ? (
                      <TrendingUp className="h-3 w-3 text-green-600" />
                    ) : (
                      <TrendingDown className="h-3 w-3 text-red-600" />
                    )}
                  </div>
                  
                  <div className={`text-sm font-medium ${getPnlColor(position.unrealizedPnl)}`}>
                    {formatCurrency(position.unrealizedPnl)}
                  </div>
                  
                  <div className="text-xs text-muted-foreground">
                    Current: ${position.currentPrice.toFixed(6)}
                  </div>
                </div>
              </div>
            ))
          )}
        </div>

        {/* Summary */}
        {positions.length > 0 && (
          <div className="mt-6 pt-4 border-t">
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <div className="text-muted-foreground">Total Unrealized P&L</div>
                <div className={`font-semibold ${getPnlColor(positions.reduce((sum, p) => sum + p.unrealizedPnl, 0))}`}>
                  {formatCurrency(positions.reduce((sum, p) => sum + p.unrealizedPnl, 0))}
                </div>
              </div>
              
              <div>
                <div className="text-muted-foreground">Total Position Value</div>
                <div className="font-semibold">
                  {formatCurrency(positions.reduce((sum, p) => sum + (p.positionSizeSol * p.currentPrice), 0))}
                </div>
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
