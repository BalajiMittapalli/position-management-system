"""
PnL History Page - Historical PnL tracking and visualization
"""

import streamlit as st
import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
from plotly.subplots import make_subplots
from datetime import datetime, timedelta
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from utils.api import get_user_positions, get_user_stats

st.set_page_config(
    page_title="PnL History | Solana Perps",
    page_icon="💰",
    layout="wide"
)

# Custom CSS
st.markdown("""
<style>
    .pnl-positive {
        color: #00d4aa;
        font-size: 32px;
        font-weight: bold;
    }
    
    .pnl-negative {
        color: #f85149;
        font-size: 32px;
        font-weight: bold;
    }
    
    .stat-card {
        background: linear-gradient(135deg, #1a1f2e 0%, #252b3b 100%);
        border-radius: 12px;
        padding: 20px;
        margin: 10px 0;
        border: 1px solid #2d3748;
        text-align: center;
    }
    
    .win-rate-high {
        color: #00d4aa;
    }
    
    .win-rate-low {
        color: #f85149;
    }
</style>
""", unsafe_allow_html=True)


def init_session_state():
    """Initialize session state"""
    if 'user_id' not in st.session_state:
        st.session_state.user_id = ""
    if 'positions' not in st.session_state:
        st.session_state.positions = []


def calculate_pnl_stats(positions: list) -> dict:
    """Calculate PnL statistics from positions"""
    if not positions:
        return {
            'total_realized': 0,
            'total_unrealized': 0,
            'total_pnl': 0,
            'winning_trades': 0,
            'losing_trades': 0,
            'win_rate': 0,
            'largest_win': 0,
            'largest_loss': 0,
            'avg_win': 0,
            'avg_loss': 0,
            'profit_factor': 0
        }
    
    total_realized = 0
    total_unrealized = 0
    wins = []
    losses = []
    
    for pos in positions:
        realized = float(pos.get('realized_pnl', 0) or 0)
        unrealized = float(pos.get('unrealized_pnl', 0) or 0)
        
        total_realized += realized
        total_unrealized += unrealized
        
        if pos.get('status') == 'closed':
            if realized > 0:
                wins.append(realized)
            elif realized < 0:
                losses.append(abs(realized))
    
    total_wins = sum(wins)
    total_losses = sum(losses)
    
    return {
        'total_realized': total_realized,
        'total_unrealized': total_unrealized,
        'total_pnl': total_realized + total_unrealized,
        'winning_trades': len(wins),
        'losing_trades': len(losses),
        'win_rate': (len(wins) / (len(wins) + len(losses)) * 100) if (len(wins) + len(losses)) > 0 else 0,
        'largest_win': max(wins) if wins else 0,
        'largest_loss': max(losses) if losses else 0,
        'avg_win': (total_wins / len(wins)) if wins else 0,
        'avg_loss': (total_losses / len(losses)) if losses else 0,
        'profit_factor': (total_wins / total_losses) if total_losses > 0 else float('inf') if total_wins > 0 else 0
    }


def create_pnl_chart(positions: list):
    """Create PnL over time chart"""
    if not positions:
        return None
    
    # Sort by created_at
    sorted_positions = sorted(positions, key=lambda x: x.get('created_at', ''))
    
    # Calculate cumulative PnL
    cumulative_pnl = []
    running_total = 0
    dates = []
    
    for pos in sorted_positions:
        pnl = float(pos.get('realized_pnl', 0) or 0) + float(pos.get('unrealized_pnl', 0) or 0)
        running_total += pnl
        cumulative_pnl.append(running_total)
        
        # Parse date
        date_str = pos.get('created_at', '')
        if date_str:
            try:
                if 'T' in date_str:
                    date = datetime.fromisoformat(date_str.replace('Z', '+00:00'))
                else:
                    date = datetime.now()
            except:
                date = datetime.now()
        else:
            date = datetime.now()
        dates.append(date)
    
    # Create chart
    fig = go.Figure()
    
    # Add cumulative PnL line
    fig.add_trace(go.Scatter(
        x=dates,
        y=cumulative_pnl,
        mode='lines+markers',
        name='Cumulative PnL',
        line=dict(color='#00d4aa', width=2),
        fill='tozeroy',
        fillcolor='rgba(0, 212, 170, 0.1)'
    ))
    
    # Add zero line
    fig.add_hline(y=0, line_dash="dash", line_color="gray")
    
    fig.update_layout(
        title='Cumulative PnL Over Time',
        xaxis_title='Date',
        yaxis_title='PnL ($)',
        template='plotly_dark',
        paper_bgcolor='rgba(0,0,0,0)',
        plot_bgcolor='rgba(0,0,0,0)',
        height=400
    )
    
    return fig


