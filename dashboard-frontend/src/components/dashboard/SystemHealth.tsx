"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Shield, Server, Wifi, Wallet, Clock, AlertTriangle } from "lucide-react";
import { SystemHealth as SystemHealthType } from "@/components/providers/websocket-provider";

interface SystemHealthProps {
  health: SystemHealthType | null;
}

export function SystemHealth({ health }: SystemHealthProps) {
  const formatUptime = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'online':
      case 'connected':
        return 'text-green-600';
      case 'offline':
      case 'disconnected':
        return 'text-red-600';
      case 'error':
        return 'text-red-600';
      case 'slow':
        return 'text-yellow-600';
      default:
        return 'text-gray-600';
    }
  };

  const getStatusBadgeVariant = (status: string) => {
    switch (status) {
      case 'online':
      case 'connected':
        return 'default';
      case 'offline':
      case 'disconnected':
      case 'error':
        return 'destructive';
      case 'slow':
        return 'secondary';
      default:
        return 'outline';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'online':
      case 'connected':
        return <Shield className="h-4 w-4 text-green-600" />;
      case 'offline':
      case 'disconnected':
      case 'error':
        return <AlertTriangle className="h-4 w-4 text-red-600" />;
      case 'slow':
        return <Clock className="h-4 w-4 text-yellow-600" />;
      default:
        return <Server className="h-4 w-4 text-gray-600" />;
    }
  };

  const getMemoryUsageColor = (usage: number) => {
    if (usage < 70) return 'text-green-600';
    if (usage < 85) return 'text-yellow-600';
    return 'text-red-600';
  };

  const getSystemLoadColor = (load: number) => {
    if (load < 0.7) return 'text-green-600';
    if (load < 0.9) return 'text-yellow-600';
    return 'text-red-600';
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Shield className="h-5 w-5" />
          System Health
          {health && (
            <Badge variant={getStatusBadgeVariant(health.botStatus)}>
              {health.botStatus}
            </Badge>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent>
        {!health ? (
          <div className="text-center py-8 text-muted-foreground">
            No system health data available
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            {/* Bot Status */}
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                {getStatusIcon(health.botStatus)}
                <span className="text-sm font-medium">Bot Status</span>
              </div>
              <div className={`text-lg font-bold ${getStatusColor(health.botStatus)}`}>
                {health.botStatus.charAt(0).toUpperCase() + health.botStatus.slice(1)}
              </div>
              <div className="text-xs text-muted-foreground">
                Uptime: {formatUptime(health.uptime)}
              </div>
            </div>

            {/* RPC Status */}
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <Wifi className="h-4 w-4" />
                <span className="text-sm font-medium">RPC Status</span>
              </div>
              <div className={`text-lg font-bold ${getStatusColor(health.rpcStatus)}`}>
                {health.rpcStatus.charAt(0).toUpperCase() + health.rpcStatus.slice(1)}
              </div>
              <div className="text-xs text-muted-foreground">
                Connection quality
              </div>
            </div>

            {/* Wallet Balance */}
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <Wallet className="h-4 w-4" />
                <span className="text-sm font-medium">Wallet Balance</span>
              </div>
              <div className="text-lg font-bold">
                {health.walletBalance.toFixed(2)} SOL
              </div>
              <div className="text-xs text-muted-foreground">
                Available for trading
              </div>
            </div>

            {/* System Load */}
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <Server className="h-4 w-4" />
                <span className="text-sm font-medium">System Load</span>
              </div>
              <div className={`text-lg font-bold ${getSystemLoadColor(health.systemLoad)}`}>
                {(health.systemLoad * 100).toFixed(0)}%
              </div>
              <div className="text-xs text-muted-foreground">
                CPU utilization
              </div>
            </div>

            {/* Memory Usage */}
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <Server className="h-4 w-4" />
                <span className="text-sm font-medium">Memory Usage</span>
              </div>
              <div className={`text-lg font-bold ${getMemoryUsageColor(health.memoryUsage)}`}>
                {health.memoryUsage.toFixed(0)}%
              </div>
              <div className="text-xs text-muted-foreground">
                RAM utilization
              </div>
            </div>

            {/* Last Update */}
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <Clock className="h-4 w-4" />
                <span className="text-sm font-medium">Last Update</span>
              </div>
              <div className="text-lg font-bold">
                {new Date(health.lastUpdate).toLocaleTimeString()}
              </div>
              <div className="text-xs text-muted-foreground">
                Data freshness
              </div>
            </div>

            {/* Health Score */}
            <div className="space-y-2 md:col-span-2">
              <div className="flex items-center gap-2">
                <Shield className="h-4 w-4" />
                <span className="text-sm font-medium">Overall Health</span>
              </div>
              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span>Health Score</span>
                  <span className="font-medium">
                    {health.botStatus === 'online' && health.rpcStatus === 'connected' && 
                     health.systemLoad < 0.8 && health.memoryUsage < 80 ? '95%' : 
                     health.botStatus === 'online' ? '75%' : '25%'}
                  </span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2 dark:bg-gray-700">
                  <div 
                    className={`h-2 rounded-full ${
                      health.botStatus === 'online' && health.rpcStatus === 'connected' && 
                      health.systemLoad < 0.8 && health.memoryUsage < 80 ? 'bg-green-600' : 
                      health.botStatus === 'online' ? 'bg-yellow-600' : 'bg-red-600'
                    }`}
                    style={{ 
                      width: `${health.botStatus === 'online' && health.rpcStatus === 'connected' && 
                               health.systemLoad < 0.8 && health.memoryUsage < 80 ? '95%' : 
                               health.botStatus === 'online' ? '75%' : '25%'}` 
                    }}
                  ></div>
                </div>
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
