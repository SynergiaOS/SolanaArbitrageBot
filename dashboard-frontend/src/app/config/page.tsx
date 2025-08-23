"use client";

import { useEffect, useState } from "react";
import { getApiBase } from "@/lib/api";

interface BotConfig {
  min_profit_usd: number;
  max_position_sol: number;
  max_daily_trades: number;
  max_daily_loss_usd: number;
  enabled: boolean;
}

export default function ConfigPage() {
  const [config, setConfig] = useState<BotConfig | null>(null);
  const [token, setToken] = useState<string>(process.env.NEXT_PUBLIC_DASHBOARD_TOKEN || "your-secret-token-here");
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const fetchConfig = async () => {
    try {
      const url = `${getApiBase()}/api/config`;
      const response = await fetch(url, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (response.ok) {
        const data = await response.json();
        setConfig(data);
      }
    } catch (error) {
      console.error('Failed to fetch config:', error);
    }
  };

  useEffect(() => {
    fetchConfig();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token]);

  const saveConfig = async () => {
    if (!config) return;

    setSaving(true);
    setMessage(null);

    try {
      const url = `${getApiBase()}/api/config`;
      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`
        },
        body: JSON.stringify(config)
      });

      if (response.ok) {
        setMessage('Configuration saved successfully!');
        setTimeout(() => setMessage(null), 3000);
      } else {
        setMessage('Failed to save configuration');
      }
    } catch (error) {
      setMessage(`Error: ${error}`);
    } finally {
      setSaving(false);
    }
  };

  const updateConfig = (field: keyof BotConfig, value: number | boolean) => {
    if (!config) return;
    setConfig({ ...config, [field]: value });
  };

  if (!config) {
    return (
      <main className="min-h-screen bg-gray-950 text-gray-200 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center">Loading configuration...</div>
        </div>
      </main>
    );
  }

  return (
    <main className="min-h-screen bg-gray-950 text-gray-200 p-6">
      <div className="max-w-4xl mx-auto">
        <header className="mb-6">
          <h1 className="text-2xl font-semibold">Bot Configuration</h1>
          <p className="text-gray-400 mt-2">Adjust bot parameters and trading limits</p>
        </header>

        <div className="space-y-6">
          {/* Trading Parameters */}
          <div className="bg-gray-900 border border-gray-800 rounded p-6">
            <h2 className="text-lg font-medium mb-4">Trading Parameters</h2>
            <div className="grid md:grid-cols-2 gap-6">
              <div>
                <label className="block text-sm text-gray-400 mb-2">Minimum Profit (USD)</label>
                <input
                  type="number"
                  step="0.01"
                  value={config.min_profit_usd}
                  onChange={(e) => updateConfig('min_profit_usd', parseFloat(e.target.value))}
                  className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-gray-200 focus:border-blue-500 focus:outline-none"
                />
                <p className="text-xs text-gray-500 mt-1">Minimum profit required to execute a trade</p>
              </div>
              
              <div>
                <label className="block text-sm text-gray-400 mb-2">Maximum Position (SOL)</label>
                <input
                  type="number"
                  step="0.01"
                  value={config.max_position_sol}
                  onChange={(e) => updateConfig('max_position_sol', parseFloat(e.target.value))}
                  className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-gray-200 focus:border-blue-500 focus:outline-none"
                />
                <p className="text-xs text-gray-500 mt-1">Maximum SOL amount per trade</p>
              </div>
            </div>
          </div>

          {/* Risk Management */}
          <div className="bg-gray-900 border border-gray-800 rounded p-6">
            <h2 className="text-lg font-medium mb-4">Risk Management</h2>
            <div className="grid md:grid-cols-2 gap-6">
              <div>
                <label className="block text-sm text-gray-400 mb-2">Max Daily Trades</label>
                <input
                  type="number"
                  value={config.max_daily_trades}
                  onChange={(e) => updateConfig('max_daily_trades', parseInt(e.target.value))}
                  className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-gray-200 focus:border-blue-500 focus:outline-none"
                />
                <p className="text-xs text-gray-500 mt-1">Maximum number of trades per day</p>
              </div>
              
              <div>
                <label className="block text-sm text-gray-400 mb-2">Max Daily Loss (USD)</label>
                <input
                  type="number"
                  step="0.01"
                  value={config.max_daily_loss_usd}
                  onChange={(e) => updateConfig('max_daily_loss_usd', parseFloat(e.target.value))}
                  className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-gray-200 focus:border-blue-500 focus:outline-none"
                />
                <p className="text-xs text-gray-500 mt-1">Maximum loss allowed per day</p>
              </div>
            </div>
          </div>

          {/* Bot Status */}
          <div className="bg-gray-900 border border-gray-800 rounded p-6">
            <h2 className="text-lg font-medium mb-4">Bot Status</h2>
            <div className="flex items-center space-x-3">
              <input
                type="checkbox"
                id="enabled"
                checked={config.enabled}
                onChange={(e) => updateConfig('enabled', e.target.checked)}
                className="w-4 h-4 text-blue-600 bg-gray-800 border-gray-600 rounded focus:ring-blue-500"
              />
              <label htmlFor="enabled" className="text-gray-200">
                Enable Bot Trading
              </label>
            </div>
            <p className="text-xs text-gray-500 mt-2">
              When disabled, the bot will not execute any trades
            </p>
          </div>

          {/* Authentication */}
          <div className="bg-gray-900 border border-gray-800 rounded p-6">
            <h2 className="text-lg font-medium mb-4">Authentication</h2>
            <div>
              <label className="block text-sm text-gray-400 mb-2">API Token</label>
              <input
                type="password"
                value={token}
                onChange={(e) => setToken(e.target.value)}
                className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-gray-200 focus:border-blue-500 focus:outline-none"
                placeholder="Enter your dashboard token"
              />
            </div>
          </div>

          {/* Save Button */}
          <div className="flex items-center justify-between">
            <button
              onClick={saveConfig}
              disabled={saving}
              className="bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white px-6 py-3 rounded font-medium transition-colors"
            >
              {saving ? 'Saving...' : 'Save Configuration'}
            </button>
            
            {message && (
              <div className={`px-4 py-2 rounded text-sm ${
                message.includes('success') ? 'bg-emerald-900 text-emerald-200' : 'bg-rose-900 text-rose-200'
              }`}>
                {message}
              </div>
            )}
          </div>
        </div>
      </div>
    </main>
  );
}
