# 🚀 Enhanced Solana Arbitrage Bot Dashboard

A comprehensive, real-time dashboard for monitoring and controlling the Enhanced Solana Arbitrage Bot with advanced trading components.

## ✨ Features

### 🎯 **Real-time Trading Dashboard**
- Live arbitrage opportunities display
- Sniper bot activity monitoring
- Current positions and P&L tracking
- Performance metrics (success rate, ROI, Sharpe ratio)

### 🧬 **Enhanced Components Monitoring**
- **Sniper Bot**: Active snipes, rug detection alerts, profit taking status
- **GEPA Optimizer**: Current generation, fitness scores, parameter evolution
- **Kestra Workflows**: Active workflows, execution status, scheduling

### 🛡️ **Risk Management Interface**
- Real-time risk metrics
- Position sizing controls
- Stop-loss and take-profit management
- Daily/weekly loss limits monitoring

### ⚙️ **Configuration Management**
- Parameter adjustment interface for all components
- Feature toggle controls (arbitrage/sniper/GEPA/Kestra)
- Risk tolerance and capital allocation settings

### 📊 **Analytics and Reporting**
- Historical performance charts
- Trade history with filtering
- Profit/loss breakdowns by strategy
- System health and uptime metrics

## 🛠️ Technology Stack

- **Framework**: Next.js 15 with TypeScript
- **Styling**: Tailwind CSS with custom design system
- **Charts**: Recharts for data visualization
- **Real-time**: WebSocket connection to bot API
- **Icons**: Lucide React
- **State Management**: React Context API

## 🚀 Quick Start

### Prerequisites
- Node.js 18+
- Enhanced Solana Arbitrage Bot running on port 8080

### Installation

1. **Install dependencies**
   ```bash
   npm install
   ```

2. **Configure environment**
   ```bash
   cp .env.local.example .env.local
   # Edit .env.local with your configuration
   ```

3. **Start development server**
   ```bash
   npm run dev
   ```

4. **Open dashboard**
   ```
   http://localhost:3000
   ```
