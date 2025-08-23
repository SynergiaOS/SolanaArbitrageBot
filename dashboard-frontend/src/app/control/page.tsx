"use client";

import { useEffect, useState } from "react";
import { getApiBase } from "@/lib/api";

interface BotStatus {
  status: string;
  uptime_seconds: number;
  trades_today: number;
  profit_today: number;
  last_trade?: string;
}

export default function ControlPage() {
  const [botStatus, setBotStatus] = useState<BotStatus | null>(null);
  const [token, setToken] = useState<string>(process.env.NEXT_PUBLIC_DASHBOARD_TOKEN || "your-secret-token-here");
  const [loading, setLoading] = useState<string | null>(null);

  const fetchStatus = async () => {
    try {
      const url = `${getApiBase()}/api/status`;
      const response = await fetch(url, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (response.ok) {
        const data = await response.json();
        setBotStatus(data);
      }
    } catch (error) {
      console.error('Failed to fetch status:', error);
    }
  };

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 5000);
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token]);

  const executeControl = async (action: string) => {
    setLoading(action);
    try {
      const url = `${getApiBase()}/api/control/${action}`;
      const response = await fetch(url, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      });

      if (response.ok) {
        await fetchStatus(); // Refresh status after action
      } else {
        alert(`Failed to ${action} bot`);
      }
    } catch (error) {
      alert(`Error: ${error}`);
    } finally {
      setLoading(null);
    }
  };

  const formatUptime = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  };

  return (
    <main className="min-h-screen bg-gray-950 text-gray-200 p-6">
      <div className="max-w-4xl mx-auto">
        <header className="mb-6">
          <h1 className="text-2xl font-semibold">Bot Control Panel</h1>
          <p className="text-gray-400 mt-2">Monitor and control your Solana arbitrage bot</p>
        </header>

        <div className="grid gap-6">
          {/* Status Overview */}
          <div className="bg-gray-900 border border-gray-800 rounded p-6">
            <h2 className="text-lg font-medium mb-4">Current Status</h2>
            {botStatus ? (
              <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-4">
                <div className="bg-gray-800 rounded p-4">
                  <h3 className="text-sm text-gray-400 mb-1">Status</h3>
                  <p className={`text-lg font-semibold ${
                    botStatus.status === 'running' ? 'text-emerald-400' : 
                    botStatus.status === 'paused' ? 'text-yellow-400' : 'text-rose-400'
                  }`}>
                    {botStatus.status.toUpperCase()}
                  </p>
                </div>
                <div className="bg-gray-800 rounded p-4">
                  <h3 className="text-sm text-gray-400 mb-1">Uptime</h3>
                  <p className="text-lg font-semibold text-blue-400">
                    {formatUptime(botStatus.uptime_seconds)}
                  </p>
                </div>
                <div className="bg-gray-800 rounded p-4">
                  <h3 className="text-sm text-gray-400 mb-1">Trades Today</h3>
                  <p className="text-lg font-semibold text-purple-400">
                    {botStatus.trades_today}
                  </p>
                </div>
                <div className="bg-gray-800 rounded p-4">
                  <h3 className="text-sm text-gray-400 mb-1">Profit Today</h3>
                  <p className={`text-lg font-semibold ${
                    botStatus.profit_today >= 0 ? 'text-emerald-400' : 'text-rose-400'
                  }`}>
                    ${botStatus.profit_today.toFixed(4)}
                  </p>
                </div>
              </div>
            ) : (
              <div className="text-gray-400">Loading status...</div>
            )}
          </div>

          {/* Control Buttons */}
          <div className="bg-gray-900 border border-gray-800 rounded p-6">
            <h2 className="text-lg font-medium mb-4">Bot Controls</h2>
            <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-4">
              <button
                onClick={() => executeControl('start')}
                disabled={loading === 'start'}
                className="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white px-4 py-3 rounded font-medium transition-colors"
              >
                {loading === 'start' ? 'Starting...' : 'Start Bot'}
              </button>
              <button
                onClick={() => executeControl('pause')}
                disabled={loading === 'pause'}
                className="bg-yellow-600 hover:bg-yellow-700 disabled:opacity-50 text-white px-4 py-3 rounded font-medium transition-colors"
              >
                {loading === 'pause' ? 'Pausing...' : 'Pause Bot'}
              </button>
              <button
                onClick={() => executeControl('stop')}
                disabled={loading === 'stop'}
                className="bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white px-4 py-3 rounded font-medium transition-colors"
              >
                {loading === 'stop' ? 'Stopping...' : 'Stop Bot'}
              </button>
              <button
                onClick={() => executeControl('emergency')}
                disabled={loading === 'emergency'}
                className="bg-rose-600 hover:bg-rose-700 disabled:opacity-50 text-white px-4 py-3 rounded font-medium transition-colors"
              >
                {loading === 'emergency' ? 'Emergency...' : 'Emergency Stop'}
              </button>
            </div>
          </div>

          {/* Authentication */}
          <div className="bg-gray-900 border border-gray-800 rounded p-6">
            <h2 className="text-lg font-medium mb-4">Authentication</h2>
            <div className="space-y-3">
              <label className="block text-sm text-gray-400">API Token</label>
              <input
                type="password"
                value={token}
                onChange={(e) => setToken(e.target.value)}
                className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-gray-200 focus:border-blue-500 focus:outline-none"
                placeholder="Enter your dashboard token"
              />
              <p className="text-xs text-gray-500">
                This token is used for API authentication. Keep it secure.
              </p>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
