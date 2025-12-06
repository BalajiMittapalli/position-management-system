"""
Solana Perpetual Position Management Dashboard
Main Application Entry Point
"""

import streamlit as st
import pandas as pd
from datetime import datetime
import sys
import os

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from utils.api import (
    health_check, solana_status, get_user_positions, 
    get_user_stats, get_funding_rates
)

# Page configuration
st.set_page_config(
    page_title="Solana Perps Dashboard",
    page_icon="📊",
    layout="wide",
    initial_sidebar_state="expanded"
)

# Custom CSS for trading dashboard look
st.markdown("""
<style>
    /* Dark theme styling */
    .stApp {
        background-color: #0e1117;
    }
    
    /* Metric cards */
    .metric-card {
        background: linear-gradient(135deg, #1a1f2e 0%, #252b3b 100%);
        border-radius: 12px;
        padding: 20px;
        border: 1px solid #2d3748;
        margin: 10px 0;
    }
    
    .metric-value {
        font-size: 28px;
        font-weight: bold;
        color: #00d4aa;
    }
    
    .metric-label {
        font-size: 14px;
        color: #8b949e;
        text-transform: uppercase;
    }
    
    /* Position cards */
    .position-long {
        border-left: 4px solid #00d4aa;
        background: rgba(0, 212, 170, 0.1);
        padding: 15px;
        border-radius: 8px;
        margin: 10px 0;
    }
    
    .position-short {
        border-left: 4px solid #f85149;
        background: rgba(248, 81, 73, 0.1);
        padding: 15px;
        border-radius: 8px;
        margin: 10px 0;
    }
    
    /* Status indicators */
    .status-connected {
        color: #00d4aa;
        font-weight: bold;
    }
    
    .status-disconnected {
        color: #f85149;
        font-weight: bold;
    }
    
    /* Headers */
    .dashboard-header {
        font-size: 32px;
        font-weight: bold;
        background: linear-gradient(90deg, #00d4aa, #7c3aed);
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        margin-bottom: 20px;
    }
    
    /* Tables */
    .dataframe {
        background-color: #1a1f2e !important;
    }
    
    /* Sidebar styling */
    .css-1d391kg {
        background-color: #161b22;
    }
</style>
""", unsafe_allow_html=True)


def init_session_state():
    """Initialize session state variables"""
    if 'user_id' not in st.session_state:
        st.session_state.user_id = ""
    if 'positions' not in st.session_state:
        st.session_state.positions = []
    if 'connected' not in st.session_state:
        st.session_state.connected = False


def check_backend_status():
    """Check backend and Solana connection status"""
    health = health_check()
    solana = solana_status()
    
    return {
        'backend': health.get('success', False) or 'status' in health,
        'solana': solana.get('data', {}).get('solana_enabled', False) if solana.get('success') else False,
        'health_data': health,
        'solana_data': solana
    }


def display_header():
    """Display dashboard header"""
    col1, col2 = st.columns([3, 1])
    
    with col1:
        st.markdown('<p class="dashboard-header">🚀 Solana Perpetual Position Manager</p>', unsafe_allow_html=True)
    
    with col2:
        status = check_backend_status()
        if status['backend']:
            st.markdown('🟢 <span class="status-connected">Backend Connected</span>', unsafe_allow_html=True)
        else:
            st.markdown('🔴 <span class="status-disconnected">Backend Offline</span>', unsafe_allow_html=True)


def display_sidebar():
    """Display sidebar with wallet input and navigation"""
    with st.sidebar:
        st.markdown("### 👛 Wallet Connection")
        
        user_id = st.text_input(
            "Wallet Address / User ID",
            value=st.session_state.user_id,
            placeholder="Enter your wallet address...",
            help="Enter your Solana wallet address or user ID"
        )
        
        if user_id != st.session_state.user_id:
            st.session_state.user_id = user_id
            st.session_state.positions = []
        
        if st.button("🔄 Load Positions", use_container_width=True):
            if user_id:
                with st.spinner("Loading positions..."):
                    result = get_user_positions(user_id)
                    if result.get('success') and result.get('data'):
                        st.session_state.positions = result['data'].get('positions', [])
                        st.success(f"Loaded {len(st.session_state.positions)} positions")
                    else:
                        st.error(result.get('error', 'Failed to load positions'))
            else:
                st.warning("Please enter a wallet address")
        
        st.markdown("---")
        
        # Quick stats
        if st.session_state.user_id:
            stats = get_user_stats(st.session_state.user_id)
            if stats.get('success') and stats.get('data'):
                data = stats['data']
                st.markdown("### 📊 Quick Stats")
                st.metric("Total Positions", data.get('total_positions', 0))
                st.metric("Open Positions", data.get('open_positions', 0))
        
        st.markdown("---")
        st.markdown("### 📈 Funding Rates")
        
        rates = get_funding_rates()
        if rates.get('success') and rates.get('data'):
            for symbol, rate in rates['data'].items():
                rate_pct = float(rate) * 100
                color = "🟢" if rate_pct >= 0 else "🔴"
                st.markdown(f"{color} **{symbol}**: {rate_pct:.4f}%")