def create_pnl_distribution_chart(positions: list):
    """Create PnL distribution chart"""
    if not positions:
        return None
    
    closed_positions = [p for p in positions if p.get('status') == 'closed']
    
    if not closed_positions:
        return None
    
    pnls = [float(p.get('realized_pnl', 0) or 0) for p in closed_positions]
    
    # Create histogram
    fig = go.Figure()
    
    fig.add_trace(go.Histogram(
        x=pnls,
        nbinsx=20,
        marker_color=['#00d4aa' if x >= 0 else '#f85149' for x in pnls],
        name='Trade PnL'
    ))
    
    fig.update_layout(
        title='PnL Distribution',
        xaxis_title='PnL ($)',
        yaxis_title='Number of Trades',
        template='plotly_dark',
        paper_bgcolor='rgba(0,0,0,0)',
        plot_bgcolor='rgba(0,0,0,0)',
        height=300
    )
    
    return fig


def create_symbol_performance_chart(positions: list):
    """Create performance by symbol chart"""
    if not positions:
        return None
    
    # Aggregate by symbol
    symbol_pnl = {}
    for pos in positions:
        symbol = pos.get('symbol', 'Unknown')
        pnl = float(pos.get('realized_pnl', 0) or 0) + float(pos.get('unrealized_pnl', 0) or 0)
        symbol_pnl[symbol] = symbol_pnl.get(symbol, 0) + pnl
    
    if not symbol_pnl:
        return None
    
    symbols = list(symbol_pnl.keys())
    pnls = list(symbol_pnl.values())
    colors = ['#00d4aa' if p >= 0 else '#f85149' for p in pnls]
    
    fig = go.Figure()
    
    fig.add_trace(go.Bar(
        x=symbols,
        y=pnls,
        marker_color=colors,
        name='PnL by Symbol'
    ))
    
    fig.update_layout(
        title='Performance by Symbol',
        xaxis_title='Symbol',
        yaxis_title='PnL ($)',
        template='plotly_dark',
        paper_bgcolor='rgba(0,0,0,0)',
        plot_bgcolor='rgba(0,0,0,0)',
        height=300
    )
    
    return fig


def create_win_loss_pie(stats: dict):
    """Create win/loss pie chart"""
    wins = stats['winning_trades']
    losses = stats['losing_trades']
    
    if wins == 0 and losses == 0:
        return None
    
    fig = go.Figure()
    
    fig.add_trace(go.Pie(
        labels=['Winning Trades', 'Losing Trades'],
        values=[wins, losses],
        marker=dict(colors=['#00d4aa', '#f85149']),
        hole=0.6,
        textinfo='label+percent'
    ))
    
    fig.update_layout(
        title='Win/Loss Ratio',
        template='plotly_dark',
        paper_bgcolor='rgba(0,0,0,0)',
        plot_bgcolor='rgba(0,0,0,0)',
        height=300,
        showlegend=False,
        annotations=[dict(
            text=f'{stats["win_rate"]:.1f}%',
            x=0.5, y=0.5,
            font_size=24,
            showarrow=False,
            font_color='#00d4aa' if stats['win_rate'] >= 50 else '#f85149'
        )]
    )
    
    return fig


