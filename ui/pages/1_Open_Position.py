"""
Open Position Page - Create new perpetual positions
"""

import streamlit as st
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from utils.api import open_position, calculate_margin, get_funding_rates

st.set_page_config(
    page_title="Open Position | Solana Perps",
    page_icon="📈",
    layout="wide"
)

# Custom CSS
st.markdown("""
<style>
    .success-box {
        background: rgba(0, 212, 170, 0.1);
        border: 1px solid #00d4aa;
        border-radius: 8px;
        padding: 20px;
        margin: 20px 0;
    }
    
    .error-box {
        background: rgba(248, 81, 73, 0.1);
        border: 1px solid #f85149;
        border-radius: 8px;
        padding: 20px;
        margin: 20px 0;
    }
    
    .info-card {
        background: linear-gradient(135deg, #1a1f2e 0%, #252b3b 100%);
        border-radius: 12px;
        padding: 20px;
        border: 1px solid #2d3748;
    }
    
    .long-button {
        background-color: #00d4aa !important;
        color: white !important;
    }
    
    .short-button {
        background-color: #f85149 !important;
        color: white !important;
    }
</style>
""", unsafe_allow_html=True)


def init_session_state():
    """Initialize session state"""
    if 'user_id' not in st.session_state:
        st.session_state.user_id = ""
    if 'last_position_result' not in st.session_state:
        st.session_state.last_position_result = None


def display_margin_preview(entry_price: float, size: float, leverage: int):
    """Display margin calculation preview"""
    if entry_price > 0 and size > 0 and leverage > 0:
        result = calculate_margin(entry_price, size, leverage)
        if result.get('success') and result.get('data'):
            margin = float(result['data'].get('required_margin', 0))
            position_value = entry_price * size
            
            col1, col2, col3 = st.columns(3)
            with col1:
                st.metric("Position Value", f"${position_value:,.2f}")
            with col2:
                st.metric("Required Margin", f"${margin:,.2f}")
            with col3:
                st.metric("Effective Leverage", f"{leverage}x")


