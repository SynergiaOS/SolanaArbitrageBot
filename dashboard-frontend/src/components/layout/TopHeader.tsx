"use client";

import { useState, useEffect } from "react";
import { 
  Wifi, 
  WifiOff, 
  Bell, 
  User, 
  ChevronDown,
  Activity
} from "lucide-react";

export function TopHeader() {
  const [connected, setConnected] = useState(false);
  const [currentTime, setCurrentTime] = useState(new Date());

  useEffect(() => {
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
          connected 
            ? 'bg-emerald-900/50 text-emerald-200 border border-emerald-700/50' 
            : 'bg-rose-900/50 text-rose-200 border border-rose-700/50'
        }`}>
          {connected ? <Wifi className="w-4 h-4" /> : <WifiOff className="w-4 h-4" />}
          {connected ? 'Connected' : 'Disconnected'}
        </div>
      </div>

      {/* Right section */}
      <div className="flex items-center gap-4">
        {/* Current time */}
        <div className="text-sm text-gray-400">
          {currentTime.toLocaleTimeString()}
        </div>

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
