/* eslint-disable @typescript-eslint/no-explicit-any */

"use client";

import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  Tooltip,
  Legend,
  ChartData,
  ChartOptions,
  ChartDataset,
} from "chart.js";
import { CandlestickController, CandlestickElement } from "chartjs-chart-financial";
import { Chart as ReactChart } from "react-chartjs-2";

ChartJS.register(
  CategoryScale,
  LinearScale,
  Tooltip,
  Legend,
  CandlestickController,
  CandlestickElement,
);

export type Candle = { o: number; h: number; l: number; c: number };

export function CandlestickChart({
  labels,
  candles,
  overlay,
  title,
}: {
  labels: string[];
  candles: Candle[];
  overlay?: number[];
  title?: string;
}) {
  const datasetCandles = candles.map((c, i) => ({ x: (labels[i] ?? i) as unknown as number, ...c }));
  const data: ChartData<'candlestick' | 'line'> = {
    labels,
    datasets: [
      {
        label: "Raydium",
        data: datasetCandles as any,
        type: "candlestick",
        borderColor: "#10b981",
      },
    ],
  };

  if (overlay && overlay.length) {
    const overlayDs: ChartDataset<'line', number[]> = {
      label: "Orca (close)",
      data: overlay,
      type: "line",
      borderColor: "#06b6d4",
      tension: 0.2,
      pointRadius: 0,
    };
    (data.datasets as (ChartDataset<'candlestick'| 'line', any>)[]).push(overlayDs as any);
  }

  const options: ChartOptions<'candlestick' | 'line'> = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { labels: { color: "#e5e7eb" } },
      title: { display: !!title, text: title, color: "#e5e7eb" },
    },
    scales: {
      x: { ticks: { color: "#9ca3af" }, grid: { color: "var(--border)" } },
      y: { ticks: { color: "#9ca3af" }, grid: { color: "var(--border)" } },
    },
  };

  return <ReactChart type={'candlestick'} data={data as any} options={options} />;
}

