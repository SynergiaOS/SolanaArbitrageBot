# 🔌 API Documentation

## Overview

Solana Arbitrage Bot provides a comprehensive REST API and WebSocket interface for monitoring and controlling the trading system.

## Base URL

```
http://localhost:3001/api
```

## Authentication

All API endpoints require authentication via Bearer token:

```bash
curl -H "Authorization: Bearer YOUR_TOKEN" http://localhost:3001/api/status
```

## REST API Endpoints

### System Status

#### GET /api/status

Returns current system status and health information.

**Response:**
```json
{
  "status": "running",
  "uptime_seconds": 3600,
  "last_trade": "2024-01-15T10:30:00Z",
  "total_trades": 42,
  "daily_pnl": 125.50,
  "active_positions": 3,
  "system_health": "healthy"
}
```

#### GET /api/metrics

Returns detailed system metrics.

**Response:**
```json
{
  "trading": {
    "total_trades": 42,
    "successful_trades": 40,
    "failed_trades": 2,
    "success_rate": 95.24,
    "average_profit_usd": 2.98,
    "total_profit_usd": 125.50
  },
  "performance": {
    "average_latency_ms": 185,
    "rpc_calls_per_minute": 120,
    "cache_hit_rate": 85.5
  },
  "positions": {
    "active_arbitrage": 0,
    "active_sniper": 3,
    "total_value_sol": 15.25
  }
}
```

### Configuration Management

#### GET /api/config

Returns current configuration.

#### POST /api/config

Updates configuration settings.

**Request Body:**
```json
{
  "limits": {
    "max_position_sol": 10.0,
    "min_profit_usd": 0.5,
    "max_daily_loss_usd": 50.0
  },
  "execution": {
    "dry_run": false,
    "priority_fee_lamports": 10000
  },
  "sniper": {
    "enabled": true,
    "max_buy_amount_sol": 1.0
  }
}
```

### Trading Control

#### POST /api/control/start

Starts the trading system.

#### POST /api/control/stop

Stops the trading system gracefully.

#### POST /api/control/emergency

Emergency stop - immediately halts all trading.

**Request Body:**
```json
{
  "reason": "manual stop",
  "source": "operator"
}
```

### Sniper Bot

#### GET /api/sniper/positions

Returns active sniper positions.

**Response:**
```json
{
  "positions": [
    {
      "token_mint": "So11111111111111111111111111111111111111112",
      "symbol": "NEWTOKEN",
      "entry_price": 0.001,
      "current_price": 0.0012,
      "amount_sol": 1.0,
      "pnl_usd": 15.50,
      "entry_time": "2024-01-15T10:00:00Z"
    }
  ]
}
```

#### POST /api/sniper/close/{token_mint}

Closes a specific sniper position.

### Trade History

#### GET /api/trades

Returns trade history with pagination.

**Query Parameters:**
- `limit`: Number of trades to return (default: 50, max: 500)
- `offset`: Offset for pagination (default: 0)
- `from`: Start date (ISO 8601)
- `to`: End date (ISO 8601)
- `type`: Trade type filter (`arbitrage`, `sniper`)

**Response:**
```json
{
  "trades": [
    {
      "id": "uuid",
      "type": "arbitrage",
      "timestamp": "2024-01-15T10:30:00Z",
      "buy_dex": "raydium",
      "sell_dex": "orca",
      "token_mint": "So11111111111111111111111111111111111111112",
      "amount_sol": 5.0,
      "profit_usd": 12.50,
      "gas_cost_sol": 0.001,
      "signature": "transaction_signature"
    }
  ],
  "total": 42,
  "has_more": true
}
```

## WebSocket API

### Connection

```javascript
const ws = new WebSocket('ws://localhost:3001/ws');
```

### Authentication

Send authentication message immediately after connection:

```javascript
ws.send(JSON.stringify({
  type: 'auth',
  token: 'YOUR_TOKEN'
}));
```

### Message Types

#### Real-time Updates

**Trade Executed:**
```json
{
  "type": "trade_executed",
  "data": {
    "type": "arbitrage",
    "profit_usd": 12.50,
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

**Position Update:**
```json
{
  "type": "position_update",
  "data": {
    "token_mint": "So11111111111111111111111111111111111111112",
    "pnl_usd": 15.50,
    "current_price": 0.0012
  }
}
```

**System Alert:**
```json
{
  "type": "system_alert",
  "data": {
    "level": "warning",
    "message": "High slippage detected",
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

#### Subscriptions

Subscribe to specific data streams:

```javascript
// Subscribe to trade updates
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'trades'
}));

// Subscribe to position updates
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'positions'
}));

// Subscribe to system alerts
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'alerts'
}));
```

## Error Handling

All API endpoints return standard HTTP status codes:

- `200` - Success
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `429` - Rate Limited
- `500` - Internal Server Error

Error responses include details:

```json
{
  "error": {
    "code": "INVALID_CONFIG",
    "message": "Invalid configuration parameter",
    "details": "max_position_sol must be positive"
  }
}
```

## Rate Limiting

API endpoints are rate limited:
- General endpoints: 100 requests/minute
- Trading control: 10 requests/minute
- WebSocket connections: 5 connections per IP

## Examples

See the main README.md for complete usage examples.
