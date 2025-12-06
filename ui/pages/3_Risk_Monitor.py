"""
Risk Monitor Page - Real-time risk monitoring with WebSocket
"""

import streamlit as st
import pandas as pd
import time
from datetime import datetime
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from utils.api import get_user_positions, position_health_check, get_funding_rates
from utils.websocket_client import (
    connect_websocket, disconnect_websocket, 
    get_latest_prices, get_latest_update, is_connected
)

st.set_page_config(
    page_title="Risk Monitor | Solana Perps",
    page_icon="⚠️",
    layout="wide"
)

# Custom CSS for risk monitoring
st.markdown("""
<style>
    .risk-low {
        background: rgba(0, 212, 170, 0.2);
        border: 2px solid #00d4aa;
        border-radius: 12px;
        padding: 20px;
        text-align: center;
    }
    
    .risk-medium {
        background: rgba(255, 193, 7, 0.2);
        border: 2px solid #ffc107;
        border-radius: 12px;
        padding: 20px;
        text-align: center;
    }
    
    .risk-high {
        background: rgba(248, 81, 73, 0.2);
        border: 2px solid #f85149;
        border-radius: 12px;
        padding: 20px;
        text-align: center;
    }
    
    .price-card {
        background: linear-gradient(135deg, #1a1f2e 0%, #252b3b 100%);
        border-radius: 12px;
        padding: 15px;
        margin: 10px 0;
        border: 1px solid #2d3748;
    }
    
    .liquidation-warning {
        background: rgba(248, 81, 73, 0.3);
        border: 2px solid #f85149;
        border-radius: 8px;
        padding: 15px;
        margin: 10px 0;
        animation: pulse 2s infinite;
    }
    
    @keyframes pulse {
        0% { opacity: 1; }
        50% { opacity: 0.7; }
        100% { opacity: 1; }
    }
    
    .ws-connected {
        color: #00d4aa;
        font-weight: bold;
    }
    
    .ws-disconnected {
        color: #f85149;
        font-weight: bold;
    }
</style>
""", unsafe_allow_html=True)


def init_session_state():
    """Initialize session state"""
    if 'user_id' not in st.session_state:
        st.session_state.user_id = ""
    if 'positions' not in st.session_state:
        st.session_state.positions = []
    if 'ws_connected' not in st.session_state:
        st.session_state.ws_connected = False
    if 'last_prices' not in st.session_state:
        st.session_state.last_prices = {}
    if 'risk_alerts' not in st.session_state:
        st.session_state.risk_alerts = []


def calculate_liquidation_price(position: dict) -> float:
    """Calculate estimated liquidation price for a position"""
    entry_price = float(position.get('entry_price', 0))
    leverage = int(position.get('leverage', 1))
    side = position.get('side', 'Long')
    
    # Simplified liquidation calculation (maintenance margin ~0.5%)
    maintenance_margin_rate = 0.005
    
    if side == 'Long':
        # Long liquidation = entry * (1 - 1/leverage + maintenance)
        liq_price = entry_price * (1 - (1 / leverage) + maintenance_margin_rate)
    else:
        # Short liquidation = entry * (1 + 1/leverage - maintenance)
        liq_price = entry_price * (1 + (1 / leverage) - maintenance_margin_rate)
    
    return liq_price


def calculate_margin_ratio(position: dict, current_price: float) -> float:
    """Calculate current margin ratio"""
    entry_price = float(position.get('entry_price', 0))
    margin = float(position.get('margin', 0) or 0)
    size = float(position.get('size', 0))
    side = position.get('side', 'Long')
    
    if margin == 0 or size == 0:
        return 100.0
    
    # Calculate unrealized PnL
    if side == 'Long':
        pnl = (current_price - entry_price) * size
    else:
        pnl = (entry_price - current_price) * size
    
    # Current equity = margin + pnl
    equity = margin + pnl
    
    # Position value
    position_value = current_price * size
    
    # Margin ratio = equity / position_value * 100
    if position_value > 0:
        return (equity / position_value) * 100
    return 100.0


