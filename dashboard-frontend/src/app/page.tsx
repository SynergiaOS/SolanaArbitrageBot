"use client";

import { useEffect, useRef, useState } from "react";

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
    await fetch(`/api/control/${action}`, { method: 'POST', headers: { 'Authorization': `Bearer ${token}` } });
  }

  return (
    <main className="min-h-screen bg-gray-950 text-gray-200">
      <div className="max-w-6xl mx-auto p-6">
        <header className="flex items-center justify-between mb-6">
          <h1 className="text-2xl font-semibold">Solana Arbitrage Bot Dashboard</h1>
          <div className={"px-3 py-1 rounded text-sm " + (connected ? 'bg-emerald-900 text-emerald-200' : 'bg-rose-900 text-rose-200')}>
            {status}
          </div>
        </header>

        <section className="grid md:grid-cols-2 gap-4">
          <div className="bg-gray-900 border border-gray-800 rounded p-4">
            <h3 className="font-medium mb-2">Prices (Raydium vs Orca)</h3>
            <div className="text-xs text-gray-400 mb-2">Last: {labels.at(-1) || '-'} | Points: {labels.length}</div>
            <PriceChart labels={labels} raydium={raydium} orca={orca} />
          </div>

          <div className="bg-gray-900 border border-gray-800 rounded p-4">
            <h3 className="font-medium mb-2">Controls</h3>
            <div className="flex gap-2 flex-wrap mb-3">
              <button className="px-3 py-2 rounded border border-gray-700 hover:bg-gray-800" onClick={() => control('start')}>Start</button>
              <button className="px-3 py-2 rounded border border-gray-700 hover:bg-gray-800" onClick={() => control('pause')}>Pause</button>
              <button className="px-3 py-2 rounded border border-gray-700 hover:bg-gray-800" onClick={() => control('stop')}>Stop</button>
              <button className="px-3 py-2 rounded border border-rose-500 text-rose-400 hover:bg-gray-800" onClick={() => control('emergency')}>Emergency Stop</button>
            </div>
            <div className="space-y-2">
              <label className="block text-sm text-gray-400">API Token</label>
              <input className="w-full bg-gray-950 border border-gray-800 rounded p-2" value={token} onChange={e=>setToken(e.target.value)} />
              <p className="text-xs text-gray-500">Authorization: Bearer {'<token>'}</p>
            </div>
          </div>
        </section>
      </div>
    </main>
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