def main():
    init_session_state()
    
    st.markdown("# 💰 PnL History & Analytics")
    st.markdown("Track your trading performance and analyze historical PnL")
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
        
        if st.button("🔄 Load Data", use_container_width=True):
            if user_id:
                result = get_user_positions(user_id)
                if result.get('success') and result.get('data'):
                    st.session_state.positions = result['data'].get('positions', [])
                    st.success(f"Loaded {len(st.session_state.positions)} positions")
                else:
                    st.error(result.get('error', 'Failed to load data'))
            else:
                st.warning("Enter a wallet address first")
        
        st.markdown("---")
        
        # Time filter
        st.markdown("### 📅 Time Filter")
        time_range = st.selectbox(
            "Show data for",
            options=["All Time", "Last 7 Days", "Last 30 Days", "Last 90 Days"]
        )
    
    # Main content
    if not st.session_state.user_id:
        st.info("👈 Enter your wallet address in the sidebar to view PnL history")
        return
    
    # Load positions if not already loaded
    if not st.session_state.positions:
        result = get_user_positions(st.session_state.user_id)
        if result.get('success') and result.get('data'):
            st.session_state.positions = result['data'].get('positions', [])
    
    positions = st.session_state.positions
    
    if not positions:
        st.info("No trading history found. Start trading to see your PnL history!")
        return
    
    # Calculate stats
    stats = calculate_pnl_stats(positions)
    
    # Summary cards
    st.markdown("### 📊 Performance Summary")
    
    col1, col2, col3, col4 = st.columns(4)
    
    with col1:
        total_pnl = stats['total_pnl']
        pnl_color = "pnl-positive" if total_pnl >= 0 else "pnl-negative"
        st.markdown(f"""
        <div class="stat-card">
            <p style="color: #8b949e; margin-bottom: 5px;">Total PnL</p>
            <p class="{pnl_color}">${total_pnl:,.2f}</p>
        </div>
        """, unsafe_allow_html=True)
    
    with col2:
        win_rate = stats['win_rate']
        win_color = "win-rate-high" if win_rate >= 50 else "win-rate-low"
        st.markdown(f"""
        <div class="stat-card">
            <p style="color: #8b949e; margin-bottom: 5px;">Win Rate</p>
            <p class="{win_color}" style="font-size: 32px; font-weight: bold;">{win_rate:.1f}%</p>
        </div>
        """, unsafe_allow_html=True)
    
    with col3:
        st.markdown(f"""
        <div class="stat-card">
            <p style="color: #8b949e; margin-bottom: 5px;">Profit Factor</p>
            <p style="font-size: 32px; font-weight: bold; color: {'#00d4aa' if stats['profit_factor'] >= 1 else '#f85149'}">
                {stats['profit_factor']:.2f}
            </p>
        </div>
        """, unsafe_allow_html=True)
    
    with col4:
        st.markdown(f"""
        <div class="stat-card">
            <p style="color: #8b949e; margin-bottom: 5px;">Total Trades</p>
            <p style="font-size: 32px; font-weight: bold; color: white;">
                {stats['winning_trades'] + stats['losing_trades']}
            </p>
        </div>
        """, unsafe_allow_html=True)
    
    st.markdown("---")
    
    # Detailed stats
    col1, col2 = st.columns(2)
    
    with col1:
        st.markdown("### 💵 PnL Breakdown")
        st.metric("Realized PnL", f"${stats['total_realized']:,.2f}")
        st.metric("Unrealized PnL", f"${stats['total_unrealized']:,.2f}")
        st.metric("Largest Win", f"${stats['largest_win']:,.2f}")
        st.metric("Largest Loss", f"-${stats['largest_loss']:,.2f}")
    
    with col2:
        st.markdown("### 📈 Trade Statistics")
        st.metric("Winning Trades", stats['winning_trades'])
        st.metric("Losing Trades", stats['losing_trades'])
        st.metric("Avg Win", f"${stats['avg_win']:,.2f}")
        st.metric("Avg Loss", f"-${stats['avg_loss']:,.2f}")
    
    st.markdown("---")
    
    # Charts
    st.markdown("### 📉 Performance Charts")
    
    # Cumulative PnL chart
    pnl_chart = create_pnl_chart(positions)
    if pnl_chart:
        st.plotly_chart(pnl_chart, use_container_width=True)
    
    # Two column charts
    col1, col2 = st.columns(2)
    
    with col1:
        win_loss_chart = create_win_loss_pie(stats)
        if win_loss_chart:
            st.plotly_chart(win_loss_chart, use_container_width=True)
    
    with col2:
        symbol_chart = create_symbol_performance_chart(positions)
        if symbol_chart:
            st.plotly_chart(symbol_chart, use_container_width=True)
    
    # PnL distribution
    dist_chart = create_pnl_distribution_chart(positions)
    if dist_chart:
        st.plotly_chart(dist_chart, use_container_width=True)
    
    st.markdown("---")
    
    # Trade history table
    st.markdown("### 📋 Trade History")
    
    df = pd.DataFrame(positions)
    
    if not df.empty:
        display_cols = ['symbol', 'side', 'size', 'entry_price', 'leverage', 'realized_pnl', 'unrealized_pnl', 'status', 'created_at']
        available_cols = [c for c in display_cols if c in df.columns]
        
        st.dataframe(
            df[available_cols],
            use_container_width=True,
            hide_index=True,
            column_config={
                "symbol": "Symbol",
                "side": "Side",
                "size": st.column_config.NumberColumn("Size", format="%.4f"),
                "entry_price": st.column_config.NumberColumn("Entry Price", format="$%.2f"),
                "leverage": "Leverage",
                "realized_pnl": st.column_config.NumberColumn("Realized PnL", format="$%.2f"),
                "unrealized_pnl": st.column_config.NumberColumn("Unrealized PnL", format="$%.2f"),
                "status": "Status",
                "created_at": "Date"
            }
        )
    
    # Export options
    st.markdown("---")
    st.markdown("### 📥 Export Data")
    
    col1, col2 = st.columns(2)
    
    with col1:
        if st.button("📊 Export to CSV", use_container_width=True):
            csv = df.to_csv(index=False)
            st.download_button(
                label="Download CSV",
                data=csv,
                file_name=f"pnl_history_{st.session_state.user_id[:8]}.csv",
                mime="text/csv"
            )
    
    with col2:
        if st.button("📈 Export Summary", use_container_width=True):
            summary = {
                "User": st.session_state.user_id,
                "Generated": datetime.now().isoformat(),
                **stats
            }
            import json
            st.download_button(
                label="Download Summary JSON",
                data=json.dumps(summary, indent=2),
                file_name=f"pnl_summary_{st.session_state.user_id[:8]}.json",
                mime="application/json"
            )


if __name__ == "__main__":
    main()
