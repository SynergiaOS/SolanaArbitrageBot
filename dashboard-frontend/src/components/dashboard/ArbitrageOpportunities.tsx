"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { TrendingUp, Clock, DollarSign } from "lucide-react";
import { ArbitrageOpportunity } from "@/components/providers/websocket-provider";
import { formatDistanceToNow } from "date-fns";

interface ArbitrageOpportunitiesProps {
  opportunities: ArbitrageOpportunity[];
}

export function ArbitrageOpportunities({ opportunities }: ArbitrageOpportunitiesProps) {
  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
    }).format(value);
  };

  const formatPercentage = (value: number) => {
    return `${value.toFixed(2)}%`;
  };

  const getSpreadColor = (spread: number) => {
    if (spread >= 2) return 'bg-green-500';
    if (spread >= 1) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <TrendingUp className="h-5 w-5" />
          Arbitrage Opportunities
          <Badge variant="secondary">{opportunities.length}</Badge>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {opportunities.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No arbitrage opportunities detected
            </div>
          ) : (
            opportunities.slice(0, 10).map((opportunity) => (
              <div
                key={opportunity.id}
                className="flex items-center justify-between p-3 border rounded-lg hover:bg-accent/50 transition-colors"
              >
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="font-medium">
                      {opportunity.tokenA}/{opportunity.tokenB}
                    </span>
                    <Badge variant="outline" className="text-xs">
                      {opportunity.dexA} → {opportunity.dexB}
                    </Badge>
                  </div>
                  
                  <div className="flex items-center gap-4 text-sm text-muted-foreground">
                    <div className="flex items-center gap-1">
                      <DollarSign className="h-3 w-3" />
                      <span>{formatCurrency(opportunity.profitUsd)}</span>
                    </div>
                    
                    <div className="flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      <span>
                        {formatDistanceToNow(new Date(opportunity.timestamp), { addSuffix: true })}
                      </span>
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-3">
                  <div className="text-right">
                    <div className="text-sm font-medium">
                      {formatPercentage(opportunity.spread)}
                    </div>
                    <div className="text-xs text-muted-foreground">spread</div>
                  </div>
                  
                  <div className={`w-2 h-8 rounded-full ${getSpreadColor(opportunity.spread)}`}></div>
                </div>
              </div>
            ))
          )}
        </div>

        {opportunities.length > 10 && (
          <div className="text-center mt-4 text-sm text-muted-foreground">
            Showing 10 of {opportunities.length} opportunities
          </div>
        )}
      </CardContent>
    </Card>
  );
}
