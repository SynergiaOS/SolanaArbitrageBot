"use client";

import { useEffect, useRef, useState } from "react";
import { getApiBase } from "@/lib/api";

export default function Home() {
  const [status, setStatus] = useState<string>("Connecting...");
  const [connected, setConnected] = useState(false);
  const [token, setToken] = useState<string>(process.env.NEXT_PUBLIC_DASHBOARD_TOKEN || "dev-token");
  const [labels, setLabels] = useState<string[]>([]);
  const [raydium, setRaydium] = useState<number[]>([]);
  const [orca, setOrca] = useState<number[]>([]);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    const apiBase = process.env.NEXT_PUBLIC_API_BASE || '';
    const wsBase = process.env.NEXT_PUBLIC_WS_BASE || '';
    const wsUrl = (wsBase || location.origin.replace('http', 'ws')) + '/ws';
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => { setConnected(true); setStatus('Connected'); };
    ws.onclose = () => { setConnected(false); setStatus('Disconnected'); setTimeout(() => location.reload(), 2000); };
    ws.onerror = () => { setConnected(false); setStatus('Error'); };

    ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data);
        if (msg.type === 'price_update') {
          const p = msg.PriceUpdate || msg.price_update || msg;
          const ts = new Date(p.timestamp).toLocaleTimeString();
          setLabels(l => [...l.slice(-99), ts]);
          setRaydium(d => [...d.slice(-99), parseFloat(p.raydium_price)]);
          setOrca(d => [...d.slice(-99), parseFloat(p.orca_price)]);
        }
      } catch {}
    };

    return () => { ws.close(); };
  }, []);

  async function control(action: string) {
    const url = `${getApiBase()}/api/control/${action}`;
    await fetch(url, { method: 'POST', headers: { 'Authorization': `Bearer ${token}` } });
  }

  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold mb-2">Dashboard Overview</h1>
        <p className="text-gray-400">Monitor your Solana arbitrage bot performance</p>
      </div>

      <div className="grid lg:grid-cols-2 gap-6">
        <div className="card p-6">
          <h3 className="text-lg font-medium mb-4">Live Price Feed</h3>
          <div className="text-sm text-gray-400 mb-4">
            Last Update: {labels.at(-1) || 'No data'} | Data Points: {labels.length}
          </div>
          <PriceChart labels={labels} raydium={raydium} orca={orca} />
        </div>

        <div className="card p-6">
          <h3 className="text-lg font-medium mb-4">Quick Controls</h3>
          <div className="grid grid-cols-2 gap-3 mb-6">
            <button
              className="px-4 py-3 rounded-lg bg-[var(--primary)] text-black hover:shadow-[0_0_24px_#10b98155] transition font-medium"
              onClick={() => control('start')}
            >
              Start Bot
            </button>
            <button
              className="px-4 py-3 rounded-lg bg-yellow-600/20 text-yellow-400 border border-yellow-600/50 hover:bg-yellow-600/30 transition"
              onClick={() => control('pause')}
            >
              Pause
            </button>
            <button
              className="px-4 py-3 rounded-lg bg-gray-600/20 text-gray-400 border border-gray-600/50 hover:bg-gray-600/30 transition"
              onClick={() => control('stop')}
            >
              Stop
            </button>
            <button
              className="px-4 py-3 rounded-lg bg-rose-600/20 text-rose-400 border border-rose-600/50 hover:bg-rose-600/30 transition"
              onClick={() => control('emergency')}
            >
              Emergency
            </button>
          </div>

          <div className="space-y-3">
            <div>
              <label className="block text-sm text-gray-400 mb-1">API Token</label>
              <input
                className="w-full bg-[var(--background)] border border-[var(--border)] rounded px-3 py-2"
                value={token}
                onChange={e => setToken(e.target.value)}
                type="password"
              />
              <p className="text-xs text-gray-500 mt-1">Used for API authentication</p>
            </div>
          </div>
        </div>
      </div>

      {/* Status Cards */}
      <div className="grid lg:grid-cols-4 gap-4 mt-6">
        <div className="card p-4">
          <div className="text-sm text-gray-400 mb-1">Connection Status</div>
          <div className={`text-lg font-semibold ${connected ? 'text-emerald-400' : 'text-rose-400'}`}>
            {status}
          </div>
        </div>
        <div className="card p-4">
          <div className="text-sm text-gray-400 mb-1">Bot Status</div>
          <div className="text-lg font-semibold text-gray-300">Inactive</div>
        </div>
        <div className="card p-4">
          <div className="text-sm text-gray-400 mb-1">Today&rsquo;s Trades</div>
          <div className="text-lg font-semibold text-gray-300">0</div>
        </div>
        <div className="card p-4">
          <div className="text-sm text-gray-400 mb-1">Today&rsquo;s P&L</div>
          <div className="text-lg font-semibold text-emerald-400">+$0.00</div>
        </div>
      </div>
    </div>
  );
}

function PriceChart({ labels, raydium, orca }: { labels: string[]; raydium: number[]; orca: number[] }) {
  return (
    <div className="h-64 bg-gray-950 border border-gray-800 rounded p-2 overflow-auto text-xs">
      {labels.slice(-20).map((l, i) => (
        <div key={i} className="grid grid-cols-3 gap-2 text-gray-300">
          <div className="text-gray-500">{l}</div>
          <div className="text-sky-300">Raydium: {raydium.at(-20 + i)?.toFixed(4)}</div>
          <div className="text-emerald-300">Orca: {orca.at(-20 + i)?.toFixed(4)}</div>
        </div>
      ))}
    </div>
  );
}