def main():
    init_session_state()
    
    st.markdown("# 📈 Open New Position")
    st.markdown("Create a new perpetual position on Solana")
    st.markdown("---")
    
    # Two column layout
    col_form, col_info = st.columns([2, 1])
    
    with col_form:
        st.markdown("### Position Details")
        
        # User ID
        user_id = st.text_input(
            "Wallet Address / User ID *",
            value=st.session_state.user_id,
            placeholder="Enter your wallet address...",
            help="Your Solana wallet address or user identifier"
        )
        
        if user_id:
            st.session_state.user_id = user_id
        
        # Symbol selection
        symbol = st.selectbox(
            "Trading Pair *",
            options=["BTC/USD", "ETH/USD", "SOL/USD"],
            help="Select the perpetual contract to trade"
        )
        
        # Side selection with styled buttons
        st.markdown("#### Position Side *")
        side_col1, side_col2 = st.columns(2)
        
        with side_col1:
            long_selected = st.button("🟢 LONG", use_container_width=True, type="primary")
        with side_col2:
            short_selected = st.button("🔴 SHORT", use_container_width=True)
        
        # Track selected side in session state
        if 'selected_side' not in st.session_state:
            st.session_state.selected_side = "Long"
        
        if long_selected:
            st.session_state.selected_side = "Long"
        if short_selected:
            st.session_state.selected_side = "Short"
        
        side = st.session_state.selected_side
        
        if side == "Long":
            st.success("📈 **LONG** - Profit when price goes UP")
        else:
            st.error("📉 **SHORT** - Profit when price goes DOWN")
        
        # Size and price inputs
        col1, col2 = st.columns(2)
        
        with col1:
            size = st.number_input(
                "Position Size *",
                min_value=0.001,
                max_value=1000000.0,
                value=1.0,
                step=0.1,
                format="%.4f",
                help="Size of the position in base currency"
            )
        
        with col2:
            # Default prices based on symbol
            default_prices = {
                "BTC/USD": 50000.0,
                "ETH/USD": 2800.0,
                "SOL/USD": 120.0
            }
            
            entry_price = st.number_input(
                "Entry Price *",
                min_value=0.01,
                max_value=1000000.0,
                value=default_prices.get(symbol, 100.0),
                step=1.0,
                format="%.2f",
                help="Entry price for the position"
            )
        
        # Leverage slider
        leverage = st.slider(
            "Leverage *",
            min_value=1,
            max_value=100,
            value=10,
            help="Position leverage (1x - 100x)"
        )
        
        # Leverage warning
        if leverage > 50:
            st.warning("⚠️ High leverage increases liquidation risk!")
        elif leverage > 20:
            st.info("ℹ️ Moderate leverage - manage risk carefully")
        
        st.markdown("---")
        
        # Optional: Stop Loss and Take Profit
        with st.expander("🎯 Stop Loss & Take Profit (Optional)"):
            sl_col, tp_col = st.columns(2)
            
            with sl_col:
                stop_loss = st.number_input(
                    "Stop Loss Price",
                    min_value=0.0,
                    max_value=1000000.0,
                    value=0.0,
                    step=1.0,
                    format="%.2f",
                    help="Price at which to automatically close position at loss"
                )
            
            with tp_col:
                take_profit = st.number_input(
                    "Take Profit Price",
                    min_value=0.0,
                    max_value=1000000.0,
                    value=0.0,
                    step=1.0,
                    format="%.2f",
                    help="Price at which to automatically close position at profit"
                )
        
        st.markdown("---")
        
        # Margin preview
        st.markdown("### 💰 Margin Preview")
        display_margin_preview(entry_price, size, leverage)
        
        st.markdown("---")
        
        # Submit button
        if st.button("🚀 Open Position", use_container_width=True, type="primary"):
            if not user_id:
                st.error("Please enter a wallet address / user ID")
            elif size <= 0:
                st.error("Position size must be greater than 0")
            elif entry_price <= 0:
                st.error("Entry price must be greater than 0")
            else:
                with st.spinner("Opening position..."):
                    result = open_position(
                        user_id=user_id,
                        symbol=symbol,
                        side=side,
                        size=size,
                        entry_price=entry_price,
                        leverage=leverage,
                        stop_loss=stop_loss if stop_loss > 0 else None,
                        take_profit=take_profit if take_profit > 0 else None
                    )
                    
                    st.session_state.last_position_result = result
        
        # Display result
        if st.session_state.last_position_result:
            result = st.session_state.last_position_result
            if result.get('success') and result.get('data'):
                data = result['data']
                st.markdown("""
                <div class="success-box">
                    <h3>✅ Position Opened Successfully!</h3>
                </div>
                """, unsafe_allow_html=True)
                
                st.json(data)
                
                if st.button("Open Another Position"):
                    st.session_state.last_position_result = None
                    st.rerun()
            else:
                st.markdown(f"""
                <div class="error-box">
                    <h3>❌ Failed to Open Position</h3>
                    <p>{result.get('error', 'Unknown error occurred')}</p>
                </div>
                """, unsafe_allow_html=True)
    
    with col_info:
        st.markdown("### 📊 Market Info")
        
        # Funding rates
        rates = get_funding_rates()
        if rates.get('success') and rates.get('data'):
            st.markdown("#### Funding Rates")
            for sym, rate in rates['data'].items():
                rate_pct = float(rate) * 100
                color = "🟢" if rate_pct >= 0 else "🔴"
                st.markdown(f"{color} **{sym}**: {rate_pct:.4f}%")
        
        st.markdown("---")
        
        # Position info card
        st.markdown("#### ℹ️ Position Info")
        st.markdown("""
        **Perpetual Contracts:**
        - No expiration date
        - Funding rate every 8 hours
        - Maximum leverage: 100x
        
        **Risk Management:**
        - Always use stop losses
        - Don't over-leverage
        - Monitor liquidation price
        """)
        
        st.markdown("---")
        
        # Quick tips
        st.markdown("#### 💡 Quick Tips")
        st.info("""
        1. **Start small** - Test with small sizes first
        2. **Set stop losses** - Protect your capital
        3. **Watch funding** - High rates can eat profits
        4. **Monitor margin** - Keep buffer for volatility
        """)


if __name__ == "__main__":
    main()
