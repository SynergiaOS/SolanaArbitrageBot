"use client";

import { useEffect, useState } from "react";

interface Trade {
  id: number;
  timestamp: string;
  type: string;
  amount_sol: number;
  profit_usd: number;
  from_dex: string;
  to_dex: string;
  signature?: string;
}

export default function HistoryPage() {
  const [trades, setTrades] = useState<Trade[]>([]);
  const [token, setToken] = useState<string>(process.env.NEXT_PUBLIC_DASHBOARD_TOKEN || "your-secret-token-here");
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<'all' | 'profitable' | 'losses'>('all');

  const fetchTrades = async () => {
    try {
      const response = await fetch('/api/trades', {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (response.ok) {
        const data = await response.json();
        setTrades(data);
      }
    } catch (error) {
      console.error('Failed to fetch trades:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTrades();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token]);

  const filteredTrades = trades.filter(trade => {
    if (filter === 'profitable') return trade.profit_usd > 0;
    if (filter === 'losses') return trade.profit_usd < 0;
    return true;
  });

  const totalProfit = trades.reduce((sum, trade) => sum + trade.profit_usd, 0);
  const profitableTrades = trades.filter(t => t.profit_usd > 0).length;
  const winRate = trades.length > 0 ? (profitableTrades / trades.length * 100) : 0;

  const exportTrades = () => {
    const csv = [
      'Timestamp,Type,Amount SOL,Profit USD,From DEX,To DEX,Signature',
      ...filteredTrades.map(trade => 
        `${trade.timestamp},${trade.type},${trade.amount_sol},${trade.profit_usd},${trade.from_dex},${trade.to_dex},${trade.signature || ''}`
      )
    ].join('\n');
    
    const blob = new Blob([csv], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `trades_${new Date().toISOString().split('T')[0]}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <main className="min-h-screen bg-gray-950 text-gray-200 p-6">
      <div className="max-w-7xl mx-auto">
        <header className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-2xl font-semibold">Trading History</h1>
            <p className="text-gray-400 mt-2">View and analyze your trading performance</p>
          </div>
          <button
            onClick={exportTrades}
            className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded font-medium transition-colors"
          >
            Export CSV
          </button>
        </header>

        {/* Summary Stats */}
        <div className="grid md:grid-cols-4 gap-4 mb-6">
          <div className="bg-gray-900 border border-gray-800 rounded p-4">
            <h3 className="text-sm text-gray-400 mb-1">Total Trades</h3>
            <p className="text-2xl font-bold text-blue-400">{trades.length}</p>
          </div>
          <div className="bg-gray-900 border border-gray-800 rounded p-4">
            <h3 className="text-sm text-gray-400 mb-1">Total Profit</h3>
            <p className={`text-2xl font-bold ${totalProfit >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
              ${totalProfit.toFixed(4)}
            </p>
          </div>
          <div className="bg-gray-900 border border-gray-800 rounded p-4">
            <h3 className="text-sm text-gray-400 mb-1">Win Rate</h3>
            <p className="text-2xl font-bold text-purple-400">{winRate.toFixed(1)}%</p>
          </div>
          <div className="bg-gray-900 border border-gray-800 rounded p-4">
            <h3 className="text-sm text-gray-400 mb-1">Profitable Trades</h3>
            <p className="text-2xl font-bold text-emerald-400">{profitableTrades}</p>
          </div>
        </div>

        {/* Filters */}
        <div className="bg-gray-900 border border-gray-800 rounded p-4 mb-6">
          <div className="flex items-center space-x-4">
            <span className="text-sm text-gray-400">Filter:</span>
            <button
              onClick={() => setFilter('all')}
              className={`px-3 py-1 rounded text-sm ${filter === 'all' ? 'bg-blue-600 text-white' : 'bg-gray-700 text-gray-300'}`}
            >
              All Trades
            </button>
            <button
              onClick={() => setFilter('profitable')}
              className={`px-3 py-1 rounded text-sm ${filter === 'profitable' ? 'bg-emerald-600 text-white' : 'bg-gray-700 text-gray-300'}`}
            >
              Profitable
            </button>
            <button
              onClick={() => setFilter('losses')}
              className={`px-3 py-1 rounded text-sm ${filter === 'losses' ? 'bg-rose-600 text-white' : 'bg-gray-700 text-gray-300'}`}
            >
              Losses
            </button>
          </div>
        </div>

        {/* Trades Table */}
        <div className="bg-gray-900 border border-gray-800 rounded overflow-hidden">
          {loading ? (
            <div className="p-8 text-center text-gray-400">Loading trades...</div>
          ) : filteredTrades.length === 0 ? (
            <div className="p-8 text-center text-gray-400">No trades found</div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-gray-800">
                  <tr>
                    <th className="px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase">Time</th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase">Type</th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase">Amount</th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase">Profit/Loss</th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase">Route</th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase">Signature</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-800">
                  {filteredTrades.map((trade) => (
                    <tr key={trade.id} className="hover:bg-gray-800">
                      <td className="px-4 py-3 text-sm text-gray-300">
                        {new Date(trade.timestamp).toLocaleString()}
                      </td>
                      <td className="px-4 py-3 text-sm">
                        <span className="px-2 py-1 rounded text-xs bg-blue-900 text-blue-200">
                          {trade.type}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-sm text-gray-300">
                        {trade.amount_sol.toFixed(4)} SOL
                      </td>
                      <td className="px-4 py-3 text-sm">
                        <span className={`font-medium ${trade.profit_usd >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
                          ${trade.profit_usd.toFixed(4)}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-sm text-gray-300">
                        {trade.from_dex} → {trade.to_dex}
                      </td>
                      <td className="px-4 py-3 text-sm">
                        {trade.signature ? (
                          <a
                            href={`https://solscan.io/tx/${trade.signature}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-blue-400 hover:text-blue-300 font-mono text-xs"
                          >
                            {trade.signature.slice(0, 8)}...
                          </a>
                        ) : (
                          <span className="text-gray-500">-</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* Authentication */}
        <div className="mt-6 bg-gray-900 border border-gray-800 rounded p-4">
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
    </main>
  );
}
