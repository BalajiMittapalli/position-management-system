"""
Active Positions Page - View and manage open positions
"""

import streamlit as st
import pandas as pd
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from utils.api import get_user_positions, close_position, modify_position, position_health_check

st.set_page_config(
    page_title="Active Positions | Solana Perps",
    page_icon="📋",
    layout="wide"
)

# Custom CSS
st.markdown("""
<style>
    .position-card {
        background: linear-gradient(135deg, #1a1f2e 0%, #252b3b 100%);
        border-radius: 12px;
        padding: 20px;
        margin: 15px 0;
        border: 1px solid #2d3748;
    }
    
    .position-long {
        border-left: 4px solid #00d4aa;
    }
    
    .position-short {
        border-left: 4px solid #f85149;
    }
    
    .pnl-positive {
        color: #00d4aa;
        font-weight: bold;
        font-size: 18px;
    }
    
    .pnl-negative {
        color: #f85149;
        font-weight: bold;
        font-size: 18px;
    }
    
    .action-button {
        margin: 5px;
    }
</style>
""", unsafe_allow_html=True)


def init_session_state():
    """Initialize session state"""
    if 'user_id' not in st.session_state:
        st.session_state.user_id = ""
    if 'positions' not in st.session_state:
        st.session_state.positions = []
    if 'selected_position' not in st.session_state:
        st.session_state.selected_position = None


def load_positions(user_id: str):
    """Load positions for user"""
    result = get_user_positions(user_id)
    if result.get('success') and result.get('data'):
        # Filter only open positions
        all_positions = result['data'].get('positions', [])
        return [p for p in all_positions if p.get('status', '').lower() == 'open']
    return []


def display_position_card(position: dict, index: int):
    """Display a single position as a card"""
    side = position.get('side', 'Long')
    card_class = 'position-long' if side == 'Long' else 'position-short'
    
    with st.container():
        col1, col2, col3, col4 = st.columns([2, 2, 2, 1])
        
        with col1:
            side_emoji = "🟢" if side == 'Long' else "🔴"
            st.markdown(f"### {side_emoji} {position.get('symbol', 'N/A')}")
            st.markdown(f"**Side:** {side} | **Leverage:** {position.get('leverage', 1)}x")
        
        with col2:
            st.metric("Size", f"{float(position.get('size', 0)):,.4f}")
            st.metric("Entry Price", f"${float(position.get('entry_price', 0)):,.2f}")
        
        with col3:
            unrealized_pnl = float(position.get('unrealized_pnl', 0) or 0)
            margin = float(position.get('margin', 0) or 0)
            
            pnl_color = "normal" if unrealized_pnl >= 0 else "inverse"
            st.metric(
                "Unrealized PnL",
                f"${unrealized_pnl:,.2f}",
                delta=f"{(unrealized_pnl/margin*100) if margin > 0 else 0:.2f}%" if margin > 0 else None,
                delta_color=pnl_color
            )
            st.metric("Margin", f"${margin:,.2f}")
        
        with col4:
            position_id = position.get('id', '')
            
            if st.button("🔧 Modify", key=f"modify_{index}", use_container_width=True):
                st.session_state.selected_position = position
                st.session_state.show_modify_modal = True
            
            if st.button("❌ Close", key=f"close_{index}", use_container_width=True, type="secondary"):
                st.session_state.position_to_close = position_id
        
        st.markdown("---")


def display_modify_form(position: dict):
    """Display form to modify a position"""
    st.markdown("### 🔧 Modify Position")
    
    position_id = position.get('id', '')
    current_size = float(position.get('size', 0))
    current_margin = float(position.get('margin', 0))
    
    st.markdown(f"**Position ID:** `{position_id[:8]}...`")
    st.markdown(f"**Symbol:** {position.get('symbol')} | **Side:** {position.get('side')}")
    
    modification_type = st.selectbox(
        "Modification Type",
        options=[
            "increase_size",
            "decrease_size", 
            "add_margin",
            "remove_margin"
        ],
        format_func=lambda x: {
            "increase_size": "📈 Increase Size",
            "decrease_size": "📉 Decrease Size",
            "add_margin": "💰 Add Margin",
            "remove_margin": "💸 Remove Margin"
        }.get(x, x)
    )
    
    # Show current values
    if "size" in modification_type:
        st.info(f"Current Size: {current_size:,.4f}")
    else:
        st.info(f"Current Margin: ${current_margin:,.2f}")
    
    amount = st.number_input(
        "Amount",
        min_value=0.001,
        max_value=1000000.0,
        value=1.0,
        step=0.1,
        format="%.4f",
        help="Amount to modify"
    )
    
    # New entry price for size increase
    new_entry_price = None
    if modification_type == "increase_size":
        new_entry_price = st.number_input(
            "New Entry Price (for averaging)",
            min_value=0.01,
            max_value=1000000.0,
            value=float(position.get('entry_price', 100)),
            step=1.0,
            format="%.2f",
            help="Entry price for the additional size"
        )
    
    col1, col2 = st.columns(2)
    
    with col1:
        if st.button("✅ Confirm Modification", use_container_width=True, type="primary"):
            with st.spinner("Modifying position..."):
                result = modify_position(
                    position_id=position_id,
                    modification_type=modification_type,
                    amount=amount,
                    new_entry_price=new_entry_price
                )
                
                if result.get('success'):
                    st.success("Position modified successfully!")
                    st.json(result.get('data', {}))
                    st.session_state.selected_position = None
                    st.rerun()
                else:
                    st.error(f"Failed to modify: {result.get('error', 'Unknown error')}")
    
    with col2:
        if st.button("🔙 Cancel", use_container_width=True):
            st.session_state.selected_position = None
            st.rerun()


