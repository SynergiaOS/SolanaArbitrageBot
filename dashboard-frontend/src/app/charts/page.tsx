"use client";

import { useEffect, useState, useRef } from "react";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
} from 'chart.js';
import { Line } from 'react-chartjs-2';
import { CandlestickChart, type Candle } from "@/components/charts/Candles";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend
);

export default function ChartsPage() {
  const [labels, setLabels] = useState<string[]>([]);
  const [raydiumData, setRaydiumData] = useState<number[]>([]);
  const [orcaData, setOrcaData] = useState<number[]>([]);
  const [spreadData, setSpreadData] = useState<number[]>([]);
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const [mode, setMode] = useState<"line"|"candles">("line");
  const [candles, setCandles] = useState<Candle[]>([]);

  useEffect(() => {
    const wsUrl = location.origin.replace('http', 'ws') + '/ws';
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => setConnected(true);
    ws.onclose = () => setConnected(false);
    ws.onerror = () => setConnected(false);

    ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data);

        // Obsługa prawdziwych OHLC jeśli przyjdą
        if (msg.type === 'ohlc') {
          const o = parseFloat(msg.o);
          const h = parseFloat(msg.h);
          const l = parseFloat(msg.l);
          const c = parseFloat(msg.c);
          const ts = new Date(msg.t || msg.timestamp).toLocaleTimeString();
          setLabels(labs => [...labs.slice(-199), ts]);
          setCandles(cs => [...cs.slice(-199), { o, h, l, c }]);
          return;
        }

        if (msg.type === 'price_update') {
          const p = msg.PriceUpdate || msg.price_update || msg;
          const ts = new Date(p.timestamp).toLocaleTimeString();
          const raydiumPrice = parseFloat(p.raydium_price);
          const orcaPrice = parseFloat(p.orca_price);
          const spread = ((orcaPrice - raydiumPrice) / raydiumPrice * 100);

          setLabels(l => [...l.slice(-199), ts]);
          setRaydiumData(d => [...d.slice(-199), raydiumPrice]);
          setOrcaData(d => [...d.slice(-199), orcaPrice]);
          setSpreadData(d => [...d.slice(-199), spread]);

          // fallback: pseudo‑OHLC z ticków
          setCandles(cs => {
            const last = cs[cs.length - 1];
            const c = raydiumPrice;
            if (!last || labels.length % 10 === 0) {
              return [...cs.slice(-199), { o: c, h: c, l: c, c }];
            }
            last.h = Math.max(last.h, c);
            last.l = Math.min(last.l, c);
            last.c = c;
            return [...cs.slice(0, -1), last];
          });
        }
      } catch {}
    };

    return () => ws.close();
  }, []);

  const priceChartData = {
    labels,
    datasets: [
      {
        label: 'Raydium SOL/USDC',
        data: raydiumData,
        borderColor: 'rgb(14, 165, 233)',
        backgroundColor: 'rgba(14, 165, 233, 0.1)',
        tension: 0.1,
      },
      {
        label: 'Orca SOL/USDC',
        data: orcaData,
        borderColor: 'rgb(34, 197, 94)',
        backgroundColor: 'rgba(34, 197, 94, 0.1)',
        tension: 0.1,
      },
    ],
  };

  const spreadChartData = {
    labels,
    datasets: [
      {
        label: 'Price Spread (%)',
        data: spreadData,
        borderColor: 'rgb(239, 68, 68)',
        backgroundColor: 'rgba(239, 68, 68, 0.1)',
        tension: 0.1,
      },
    ],
  };

  const chartOptions = {
    responsive: true,
    plugins: {
      legend: {
        position: 'top' as const,
        labels: { color: '#e5e7eb' }
      },
      title: {
        display: true,
        color: '#e5e7eb'
      },
    },
    scales: {
      x: {
        ticks: { color: '#9ca3af' },
        grid: { color: '#374151' }
      },
      y: {
        ticks: { color: '#9ca3af' },
        grid: { color: '#374151' }
      }
    },
    maintainAspectRatio: false,
  };

  return (
    <div className="h-full flex flex-col">
      {/* Chart header with controls */}
      <div className="flex items-center justify-between p-6 border-b border-[var(--border)]">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-semibold">SOL/USDC Trading</h1>
          <div className="flex items-center gap-2 text-sm text-gray-400">
            <span className={`inline-block w-2 h-2 rounded-full ${connected ? 'bg-emerald-500' : 'bg-rose-500'}`} />
            {connected ? 'Live Data' : 'Disconnected'}
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div className="bg-[var(--panel)] border border-[var(--border)] rounded-lg overflow-hidden">
            <button
              onClick={() => setMode('line')}
              className={`px-4 py-2 text-sm transition-colors ${
                mode === 'line'
                  ? 'bg-[var(--primary)] text-black'
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              Line
            </button>
            <button
              onClick={() => setMode('candles')}
              className={`px-4 py-2 text-sm transition-colors ${
                mode === 'candles'
                  ? 'bg-[var(--primary)] text-black'
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              Candles
            </button>
          </div>
        </div>
      </div>

      {/* Main content area */}
      <div className="flex-1 flex overflow-hidden">
        {/* Chart area */}
        <div className="flex-1 flex flex-col p-6 space-y-6">
          {/* Main chart */}
          <div className="card flex-1 min-h-0">
            <div className="p-6 h-full flex flex-col">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-medium">Price Chart</h2>
                <div className="flex items-center gap-4 text-sm text-gray-400">
                  <span>Last: {raydiumData.at(-1)?.toFixed(4) || '---'}</span>
                  <span className={`${spreadData.at(-1) && spreadData.at(-1)! > 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
                    {spreadData.at(-1)?.toFixed(3) || '---'}%
                  </span>
                </div>
              </div>
              <div className="flex-1 min-h-0">
                {mode === 'line' ? (
                  <Line data={priceChartData} options={{...chartOptions, maintainAspectRatio: false}} />
                ) : (
                  <CandlestickChart labels={labels} candles={candles} overlay={orcaData} title="" />
                )}
              </div>
            </div>
          </div>

          {/* Bottom metrics */}
          <div className="grid grid-cols-4 gap-4">
            <div className="card p-4">
              <div className="text-sm text-gray-400 mb-1">Raydium</div>
              <div className="text-xl font-bold text-sky-300">${raydiumData.at(-1)?.toFixed(4) || '---'}</div>
            </div>
            <div className="card p-4">
              <div className="text-sm text-gray-400 mb-1">Orca</div>
              <div className="text-xl font-bold text-emerald-300">${orcaData.at(-1)?.toFixed(4) || '---'}</div>
            </div>
            <div className="card p-4">
              <div className="text-sm text-gray-400 mb-1">Spread</div>
              <div className="text-xl font-bold text-rose-300">{spreadData.at(-1)?.toFixed(3) || '---'}%</div>
            </div>
            <div className="card p-4">
              <div className="text-sm text-gray-400 mb-1">Volume</div>
              <div className="text-xl font-bold text-gray-300">---</div>
            </div>
          </div>
        </div>

        {/* Right panel */}
        <div className="w-80 border-l border-[var(--border)] p-6 space-y-6 overflow-y-auto">
          {/* Order Book */}
          <div className="card p-4">
            <h3 className="text-lg font-medium mb-4">Order Book</h3>
            <div className="space-y-2">
              <div className="grid grid-cols-3 gap-2 text-xs text-gray-400 pb-2 border-b border-[var(--border)]">
                <span>Price</span>
                <span>Size</span>
                <span>Total</span>
              </div>
              {/* Asks */}
              {[...Array(5)].map((_, i) => (
                <div key={`ask-${i}`} className="grid grid-cols-3 gap-2 text-xs">
                  <span className="text-rose-400">{(246.31 + i * 0.01).toFixed(2)}</span>
                  <span className="text-gray-300">{(Math.random() * 100).toFixed(1)}</span>
                  <span className="text-gray-400">{(Math.random() * 1000).toFixed(0)}</span>
                </div>
              ))}
              <div className="py-2 text-center text-lg font-bold">
                ${raydiumData.at(-1)?.toFixed(2) || '246.31'}
              </div>
              {/* Bids */}
              {[...Array(5)].map((_, i) => (
                <div key={`bid-${i}`} className="grid grid-cols-3 gap-2 text-xs">
                  <span className="text-emerald-400">{(246.30 - i * 0.01).toFixed(2)}</span>
                  <span className="text-gray-300">{(Math.random() * 100).toFixed(1)}</span>
                  <span className="text-gray-400">{(Math.random() * 1000).toFixed(0)}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Trading Controls */}
          <div className="card p-4">
            <h3 className="text-lg font-medium mb-4">Trading Controls</h3>
            <div className="grid grid-cols-2 gap-3 mb-4">
              <button className="px-3 py-2 rounded-md bg-[var(--primary)] text-black hover:shadow-[0_0_24px_#10b98155] transition font-medium">
                Start Bot
              </button>
              <button className="px-3 py-2 rounded-md bg-yellow-600/20 text-yellow-400 border border-yellow-600/50 hover:bg-yellow-600/30 transition">
                Pause
              </button>
              <button className="px-3 py-2 rounded-md bg-gray-600/20 text-gray-400 border border-gray-600/50 hover:bg-gray-600/30 transition">
                Stop
              </button>
              <button className="px-3 py-2 rounded-md bg-rose-600/20 text-rose-400 border border-rose-600/50 hover:bg-rose-600/30 transition">
                Emergency
              </button>
            </div>

            <div className="space-y-3">
              <div>
                <label className="block text-sm text-gray-400 mb-1">Position Size (SOL)</label>
                <input
                  type="number"
                  className="w-full bg-[var(--background)] border border-[var(--border)] rounded px-3 py-2 text-sm"
                  placeholder="0.1"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Min Profit (%)</label>
                <input
                  type="number"
                  className="w-full bg-[var(--background)] border border-[var(--border)] rounded px-3 py-2 text-sm"
                  placeholder="0.5"
                />
              </div>
            </div>
          </div>

          {/* Bot Status */}
          <div className="card p-4">
            <h3 className="text-lg font-medium mb-4">Bot Status</h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">Status:</span>
                <span className={connected ? 'text-emerald-400' : 'text-rose-400'}>
                  {connected ? 'Active' : 'Inactive'}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Trades Today:</span>
                <span className="text-white">0</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">P&L Today:</span>
                <span className="text-emerald-400">+$0.00</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Data Points:</span>
                <span className="text-white">{labels.length}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
