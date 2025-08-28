# Solana Arbitrage Bot 🚀

High-performance arbitrage bot for Solana blockchain with advanced features including MEV protection, multi-DEX support, and AI-powered optimization.

## 🎯 Features

- **Multi-DEX Arbitrage**: Supports Raydium, Orca, Jupiter, and other major Solana DEXs
- **MEV Protection**: Jito bundle integration for private mempool transactions
- **AI Optimization**: Genetic algorithm (GEPA) for strategy parameter optimization
- **New Token Sniping**: Automatic detection and trading of newly launched tokens
- **Risk Management**: Comprehensive safety checks and position limits
- **Real-time Dashboard**: WebSocket-based monitoring interface

## 📊 Performance Targets

- **Profit**: >50 USD/day
- **Latency**: <300ms execution time
- **Uptime**: >99.5%
- **Monitoring**: 1000+ opportunities/second

## 🏗️ Architecture

The bot uses a modular, feature-flag based architecture:

```
src/
├── main.rs           # Entry point with CLI
├── config_manager.rs # Configuration management
├── monitor.rs        # DEX price monitoring
├── calculator.rs     # Profit calculations
├── executor.rs       # Transaction execution
├── safety.rs         # Risk management
├── sniper/          # Token sniping module
├── dex/             # DEX integrations
├── web/             # Dashboard API
└── performance/     # Optimization modules
```

See [.bmad/ARCHITECTURE.md](.bmad/ARCHITECTURE.md) for detailed architecture documentation.

## 📋 Prerequisites

- Rust 1.70+ 
- Solana CLI tools
- Node.js 16+ (for dashboard)
- PostgreSQL (optional, for production)

## 🚀 Quick Start

### 1. Clone and Setup
```bash
git clone https://github.com/SynergiaOS/SolanaArbitrageBot.git
cd SolanaArbitrageBot
```

### 2. Configure
```bash
cp config.yaml config.local.yaml
# Edit config.local.yaml with your settings
```

### 3. Build

**Monitor-only mode (Production):**
```bash
cargo build --release --no-default-features --features monitor
```

**Full features (Development):**
```bash
cargo build --release --all-features
```

### 4. Run

**Monitor mode:**
```bash
./scripts/start_monitor_only.sh
```

**Full bot:**
```bash
cargo run --release
```

## 📊 Configuration

Key configuration sections in `config.yaml`:

```yaml
limits:
  max_position_sol: 10.0
  max_daily_loss_usd: 100.0
  
sniper:
  enabled: true
  min_liquidity_sol: 5.0
  
dex:
  raydium:
    enabled: true
    rpc_url: "https://api.mainnet-beta.solana.com"
```

## 🛡️ Security

- **Private Key Management**: Supports hardware wallets and secure vaults
- **Position Limits**: Configurable max position size
- **Daily Loss Limits**: Automatic circuit breaker
- **Slippage Protection**: Dynamic slippage calculation

See [.bmad/DEPLOYMENT.md](.bmad/DEPLOYMENT.md) for production security setup.

## 🧪 Testing

```bash
# Unit tests
cargo test

# Integration tests  
cargo test --workspace

# Safety tests
./scripts/run_safety_tests.sh
```

## 📈 Performance

- **Latency**: <300ms transaction execution
- **Throughput**: 1000+ opportunities/second monitoring
- **Uptime**: 99.5%+ with automatic recovery

## 🔧 Development

### Feature Flags
- `monitor` - Core monitoring functionality
- `sniper` - Token sniping features  
- `web` - Dashboard and API
- `gepa` - Genetic algorithm optimization
- `full` - All features enabled

### Building Specific Features
```bash
cargo build --release --features "monitor,sniper"
```

## 📚 Documentation

- [Architecture](.bmad/ARCHITECTURE.md) - System design and components
- [Deployment](.bmad/DEPLOYMENT.md) - Production deployment guide
- [Development Stories](.bmad/DEVELOPMENT_STORIES.md) - Feature implementation roadmap
- [Project Requirements](.bmad/PROJECT_REQUIREMENTS.md) - Detailed specifications

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⚠️ Disclaimer

This software is for educational purposes only. Cryptocurrency trading involves substantial risk of loss. Use at your own risk.

## 🙏 Acknowledgments

- Solana Foundation
- Jito Labs for MEV infrastructure
- DEX protocols (Raydium, Orca, Jupiter)
- Open source contributors

---

For support, please open an issue or contact the team.
