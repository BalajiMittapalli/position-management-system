# Position Management Backend API

## Setup

1. Make sure PostgreSQL is running
2. Create database: `createdb position_management`
3. Run migrations: `psql -d position_management -f migrations/001_initial.sql`
4. Set environment variables in `.env`:
   ```
   DATABASE_URL=postgresql://postgres:password@localhost/position_management
   RUST_LOG=info
   ```
5. Run the server: `cargo run`

Server will start on `http://localhost:3000`

## API Endpoints for Postman Testing

### 1. Open Position
**POST** `http://localhost:3000/api/positions/open`
```json
{
  "user_id": "user123",
  "symbol": "BTCUSDT",
  "side": "Long",
  "size": "1.5",
  "entry_price": "50000.00",
  "leverage": 10,
  "stop_loss": "45000.00",
  "take_profit": "60000.00"
}
```

### 2. Modify Position
**PUT** `http://localhost:3000/api/positions/{position_id}/modify`
```json
{
  "size": "2.0",
  "stop_loss": "47000.00",
  "take_profit": "65000.00"
}
```

### 3. Close Position
**DELETE** `http://localhost:3000/api/positions/{position_id}/close`

### 4. Get Position
**GET** `http://localhost:3000/api/positions/{position_id}`

### 5. Get User Positions
**GET** `http://localhost:3000/api/users/{user_id}/positions?limit=10&offset=0&status=open`

### 6. Get User Trading Stats
**GET** `http://localhost:3000/api/users/{user_id}/stats`

### 7. Get User Risk Metrics
**GET** `http://localhost:3000/api/users/{user_id}/risk`

### 8. Get PnL Snapshots
**GET** `http://localhost:3000/api/pnl/snapshots/{user_id}`

### 9. Get Funding Rates
**GET** `http://localhost:3000/api/funding-rates`

### 10. WebSocket Connection
**WebSocket** `ws://localhost:3000/api/ws`

## Response Format

All API responses follow this format:
```json
{
  "success": true,
  "data": {...},
  "error": null
}
```

Error responses:
```json
{
  "success": false,
  "data": null,
  "error": "Error message"
}
```

## Sample Test Flow

1. **Open a position** using POST /api/positions/open
2. **Get the position** using GET /api/positions/{position_id}
3. **Modify the position** using PUT /api/positions/{position_id}/modify
4. **Get user positions** using GET /api/users/{user_id}/positions
5. **Close the position** using DELETE /api/positions/{position_id}/close
6. **Check user stats** using GET /api/users/{user_id}/stats

## Database Schema

The API works with these main tables:
- `positions` - Current and historical positions
- `position_modifications` - Audit trail of position changes
- `pnl_snapshots` - PnL tracking over time
- `user_trading_stats` - User trading statistics
- `user_risk_metrics` - Risk metrics per user
- `funding_rates` - Funding rates for symbols

## Notes

- All decimal values (prices, sizes) should be sent as strings to maintain precision
- Position IDs are UUIDs generated automatically
- User IDs can be any string identifier
- The WebSocket endpoint sends periodic updates every 30 seconds (demo purposes)
- CORS is enabled for all origins during development
