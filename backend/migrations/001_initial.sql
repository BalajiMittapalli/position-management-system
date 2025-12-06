-- Position Management Database Schema

-- Create custom types
CREATE TYPE side_type AS ENUM ('Long', 'Short');
CREATE TYPE position_status AS ENUM ('open', 'closed', 'liquidated');

-- Positions table (current and historical)
CREATE TABLE positions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    side side_type NOT NULL,
    size DECIMAL(20, 8) NOT NULL,
    entry_price DECIMAL(20, 8) NOT NULL,
    leverage SMALLINT NOT NULL,
    margin DECIMAL(20, 8) NOT NULL,
    status position_status NOT NULL DEFAULT 'open',
    stop_loss DECIMAL(20, 8),
    take_profit DECIMAL(20, 8),
    realized_pnl DECIMAL(20, 8) DEFAULT 0,
    unrealized_pnl DECIMAL(20, 8) DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Position modifications audit trail
CREATE TABLE position_modifications (
    id SERIAL PRIMARY KEY,
    position_id TEXT NOT NULL REFERENCES positions(id),
    user_id TEXT NOT NULL,
    modification_type TEXT NOT NULL,
    old_value DECIMAL(20, 8),
    new_value DECIMAL(20, 8),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- PnL snapshots (hourly/daily)
CREATE TABLE pnl_snapshots (
    id SERIAL PRIMARY KEY,
    position_id TEXT NOT NULL REFERENCES positions(id),
    user_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    unrealized_pnl DECIMAL(20, 8) NOT NULL,
    realized_pnl DECIMAL(20, 8) NOT NULL,
    price_at_snapshot DECIMAL(20, 8) NOT NULL,
    margin_ratio DECIMAL(10, 4) NOT NULL,
    snapshot_type TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- User trading statistics
CREATE TABLE user_trading_stats (
    id SERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    total_positions_opened INTEGER DEFAULT 0,
    total_positions_closed INTEGER DEFAULT 0,
    total_positions_liquidated INTEGER DEFAULT 0,
    total_volume DECIMAL(20, 8) DEFAULT 0,
    total_realized_pnl DECIMAL(20, 8) DEFAULT 0,
    win_rate DECIMAL(5, 4) DEFAULT 0,
    avg_holding_time_hours DECIMAL(10, 2) DEFAULT 0,
    largest_win DECIMAL(20, 8) DEFAULT 0,
    largest_loss DECIMAL(20, 8) DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Risk metrics per user
CREATE TABLE user_risk_metrics (
    id SERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    current_exposure DECIMAL(20, 8) DEFAULT 0,
    max_leverage_used SMALLINT DEFAULT 1,
    avg_leverage_used DECIMAL(5, 2) DEFAULT 1,
    liquidation_risk_score DECIMAL(3, 2) DEFAULT 0,
    var_95 DECIMAL(20, 8) DEFAULT 0,
    max_drawdown DECIMAL(20, 8) DEFAULT 0,
    sharpe_ratio DECIMAL(10, 4) DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Funding rates table
CREATE TABLE funding_rates (
    symbol TEXT PRIMARY KEY,
    rate DECIMAL(10, 6) NOT NULL,
    next_funding_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for better performance
CREATE INDEX idx_positions_user_id ON positions(user_id);
CREATE INDEX idx_positions_symbol ON positions(symbol);
CREATE INDEX idx_positions_status ON positions(status);
CREATE INDEX idx_positions_created_at ON positions(created_at);

CREATE INDEX idx_position_modifications_position_id ON position_modifications(position_id);
CREATE INDEX idx_position_modifications_user_id ON position_modifications(user_id);
CREATE INDEX idx_position_modifications_timestamp ON position_modifications(timestamp);

CREATE INDEX idx_pnl_snapshots_position_id ON pnl_snapshots(position_id);
CREATE INDEX idx_pnl_snapshots_user_id ON pnl_snapshots(user_id);
CREATE INDEX idx_pnl_snapshots_timestamp ON pnl_snapshots(timestamp);
CREATE INDEX idx_pnl_snapshots_type ON pnl_snapshots(snapshot_type);

CREATE INDEX idx_user_trading_stats_user_id ON user_trading_stats(user_id);
CREATE INDEX idx_user_risk_metrics_user_id ON user_risk_metrics(user_id);
