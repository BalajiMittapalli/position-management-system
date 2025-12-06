# Position Management System

A comprehensive decentralized perpetual trading platform built on Solana blockchain, featuring real-time position management, margin calculations, and risk monitoring with a professional trading interface.

## 📋 Table of Contents

- [Overview](#overview)
- [System Architecture](#system-architecture)
- [Components](#components)
- [Features](#features)
- [Technology Stack](#technology-stack)
- [Installation](#installation)
- [Usage](#usage)
- [API Documentation](#api-documentation)

<a name="overview"></a>
## 🎯 Overview

The Position Management System is a full-stack decentralized application (dApp) that enables traders to manage leveraged positions on the Solana blockchain. The system provides:

- **On-chain Position Management**: Smart contracts for opening, modifying, and closing leveraged positions
- **Real-time Risk Monitoring**: Continuous tracking of margin levels and liquidation risks
- **Professional Trading UI**: Streamlit-based dashboard with real-time updates via WebSocket
- **Comprehensive Backend**: Rust-based API server with PostgreSQL for historical data and analytics

<a name="system-architecture"></a>
## 🏗️ System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Interface (UI)                       │
│                    Streamlit Dashboard (Python)                  │
│  ┌──────────────┬──────────────┬──────────────┬──────────────┐ │
│  │ Open Position│Active Positions│Risk Monitor │  PnL History │ │
│  └──────────────┴──────────────┴──────────────┴──────────────┘ │
└────────────────────────┬────────────────────────────────────────┘
                         │ HTTP/REST + WebSocket
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Backend API Server (Rust)                     │
│                         Axum Framework                           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Routes  │  Position Monitor  │  Margin Calculator       │  │
│  │  PnL Tracker  │  WebSocket Handler  │  Position Manager │  │
│  └──────────────────────────────────────────────────────────┘  │
└───────┬────────────────────────────┬────────────────────────────┘
        │                            │
        ▼                            ▼
┌──────────────────┐      ┌──────────────────────────────────────┐
│   PostgreSQL     │      │      Solana Blockchain               │
│    Database      │      │                                      │
│                  │      │  ┌────────────────────────────────┐ │
│ • Positions      │      │  │   Position Management Program  │ │
│ • Users          │      │  │      (Anchor Framework)        │ │
│ • PnL History    │      │  │                                │ │
│ • Liquidations   │      │  │  • Open Position              │ │
│ • Funding Rates  │      │  │  • Modify Position            │ │
└──────────────────┘      │  │  • Close Position             │ │
                          │  └────────────────────────────────┘ │
                          │                                      │
                          │  Program ID: 3BpQ5UZ...yraf          │
                          └──────────────────────────────────────┘
```

### Data Flow

1. **User Action** → UI sends request via HTTP/WebSocket
2. **Backend Processing** → Validates, calculates margin/risk, updates DB
3. **Blockchain Transaction** → Backend calls Solana program for on-chain state
4. **Real-time Updates** → WebSocket pushes updates to all connected clients
5. **Historical Storage** → PostgreSQL stores all transactions for analytics

<a name="components"></a>
## 🧩 Components

### 1. Solana Smart Contract (Anchor Program)

**Location**: `programs/position-management/`

The on-chain program handles all position state management on Solana blockchain.

#### Key Files:

- **`lib.rs`**: Main program entry point with instruction handlers
- **`state.rs`**: Account structures for positions and user data
- **`calculations.rs`**: On-chain PnL and margin calculations
- **`error.rs`**: Custom error definitions
- **`instructions/`**: Individual instruction handlers
  - `open_position.rs`: Create new leveraged positions
  - `modify_position.rs`: Adjust position size or margin
  - `close_position.rs`: Close positions and realize PnL

#### State Structures:

```rust
// User Account: Tracks user's overall portfolio
pub struct UserAccount {
    pub authority: Pubkey,           // User's wallet address
    pub total_positions: u32,        // Number of open positions
    pub total_collateral: u64,       // Total collateral deposited
    pub unrealized_pnl: i64,         // Current unrealized profit/loss
    pub realized_pnl: i64,           // Lifetime realized PnL
    pub last_funding_update: i64,    // Last funding payment timestamp
}

// Position: Individual trading position
pub struct Position {
    pub user: Pubkey,                // Owner's wallet
    pub symbol: String,              // Trading pair (e.g., "SOL/USDC")
    pub side: Side,                  // Long or Short
    pub size: u64,                   // Position size in base units
    pub entry_price: u64,            // Average entry price
    pub leverage: u8,                // Leverage multiplier (1-100x)
    pub margin: u64,                 // Collateral for this position
    pub liquidation_price: u64,      // Price at which position liquidates
    pub unrealized_pnl: i64,         // Current profit/loss
    pub funding_paid: i64,           // Accumulated funding payments
    pub opened_at: i64,              // Position open timestamp
}
```

#### Instructions:

1. **Open Position**
   - Validates leverage and margin requirements
   - Calculates liquidation price
   - Creates position account on-chain
   - Transfers collateral from user

2. **Modify Position**
   - Supports: Add margin, Remove margin, Increase size, Decrease size
   - Recalculates liquidation price
   - Updates position state atomically

3. **Close Position**
   - Calculates final PnL
   - Transfers funds back to user
   - Closes position account
   - Updates user's realized PnL

### 2. Backend API Server (Rust)

**Location**: `backend/`

High-performance API server built with Axum framework, providing REST endpoints and WebSocket connections.

#### Modules:

**`main.rs`**: Application bootstrap and server initialization
- Initializes database connection pool
- Starts background monitoring tasks
- Configures CORS and middleware
- Sets up WebSocket and HTTP routes

**`routes.rs`**: HTTP endpoint handlers
- `GET /health` - Health check
- `GET /solana/status` - Blockchain connection status
- `POST /positions/open` - Open new position
- `PUT /positions/modify` - Modify existing position
- `POST /positions/close` - Close position
- `GET /positions/user/:address` - Get user's positions
- `GET /positions/:id` - Get specific position details
- `GET /pnl/user/:address` - Get user's PnL history
- `GET /funding-rates` - Get current funding rates
- `WebSocket /ws` - Real-time position updates

**`position_manager.rs`**: Core position logic
- Position creation and validation
- Margin requirement calculations
- Position size and leverage limits
- Database operations for positions

**`margin_calculator.rs`**: Margin and liquidation calculations
```rust
pub struct MarginCalculator;

impl MarginCalculator {
    // Calculate required initial margin for a position
    pub fn calculate_initial_margin(size: Decimal, price: Decimal, leverage: u8) -> Decimal
    
    // Calculate maintenance margin (minimum to avoid liquidation)
    pub fn calculate_maintenance_margin(size: Decimal, price: Decimal, leverage: u8) -> Decimal
    
    // Calculate liquidation price for a position
    pub fn calculate_liquidation_price(
        entry_price: Decimal,
        leverage: u8,
        side: Side,
    ) -> Decimal
    
    // Calculate current margin ratio
    pub fn calculate_margin_ratio(
        margin: Decimal,
        unrealized_pnl: Decimal,
        position_value: Decimal,
    ) -> Decimal
}
```

**`position_monitor.rs`**: Real-time position monitoring
- Continuously checks all open positions
- Monitors prices from oracle feeds
- Triggers liquidation alerts
- Updates margin ratios in real-time
- Sends WebSocket notifications

**`pnl_tracker.rs`**: Profit & Loss tracking
- Calculates unrealized PnL for open positions
- Records realized PnL on position close
- Maintains historical PnL records
- Generates PnL analytics and reports

**`db.rs`**: Database connection and pool management
- PostgreSQL connection pooling
- Database schema migrations
- Connection retry logic

**`models.rs`**: Data structures and serialization
```rust
pub struct Position {
    pub id: Uuid,
    pub user_address: String,
    pub symbol: String,
    pub side: Side,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub leverage: i16,
    pub margin: Decimal,
    pub liquidation_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub status: PositionStatus,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

pub struct PnlRecord {
    pub id: Uuid,
    pub position_id: Uuid,
    pub user_address: String,
    pub realized_pnl: Decimal,
    pub fees: Decimal,
    pub timestamp: DateTime<Utc>,
}
```

**`utils.rs`**: Shared utilities
- Price formatting and conversion
- Time utilities
- Validation helpers
- Error handling utilities

#### Database Schema:

```sql
-- Users table
CREATE TABLE users (
    address VARCHAR(44) PRIMARY KEY,
    total_collateral DECIMAL(20, 8),
    unrealized_pnl DECIMAL(20, 8),
    realized_pnl DECIMAL(20, 8),
    created_at TIMESTAMP DEFAULT NOW()
);

-- Positions table
CREATE TABLE positions (
    id UUID PRIMARY KEY,
    user_address VARCHAR(44) REFERENCES users(address),
    symbol VARCHAR(20) NOT NULL,
    side VARCHAR(5) NOT NULL,
    size DECIMAL(20, 8) NOT NULL,
    entry_price DECIMAL(20, 8) NOT NULL,
    leverage SMALLINT NOT NULL,
    margin DECIMAL(20, 8) NOT NULL,
    liquidation_price DECIMAL(20, 8) NOT NULL,
    unrealized_pnl DECIMAL(20, 8) DEFAULT 0,
    status VARCHAR(20) DEFAULT 'open',
    opened_at TIMESTAMP DEFAULT NOW(),
    closed_at TIMESTAMP
);

-- PnL history table
CREATE TABLE pnl_history (
    id UUID PRIMARY KEY,
    position_id UUID REFERENCES positions(id),
    user_address VARCHAR(44) REFERENCES users(address),
    realized_pnl DECIMAL(20, 8),
    fees DECIMAL(20, 8),
    timestamp TIMESTAMP DEFAULT NOW()
);

-- Liquidations table
CREATE TABLE liquidations (
    id UUID PRIMARY KEY,
    position_id UUID REFERENCES positions(id),
    user_address VARCHAR(44),
    liquidation_price DECIMAL(20, 8),
    loss_amount DECIMAL(20, 8),
    timestamp TIMESTAMP DEFAULT NOW()
);

-- Funding rates table
CREATE TABLE funding_rates (
    id UUID PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL,
    rate DECIMAL(10, 8) NOT NULL,
    timestamp TIMESTAMP DEFAULT NOW()
);
```

### 3. User Interface (Streamlit Dashboard)

**Location**: `ui/`

Professional trading dashboard built with Streamlit, providing real-time market data and position management.

#### Main Application (`app.py`):

The main dashboard page showing:
- **System Status**: Backend API and Solana blockchain connectivity
- **Market Overview**: Current prices and funding rates
- **Portfolio Summary**: Total positions, margin, PnL
- **Quick Actions**: Fast access to open/close positions

Features:
- Real-time WebSocket connection for live updates
- Dark theme optimized for trading
- Responsive layout with metric cards
- Auto-refresh every 5 seconds

#### Pages:

**1. Open Position (`pages/1_Open_Position.py`)**
- Symbol selection (SOL/USDC, BTC/USDC, ETH/USDC, etc.)
- Side selection (Long/Short)
- Position size input
- Leverage slider (1x to 100x)
- Entry price specification
- Margin calculator showing:
  - Required margin
  - Liquidation price
  - Max position size
  - Estimated fees
- One-click position opening

**2. Active Positions (`pages/2_Active_Positions.py`)**
- Real-time table of all open positions
- Columns: Symbol, Side, Size, Entry Price, Current Price, PnL, Margin Ratio, Leverage
- Color-coded PnL (green for profit, red for loss)
- Quick action buttons:
  - Modify position (add/remove margin, increase/decrease size)
  - Close position
  - View details
- Auto-refresh with WebSocket updates
- Position count and total exposure metrics

**3. Risk Monitor (`pages/3_Risk_Monitor.py`)**
- Real-time risk metrics dashboard
- Liquidation alerts with urgency levels:
  - 🔴 Critical (margin ratio < 50%)
  - 🟡 Warning (margin ratio < 75%)
  - 🟢 Healthy (margin ratio > 75%)
- Position-wise risk breakdown:
  - Current margin ratio
  - Distance to liquidation
  - Required margin to avoid liquidation
- Portfolio-level metrics:
  - Total risk exposure
  - Margin utilization
  - Diversification score
- Interactive charts and gauges

**4. PnL History (`pages/4_PnL_History.py`)**
- Historical profit/loss analysis
- Time-series charts:
  - Daily PnL
  - Cumulative PnL
  - Win rate over time
- Statistics table:
  - Total trades
  - Win rate
  - Average win/loss
  - Best/worst trades
  - Sharpe ratio
- Filtering by date range and symbol
- Export to CSV functionality

#### Utilities:

**`utils/api.py`**: Backend API client
```python
# Health check
def health_check() -> dict

# Get Solana status
def solana_status() -> dict

# Position operations
def open_position(user_address, symbol, side, size, leverage, entry_price) -> dict
def modify_position(position_id, modification_type, amount) -> dict
def close_position(position_id, exit_price) -> dict

# Data retrieval
def get_user_positions(user_address) -> list
def get_position_details(position_id) -> dict
def get_user_stats(user_address) -> dict
def get_pnl_history(user_address, days) -> list
def get_funding_rates() -> list
```

**`utils/websocket_client.py`**: Real-time WebSocket handler
```python
class WebSocketClient:
    def __init__(self, url: str):
        # Initialize WebSocket connection
        
    async def connect(self):
        # Establish connection with auto-reconnect
        
    async def subscribe_positions(self, user_address: str):
        # Subscribe to position updates
        
    def on_message(self, callback):
        # Register callback for incoming messages
        
    async def disconnect(self):
        # Clean connection close
```

<a name="features"></a>
## ✨ Features

### Core Trading Features

1. **Leveraged Position Management**
   - Open long/short positions with up to 100x leverage
   - Multi-asset support (SOL, BTC, ETH, and more)
   - Flexible position sizing
   - Real-time margin calculations

2. **Position Modification**
   - Add/remove margin to adjust risk
   - Increase/decrease position size
   - Dynamic leverage adjustment
   - Take-profit and stop-loss orders

3. **Risk Management**
   - Automated liquidation price calculation
   - Real-time margin ratio monitoring
   - Multi-level risk alerts
   - Portfolio-wide risk metrics

4. **PnL Tracking**
   - Real-time unrealized PnL calculation
   - Historical realized PnL records
   - Position-level and account-level tracking
   - Funding rate integration

### Technical Features

1. **Blockchain Integration**
   - Solana smart contract for trustless position management
   - On-chain state verification
   - Atomic transaction execution
   - Low transaction fees

2. **Real-time Updates**
   - WebSocket connections for instant notifications
   - Price feed integration
   - Position monitoring every second
   - Live risk metric updates

3. **Performance**
   - High-throughput Rust backend (10,000+ req/s)
   - Connection pooling for database
   - Efficient state management on Solana
   - Optimized SQL queries

4. **Data Persistence**
   - PostgreSQL for historical data
   - Complete audit trail
   - Backup and recovery
   - Analytics-ready schema

<a name="technology-stack"></a>
## 🛠️ Technology Stack

### Blockchain Layer
- **Solana**: High-performance blockchain (65,000 TPS)
- **Anchor Framework 0.32.1**: Rust-based Solana development framework
- **Solana SDK 2.0**: Blockchain interaction libraries

### Backend Layer
- **Rust 2021**: Systems programming language
- **Axum 0.7**: Modern async web framework
- **SQLx 0.8**: Async SQL toolkit with compile-time query checking
- **PostgreSQL**: Robust relational database
- **Tokio**: Async runtime for Rust
- **Tower**: Middleware framework

### Frontend Layer
- **Python 3.9+**: Programming language
- **Streamlit 1.28+**: Web app framework
- **Pandas**: Data manipulation
- **Plotly**: Interactive charts
- **WebSocket**: Real-time communication

### Development Tools
- **Yarn**: Package manager
- **TypeScript**: Testing framework
- **Mocha**: Test runner
- **Docker**: Containerization (optional)

<a name="installation"></a>
## 📦 Installation

### Prerequisites

```bash
# Required
- Rust 1.75+ (rustc --version)
- Solana CLI 1.18+ (solana --version)
- Anchor CLI 0.32+ (anchor --version)
- Node.js 18+ (node --version)
- Yarn 1.22+ (yarn --version)
- Python 3.9+ (python3 --version)
- PostgreSQL 14+ (psql --version)

# Optional
- Docker 24+ (for containerized deployment)
```

### Step 1: Clone the Repository

```bash
git clone https://github.com/BalajiMittapalli/position-management-system.git
cd position-management-system
```

### Step 2: Install Dependencies

#### Install Node.js dependencies
```bash
yarn install
```

#### Install Rust dependencies (automatically handled by Cargo)
```bash
cd backend
cargo build --release
cd ..
```

#### Install Python dependencies
```bash
cd ui
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt
cd ..
```

### Step 3: Set Up Solana

#### Configure Solana CLI for localnet
```bash
solana config set --url localhost
solana-keygen new  # Create a new wallet if needed
```

#### Start local Solana validator
```bash
solana-test-validator
```

Keep this terminal running.

### Step 4: Build and Deploy Smart Contract

```bash
# Build the Anchor program
anchor build

# Deploy to localnet
anchor deploy

# Note: Your program ID will be displayed. Update Anchor.toml if needed.
```

### Step 5: Set Up Database

```bash
# Create PostgreSQL database
createdb position_management

# Run migrations
cd backend
sqlx database create
sqlx migrate run
cd ..
```

### Step 6: Configure Environment Variables

#### Backend configuration
```bash
cd backend
cp .env.example .env
```

Edit `backend/.env`:
```env
DATABASE_URL=postgresql://postgres:password@localhost/position_management
PROGRAM_ID=3BpQ5UZ3B3jK4SioSf31d2fLaSV3jDeu77ZE3EeVyraf
WALLET_PATH=/home/your-user/.config/solana/id.json
RPC_URL=http://localhost:8899
PORT=8080
RUST_LOG=info
```

#### UI configuration
Edit `ui/utils/api.py` to set:
```python
API_BASE_URL = "http://localhost:8080"
WS_URL = "ws://localhost:8080/ws"
```

### Step 7: Run the Application

#### Terminal 1: Solana Validator (already running)
```bash
solana-test-validator
```

#### Terminal 2: Backend API Server
```bash
cd backend
cargo run --release
```

The API will start on `http://localhost:8080`

#### Terminal 3: Streamlit UI
```bash
cd ui
source venv/bin/activate  # On Windows: venv\Scripts\activate
streamlit run app.py
```

The UI will open automatically at `http://localhost:8501`

<a name="usage"></a>
## 🚀 Usage

### Opening a Position

1. Navigate to **Open Position** page
2. Select trading pair (e.g., SOL/USDC)
3. Choose side (Long or Short)
4. Enter position size
5. Set leverage (1x to 100x)
6. Specify entry price
7. Review margin requirements
8. Click "Open Position"

### Monitoring Positions

1. Go to **Active Positions** page
2. View all open positions in real-time
3. Check current PnL for each position
4. Monitor margin ratios
5. Use filters to sort by symbol, PnL, etc.

### Managing Risk

1. Access **Risk Monitor** page
2. Review overall portfolio risk
3. Check positions near liquidation
4. View margin utilization
5. Take action on high-risk positions

### Closing a Position

1. From **Active Positions** page
2. Click "Close" button on desired position
3. Confirm exit price
4. Review final PnL
5. Complete transaction

<a name="api-documentation"></a>
## 📚 API Documentation

### REST Endpoints

#### Health Check
```http
GET /health
Response: { "status": "healthy" }
```

#### Solana Status
```http
GET /solana/status
Response: {
  "connected": true,
  "program_id": "3BpQ...",
  "rpc_url": "http://localhost:8899"
}
```

#### Open Position
```http
POST /positions/open
Content-Type: application/json

{
  "user_address": "7xKXt...abc",
  "symbol": "SOL/USDC",
  "side": "long",
  "size": 10.5,
  "leverage": 5,
  "entry_price": 100.25
}

Response: {
  "success": true,
  "position_id": "550e8400-e29b-41d4-a716-446655440000",
  "signature": "3BpQ5..."
}
```

#### Get User Positions
```http
GET /positions/user/{address}
Response: [
  {
    "id": "550e8400-...",
    "symbol": "SOL/USDC",
    "side": "long",
    "size": 10.5,
    "entry_price": 100.25,
    "current_price": 105.50,
    "unrealized_pnl": 55.125,
    "margin_ratio": 0.85,
    "liquidation_price": 80.20,
    "leverage": 5,
    "opened_at": "2025-12-06T10:30:00Z"
  }
]
```

#### Close Position
```http
POST /positions/close
Content-Type: application/json

{
  "position_id": "550e8400-...",
  "exit_price": 105.50
}

Response: {
  "success": true,
  "realized_pnl": 55.125,
  "signature": "5yt7..."
}
```

#### Get PnL History
```http
GET /pnl/user/{address}?days=30
Response: [
  {
    "date": "2025-12-06",
    "realized_pnl": 55.125,
    "unrealized_pnl": 120.50,
    "total_pnl": 175.625,
    "trades": 5
  }
]
```

### WebSocket Protocol

#### Connect
```javascript
ws://localhost:8080/ws
```

#### Subscribe to Position Updates
```json
{
  "type": "subscribe",
  "user_address": "7xKXt...abc"
}
```

#### Position Update Message
```json
{
  "type": "position_update",
  "position_id": "550e8400-...",
  "current_price": 105.50,
  "unrealized_pnl": 55.125,
  "margin_ratio": 0.85,
  "timestamp": "2025-12-06T10:30:00Z"
}
```

#### Liquidation Alert
```json
{
  "type": "liquidation_alert",
  "position_id": "550e8400-...",
  "symbol": "SOL/USDC",
  "current_price": 82.00,
  "liquidation_price": 80.20,
  "margin_ratio": 0.45,
  "urgency": "critical"
}
```

---

**Built with ❤️ on Solana**