def main():
    init_session_state()
    
    st.markdown("# 📋 Active Positions")
    st.markdown("View and manage your open perpetual positions")
    st.markdown("---")
    
    # Sidebar for user input
    with st.sidebar:
        st.markdown("### 👛 Wallet")
        
        user_id = st.text_input(
            "Wallet Address / User ID",
            value=st.session_state.user_id,
            placeholder="Enter wallet address..."
        )
        
        if user_id:
            st.session_state.user_id = user_id
        
        if st.button("🔄 Refresh Positions", use_container_width=True):
            if user_id:
                st.session_state.positions = load_positions(user_id)
                st.success(f"Loaded {len(st.session_state.positions)} open positions")
            else:
                st.warning("Enter a wallet address first")
        
        st.markdown("---")
        
        # Quick stats
        if st.session_state.positions:
            positions = st.session_state.positions
            total_margin = sum(float(p.get('margin', 0) or 0) for p in positions)
            total_pnl = sum(float(p.get('unrealized_pnl', 0) or 0) for p in positions)
            
            st.markdown("### 📊 Summary")
            st.metric("Open Positions", len(positions))
            st.metric("Total Margin", f"${total_margin:,.2f}")
            st.metric(
                "Total Unrealized PnL",
                f"${total_pnl:,.2f}",
                delta_color="normal" if total_pnl >= 0 else "inverse"
            )
    
    # Handle position close confirmation
    if 'position_to_close' in st.session_state and st.session_state.position_to_close:
        position_id = st.session_state.position_to_close
        
        st.warning(f"⚠️ Are you sure you want to close position `{position_id[:8]}...`?")
        
        col1, col2 = st.columns(2)
        with col1:
            if st.button("✅ Yes, Close Position", type="primary"):
                with st.spinner("Closing position..."):
                    result = close_position(position_id)
                    
                    if result.get('success'):
                        st.success("Position closed successfully!")
                        st.session_state.position_to_close = None
                        # Reload positions
                        if st.session_state.user_id:
                            st.session_state.positions = load_positions(st.session_state.user_id)
                        st.rerun()
                    else:
                        st.error(f"Failed to close: {result.get('error', 'Unknown error')}")
        
        with col2:
            if st.button("❌ Cancel"):
                st.session_state.position_to_close = None
                st.rerun()
        
        st.markdown("---")
    
    # Main content
    if st.session_state.selected_position:
        display_modify_form(st.session_state.selected_position)
    else:
        # Display positions
        if not st.session_state.user_id:
            st.info("👈 Enter your wallet address in the sidebar to view positions")
        elif not st.session_state.positions:
            # Try to load positions
            positions = load_positions(st.session_state.user_id)
            if positions:
                st.session_state.positions = positions
            else:
                st.info("No open positions found. Open a new position to get started!")
                if st.button("�� Open New Position"):
                    st.switch_page("pages/1_Open_Position.py")
        
        if st.session_state.positions:
            # View options
            view_mode = st.radio(
                "View Mode",
                options=["Cards", "Table"],
                horizontal=True
            )
            
            st.markdown("---")
            
            if view_mode == "Cards":
                for idx, position in enumerate(st.session_state.positions):
                    display_position_card(position, idx)
            else:
                # Table view
                df = pd.DataFrame(st.session_state.positions)
                
                display_cols = ['symbol', 'side', 'size', 'entry_price', 'leverage', 'margin', 'unrealized_pnl']
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
                        "margin": st.column_config.NumberColumn("Margin", format="$%.2f"),
                        "unrealized_pnl": st.column_config.NumberColumn("Unrealized PnL", format="$%.2f")
                    }
                )
                
                # Actions for selected row
                st.markdown("---")
                st.markdown("### Actions")
                
                position_options = {
                    f"{p.get('symbol')} - {p.get('side')} ({p.get('id', '')[:8]}...)": p 
                    for p in st.session_state.positions
                }
                
                selected = st.selectbox("Select Position", options=list(position_options.keys()))
                
                if selected:
                    position = position_options[selected]
                    
                    col1, col2, col3 = st.columns(3)
                    
                    with col1:
                        if st.button("🔧 Modify Position", use_container_width=True):
                            st.session_state.selected_position = position
                            st.rerun()
                    
                    with col2:
                        if st.button("❌ Close Position", use_container_width=True, type="secondary"):
                            st.session_state.position_to_close = position.get('id')
                            st.rerun()
                    
                    with col3:
                        if st.button("🏥 Health Check", use_container_width=True):
                            result = position_health_check(position.get('id'))
                            if result.get('success'):
                                st.json(result.get('data', {}))
                            else:
                                st.error(result.get('error', 'Failed to check health'))


if __name__ == "__main__":
    main()