def get_risk_level(margin_ratio: float) -> tuple:
    """Get risk level based on margin ratio"""
    if margin_ratio > 10:
        return "LOW", "🟢", "risk-low"
    elif margin_ratio > 5:
        return "MEDIUM", "🟡", "risk-medium"
    else:
        return "HIGH", "🔴", "risk-high"


def display_price_ticker(prices: dict):
    """Display live price ticker"""
    if not prices:
        st.info("Waiting for price data...")
        return
    
    cols = st.columns(len(prices))
    
    for idx, (symbol, price) in enumerate(prices.items()):
        with cols[idx]:
            st.metric(
                label=symbol,
                value=f"${price:,.2f}",
                delta=f"{(price * 0.001):+.2f}",  # Mock delta
                delta_color="normal"
            )


def display_position_risk(position: dict, current_prices: dict):
    """Display risk metrics for a position"""
    symbol = position.get('symbol', '')
    side = position.get('side', 'Long')
    
    # Get current price
    current_price = current_prices.get(symbol, float(position.get('entry_price', 0)))
    
    # Calculate metrics
    liq_price = calculate_liquidation_price(position)
    margin_ratio = calculate_margin_ratio(position, current_price)
    entry_price = float(position.get('entry_price', 0))
    
    # Distance to liquidation
    if side == 'Long':
        distance_to_liq = ((current_price - liq_price) / current_price) * 100
    else:
        distance_to_liq = ((liq_price - current_price) / current_price) * 100
    
    risk_level, risk_emoji, risk_class = get_risk_level(margin_ratio)
    
    # Display card
    side_emoji = "🟢" if side == 'Long' else "🔴"
    
    with st.container():
        st.markdown(f"### {side_emoji} {symbol} - {side} {position.get('leverage')}x")
        
        col1, col2, col3, col4 = st.columns(4)
        
        with col1:
            st.metric("Entry Price", f"${entry_price:,.2f}")
            st.metric("Current Price", f"${current_price:,.2f}")
        
        with col2:
            st.metric("Liquidation Price", f"${liq_price:,.2f}")
            st.metric(
                "Distance to Liq",
                f"{distance_to_liq:.2f}%",
                delta_color="normal" if distance_to_liq > 10 else "inverse"
            )
        
        with col3:
            st.metric("Margin Ratio", f"{margin_ratio:.2f}%")
            st.metric("Leverage", f"{position.get('leverage')}x")
        
        with col4:
            st.markdown(f"""
            <div class="{risk_class}">
                <h2>{risk_emoji}</h2>
                <h3>{risk_level} RISK</h3>
            </div>
            """, unsafe_allow_html=True)
        
        # Liquidation warning
        if distance_to_liq < 5:
            st.markdown("""
            <div class="liquidation-warning">
                ⚠️ <strong>LIQUIDATION WARNING!</strong> Position is close to liquidation price!
            </div>
            """, unsafe_allow_html=True)
        
        st.markdown("---")


