"use client";

import { useState, useEffect } from "react";
import { useWebSocket } from "@/components/providers/websocket-provider";
import { WalletStatus } from "@/components/wallet/WalletButton";
import {
  Wifi,
  WifiOff,
  Bell,
  User,
  ChevronDown,
  Activity
} from "lucide-react";

export function TopHeader() {
  const { isConnected, systemHealth } = useWebSocket();
  const [currentTime, setCurrentTime] = useState<Date | null>(null);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
    setCurrentTime(new Date());
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  // Connect to WebSocket to get real connection status
  useEffect(() => {
    const wsUrl = location.origin.replace('http', 'ws') + '/ws';
    const ws = new WebSocket(wsUrl);

    ws.onopen = () => setConnected(true);
    ws.onclose = () => setConnected(false);
    ws.onerror = () => setConnected(false);

    return () => ws.close();
  }, []);

  return (
    <header className="h-16 bg-[var(--panel)] border-b border-[var(--border)] flex items-center justify-between px-6">
      {/* Left section */}
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <Activity className="w-5 h-5 text-[var(--primary)]" />
          <span className="text-lg font-semibold">Arbitrage Dashboard</span>
        </div>
        
        {/* Connection status */}
        <div className={`flex items-center gap-2 px-3 py-1 rounded-full text-sm ${
          isConnected
            ? 'bg-emerald-900/50 text-emerald-200 border border-emerald-700/50'
            : 'bg-rose-900/50 text-rose-200 border border-rose-700/50'
        }`}>
          {isConnected ? <Wifi className="w-4 h-4" /> : <WifiOff className="w-4 h-4" />}
          {isConnected ? 'Connected' : 'Disconnected'}
        </div>
      </div>

      {/* Right section */}
      <div className="flex items-center gap-4">
        {/* Current time */}
        <div className="text-sm text-gray-400" suppressHydrationWarning>
          {mounted && currentTime ? currentTime.toLocaleTimeString() : '--:--:--'}
        </div>



        {/* Wallet Status */}
        <WalletStatus />

        {/* System Health */}
        {systemHealth && (
          <div className="flex items-center gap-2">
            <Activity className="w-4 h-4 text-blue-400" />
            <span className="text-sm text-gray-400">
              Bot: {systemHealth.walletBalance.toFixed(2)} SOL
            </span>
          </div>
        )}

        {/* Notifications */}
        <button className="p-2 rounded-lg hover:bg-white/5 transition-colors relative">
          <Bell className="w-5 h-5 text-gray-400" />
          <span className="absolute -top-1 -right-1 w-3 h-3 bg-[var(--primary)] rounded-full text-xs"></span>
        </button>

        {/* User menu */}
        <div className="flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-white/5 transition-colors cursor-pointer">
          <div className="w-8 h-8 rounded-full bg-[var(--primary)]/20 flex items-center justify-center">
            <User className="w-4 h-4 text-[var(--primary)]" />
          </div>
          <span className="text-sm font-medium">Trader</span>
          <ChevronDown className="w-4 h-4 text-gray-400" />
        </div>
      </div>
    </header>
  );
}