def display_positions_table(positions: list):
    """Display positions in a styled table"""
    if not positions:
        st.info("No positions found. Open a new position to get started!")
        return
    
    # Convert to DataFrame
    df = pd.DataFrame(positions)
    
    # Select and rename columns
    display_cols = {
        'symbol': 'Symbol',
        'side': 'Side',
        'size': 'Size',
        'entry_price': 'Entry Price',
        'leverage': 'Leverage',
        'margin': 'Margin',
        'status': 'Status',
        'unrealized_pnl': 'Unrealized PnL',
        'realized_pnl': 'Realized PnL'
    }
    
    available_cols = [col for col in display_cols.keys() if col in df.columns]
    df_display = df[available_cols].rename(columns={k: v for k, v in display_cols.items() if k in available_cols})
    
    # Style the dataframe
    def style_side(val):
        if val == 'Long':
            return 'color: #00d4aa; font-weight: bold'
        elif val == 'Short':
            return 'color: #f85149; font-weight: bold'
        return ''
    
    def style_pnl(val):
        try:
            num = float(val) if val else 0
            if num > 0:
                return 'color: #00d4aa'
            elif num < 0:
                return 'color: #f85149'
        except:
            pass
        return ''
    
    def style_status(val):
        if val.lower() == 'open' if val else False:
            return 'color: #00d4aa'
        elif val == 'closed':
            return 'color: #8b949e'
        elif val == 'liquidated':
            return 'color: #f85149'
        return ''
    
    # Apply styling
    styled_df = df_display.style
    if 'Side' in df_display.columns:
        styled_df = styled_df.applymap(style_side, subset=['Side'])
    if 'Status' in df_display.columns:
        styled_df = styled_df.applymap(style_status, subset=['Status'])
    if 'Unrealized PnL' in df_display.columns:
        styled_df = styled_df.applymap(style_pnl, subset=['Unrealized PnL'])
    if 'Realized PnL' in df_display.columns:
        styled_df = styled_df.applymap(style_pnl, subset=['Realized PnL'])
    
    st.dataframe(styled_df, use_container_width=True, hide_index=True)


def display_metrics(positions: list):
    """Display key metrics from positions"""
    if not positions:
        return
    
    # Calculate metrics
    open_positions = [p for p in positions if p.get('status', '').lower() == 'open']
    long_positions = [p for p in open_positions if p.get('side') == 'Long']
    short_positions = [p for p in open_positions if p.get('side') == 'Short']
    
    total_unrealized = sum(float(p.get('unrealized_pnl', 0) or 0) for p in open_positions)
    total_realized = sum(float(p.get('realized_pnl', 0) or 0) for p in positions)
    total_margin = sum(float(p.get('margin', 0) or 0) for p in open_positions)
    
    col1, col2, col3, col4 = st.columns(4)
    
    with col1:
        st.metric(
            label="Open Positions",
            value=len(open_positions),
            delta=f"{len(long_positions)}L / {len(short_positions)}S"
        )
    
    with col2:
        st.metric(
            label="Unrealized PnL",
            value=f"${total_unrealized:,.2f}",
            delta="Live" if total_unrealized != 0 else None,
            delta_color="normal" if total_unrealized >= 0 else "inverse"
        )
    
    with col3:
        st.metric(
            label="Realized PnL",
            value=f"${total_realized:,.2f}",
            delta_color="normal" if total_realized >= 0 else "inverse"
        )
    
    with col4:
        st.metric(
            label="Total Margin Used",
            value=f"${total_margin:,.2f}"
        )


def main():
    """Main application entry point"""
    init_session_state()
    display_header()
    display_sidebar()
    
    # Main content area
    st.markdown("---")
    
    # Display metrics if we have positions
    if st.session_state.positions:
        display_metrics(st.session_state.positions)
        st.markdown("---")
    
    # Tabs for different views
    tab1, tab2 = st.tabs(["📊 All Positions", "📈 Open Positions"])
    
    with tab1:
        st.markdown("### All Positions")
        display_positions_table(st.session_state.positions)
    
    with tab2:
        st.markdown("### Open Positions")
        open_positions = [p for p in st.session_state.positions if p.get('status', '').lower() == 'open']
        display_positions_table(open_positions)
    
    # Quick actions
    st.markdown("---")
    st.markdown("### ⚡ Quick Actions")
    
    col1, col2, col3, col4 = st.columns(4)
    
    with col1:
        if st.button("📈 Open Position", use_container_width=True):
            st.switch_page("pages/1_Open_Position.py")
    
    with col2:
        if st.button("📋 Active Positions", use_container_width=True):
            st.switch_page("pages/2_Active_Positions.py")
    
    with col3:
        if st.button("⚠️ Risk Monitor", use_container_width=True):
            st.switch_page("pages/3_Risk_Monitor.py")
    
    with col4:
        if st.button("💰 PnL History", use_container_width=True):
            st.switch_page("pages/4_PnL_History.py")
    
    # Footer
    st.markdown("---")
    st.markdown(
        """
        <div style="text-align: center; color: #8b949e; font-size: 12px;">
            Solana Perpetual Position Management System | Built with Streamlit
        </div>
        """,
        unsafe_allow_html=True
    )


if __name__ == "__main__":
    main()
