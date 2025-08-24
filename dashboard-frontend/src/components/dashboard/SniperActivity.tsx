"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Zap, Clock, Shield, TrendingUp, AlertTriangle } from "lucide-react";
import { SniperActivity as SniperActivityType } from "@/components/providers/websocket-provider";
import { formatDistanceToNow } from "date-fns";

interface SniperActivityProps {
  activities: SniperActivityType[];
}

export function SniperActivity({ activities }: SniperActivityProps) {
  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
    }).format(value);
  };

  const getActionIcon = (action: string) => {
    switch (action) {
      case 'detected':
        return <Zap className="h-3 w-3" />;
      case 'analyzing':
        return <Shield className="h-3 w-3" />;
      case 'sniped':
        return <TrendingUp className="h-3 w-3" />;
      case 'rejected':
        return <AlertTriangle className="h-3 w-3" />;
      default:
        return <Clock className="h-3 w-3" />;
    }
  };

  const getActionColor = (action: string) => {
    switch (action) {
      case 'detected':
        return 'bg-blue-500';
      case 'analyzing':
        return 'bg-yellow-500';
      case 'sniped':
        return 'bg-green-500';
      case 'rejected':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getActionBadgeVariant = (action: string) => {
    switch (action) {
      case 'sniped':
        return 'default';
      case 'rejected':
        return 'destructive';
      case 'analyzing':
        return 'secondary';
      default:
        return 'outline';
    }
  };

  const getRiskColor = (riskScore: number) => {
    if (riskScore <= 0.3) return 'text-green-600';
    if (riskScore <= 0.6) return 'text-yellow-600';
    return 'text-red-600';
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Zap className="h-5 w-5" />
          Sniper Activity
          <Badge variant="secondary">{activities.length}</Badge>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {activities.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No sniper activity detected
            </div>
          ) : (
            activities.slice(0, 10).map((activity) => (
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
                      <Badge variant={getActionBadgeVariant(activity.action)} className="text-xs">
                        {getActionIcon(activity.action)}
                        {activity.action}
                      </Badge>
                    </div>
                    
                    <div className="flex items-center gap-4 text-sm text-muted-foreground">
                      <div className="flex items-center gap-1">
                        <Shield className="h-3 w-3" />
                        <span className={getRiskColor(activity.riskScore)}>
                          Risk: {(activity.riskScore * 100).toFixed(0)}%
                        </span>
                      </div>
                      
                      <div>
                        Liquidity: {activity.liquiditySol.toFixed(1)} SOL
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
                    {activity.profitMultiplier && ` • ${activity.profitMultiplier.toFixed(1)}x`}
                  </div>
                </div>
              </div>
            ))
          )}
        </div>

        {activities.length > 10 && (
          <div className="text-center mt-4 text-sm text-muted-foreground">
            Showing 10 of {activities.length} activities
          </div>
        )}
      </CardContent>
    </Card>
  );
}
