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
    <main className="min-h-screen bg-gray-950 text-gray-200 p-6">
      <div className="max-w-7xl mx-auto">
        <header className="flex items-center justify-between mb-6">
          <h1 className="text-2xl font-semibold">Price Charts</h1>
          <div className={"px-3 py-1 rounded text-sm " + (connected ? 'bg-emerald-900 text-emerald-200' : 'bg-rose-900 text-rose-200')}>
            {connected ? 'Connected' : 'Disconnected'}
          </div>
        </header>

        <div className="grid gap-6">
          <div className="bg-gray-900 border border-gray-800 rounded p-6">
            <h2 className="text-lg font-medium mb-4">SOL/USDC Price Comparison</h2>
            <div className="h-96">
              <Line data={priceChartData} options={{...chartOptions, plugins: {...chartOptions.plugins, title: {display: true, text: 'Raydium vs Orca Prices', color: '#e5e7eb'}}}} />
            </div>
          </div>

          <div className="bg-gray-900 border border-gray-800 rounded p-6">
            <h2 className="text-lg font-medium mb-4">Price Spread Analysis</h2>
            <div className="h-96">
              <Line data={spreadChartData} options={{...chartOptions, plugins: {...chartOptions.plugins, title: {display: true, text: 'Price Spread Percentage', color: '#e5e7eb'}}}} />
            </div>
          </div>

          <div className="grid md:grid-cols-3 gap-4">
            <div className="bg-gray-900 border border-gray-800 rounded p-4">
              <h3 className="font-medium text-gray-400 mb-2">Current Raydium</h3>
              <p className="text-2xl font-bold text-sky-300">${raydiumData.at(-1)?.toFixed(4) || '---'}</p>
            </div>
            <div className="bg-gray-900 border border-gray-800 rounded p-4">
              <h3 className="font-medium text-gray-400 mb-2">Current Orca</h3>
              <p className="text-2xl font-bold text-emerald-300">${orcaData.at(-1)?.toFixed(4) || '---'}</p>
            </div>
            <div className="bg-gray-900 border border-gray-800 rounded p-4">
              <h3 className="font-medium text-gray-400 mb-2">Current Spread</h3>
              <p className="text-2xl font-bold text-rose-300">{spreadData.at(-1)?.toFixed(3) || '---'}%</p>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
