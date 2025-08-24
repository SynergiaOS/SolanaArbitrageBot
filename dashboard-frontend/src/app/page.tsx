"use client";

import { useWebSocket } from "@/components/providers/websocket-provider";
import { DashboardOverview } from "@/components/dashboard/DashboardOverview";
import { ArbitrageOpportunities } from "@/components/dashboard/ArbitrageOpportunities";
import { SniperActivity } from "@/components/dashboard/SniperActivity";
import { PerformanceMetrics } from "@/components/dashboard/PerformanceMetrics";
import { SystemHealth } from "@/components/dashboard/SystemHealth";
import { ActivePositions } from "@/components/dashboard/ActivePositions";

export default function Dashboard() {
  const {
    isConnected,
    arbitrageOpportunities,
    sniperActivities,
    positions,
    performanceMetrics,
    systemHealth
  } = useWebSocket();

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-foreground">
            Enhanced Trading Dashboard
          </h1>
          <p className="text-muted-foreground">
            Real-time monitoring of arbitrage, sniper, and optimization systems
          </p>
        </div>
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-2">
            <div className={`w-3 h-3 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`}></div>
            <span className="text-sm text-muted-foreground">
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
          {!isConnected && (
            <div className="flex items-center space-x-2">
              <div className="w-3 h-3 rounded-full bg-blue-500"></div>
              <span className="text-sm text-blue-400">
                Demo Mode
              </span>
            </div>
          )}
        </div>
      </div>

      {/* Overview Cards */}
      <DashboardOverview
        performanceMetrics={performanceMetrics}
        systemHealth={systemHealth}
        positionsCount={positions.length}
        opportunitiesCount={arbitrageOpportunities.length}
      />

      {/* Main Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Left Column */}
        <div className="space-y-6">
          <ArbitrageOpportunities opportunities={arbitrageOpportunities} />
          <SniperActivity activities={sniperActivities} />
        </div>

        {/* Right Column */}
        <div className="space-y-6">
          <PerformanceMetrics metrics={performanceMetrics} />
          <ActivePositions positions={positions} />
        </div>
      </div>

      {/* System Health */}
      <SystemHealth health={systemHealth} />
    </div>
  );
}
