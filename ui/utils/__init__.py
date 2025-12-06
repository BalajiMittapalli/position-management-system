"""
Utils package for Solana Perpetual Position Management UI

This package provides utility modules for:
- api.py: REST API client for backend communication
- websocket_client.py: WebSocket client for real-time updates
"""

from .api import (
    PositionAPI,
    api_client,
    health_check,
    solana_status,
    open_position,
    get_position,
    close_position,
    modify_position,
    get_user_positions,
    get_user_stats,
    get_funding_rates,
    calculate_margin,
    position_health_check,
)

from .websocket_client import (
    PositionWebSocket,
    ws_client,
    connect_websocket,
    disconnect_websocket,
    get_latest_prices,
    get_latest_update,
    is_connected,
    subscribe,
)

__all__ = [
    # API
    "PositionAPI",
    "api_client",
    "health_check",
    "solana_status",
    "open_position",
    "get_position",
    "close_position",
    "modify_position",
    "get_user_positions",
    "get_user_stats",
    "get_funding_rates",
    "calculate_margin",
    "position_health_check",
    # WebSocket
    "PositionWebSocket",
    "ws_client",
    "connect_websocket",
    "disconnect_websocket",
    "get_latest_prices",
    "get_latest_update",
    "is_connected",
    "subscribe",
]