def main():
    init_session_state()
    
    st.markdown("# ⚠️ Real-Time Risk Monitor")
    st.markdown("Monitor your positions and liquidation risks in real-time")
    st.markdown("---")
    
    # Sidebar
    with st.sidebar:
        st.markdown("### 👛 Wallet")
        
        user_id = st.text_input(
            "Wallet Address / User ID",
            value=st.session_state.user_id,
            placeholder="Enter wallet address..."
        )
        
        if user_id:
            st.session_state.user_id = user_id
        
        st.markdown("---")
        
        # WebSocket connection
        st.markdown("### 🔌 WebSocket")
        
        ws_status = is_connected()
        if ws_status:
            st.markdown('🟢 <span class="ws-connected">Connected</span>', unsafe_allow_html=True)
        else:
            st.markdown('🔴 <span class="ws-disconnected">Disconnected</span>', unsafe_allow_html=True)
        
        col1, col2 = st.columns(2)
        with col1:
            if st.button("Connect", use_container_width=True):
                connect_websocket()
                st.session_state.ws_connected = True
                st.success("Connecting...")
        
        with col2:
            if st.button("Disconnect", use_container_width=True):
                disconnect_websocket()
                st.session_state.ws_connected = False
        
        st.markdown("---")
        
        # Auto-refresh
        st.markdown("### ⏱️ Auto Refresh")
        auto_refresh = st.checkbox("Enable Auto Refresh", value=True)
        refresh_rate = st.slider("Refresh Rate (seconds)", 1, 30, 5)
        
        if st.button("🔄 Manual Refresh", use_container_width=True):
            st.rerun()
    
    # Main content
    # Price ticker
    st.markdown("### 📊 Live Prices")
    
    # Get prices (from WebSocket or API)
    prices = get_latest_prices()
    if not prices:
        # Fallback to funding rates endpoint for mock prices
        rates = get_funding_rates()
        if rates.get('success') and rates.get('data'):
            prices = {
                "BTC/USD": 51000.0,
                "ETH/USD": 2850.0,
                "SOL/USD": 122.0
            }
    
    display_price_ticker(prices)
    st.session_state.last_prices = prices
    
    st.markdown("---")
    
    # Load positions
    if st.session_state.user_id:
        result = get_user_positions(st.session_state.user_id)
        if result.get('success') and result.get('data'):
            all_positions = result['data'].get('positions', [])
            open_positions = [p for p in all_positions if p.get('status', '').lower() == 'open']
            st.session_state.positions = open_positions
    
    # Risk Overview
    if st.session_state.positions:
        st.markdown("### 🎯 Risk Overview")
        
        positions = st.session_state.positions
        
        # Calculate aggregate metrics
        total_margin = sum(float(p.get('margin', 0) or 0) for p in positions)
        
        high_risk_count = 0
        medium_risk_count = 0
        low_risk_count = 0
        
        for pos in positions:
            symbol = pos.get('symbol', '')
            current_price = prices.get(symbol, float(pos.get('entry_price', 0)))
            margin_ratio = calculate_margin_ratio(pos, current_price)
            
            level, _, _ = get_risk_level(margin_ratio)
            if level == "HIGH":
                high_risk_count += 1
            elif level == "MEDIUM":
                medium_risk_count += 1
            else:
                low_risk_count += 1
        
        # Summary cards
        col1, col2, col3, col4 = st.columns(4)
        
        with col1:
            st.metric("Total Positions", len(positions))
        
        with col2:
            st.metric("🟢 Low Risk", low_risk_count)
        
        with col3:
            st.metric("🟡 Medium Risk", medium_risk_count)
        
        with col4:
            st.metric("🔴 High Risk", high_risk_count)
        
        st.markdown("---")
        
        # Individual position risks
        st.markdown("### 📋 Position Details")
        
        for position in positions:
            display_position_risk(position, prices)
    else:
        if st.session_state.user_id:
            st.info("No open positions to monitor. Open a position to see risk metrics.")
        else:
            st.info("👈 Enter your wallet address in the sidebar to monitor positions")
    
    # WebSocket updates display
    st.markdown("---")
    st.markdown("### 📡 Latest Updates")
    
    latest_update = get_latest_update()
    if latest_update:
        with st.expander("View Latest WebSocket Message"):
            st.json(latest_update)
    else:
        st.info("No WebSocket updates yet. Connect to WebSocket for real-time data.")
    
    # Auto-refresh
    if auto_refresh and st.session_state.user_id:
        time.sleep(0.1)  # Small delay to prevent rate limiting
        placeholder = st.empty()
        placeholder.markdown(f"_Last updated: {datetime.now().strftime('%H:%M:%S')}_")


if __name__ == "__main__":
    main()
