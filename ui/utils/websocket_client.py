"""
WebSocket Client for Real-Time Position Updates
"""

import json
import threading
import time
from typing import Callable, Optional, Dict, Any
import websocket
import os
from dotenv import load_dotenv

load_dotenv()

# Configuration
WS_URL = os.getenv("WS_URL", "ws://localhost:3000/api/ws")


class PositionWebSocket:
    """WebSocket client for real-time position updates"""
    
    def __init__(self, url: str = WS_URL):
        self.url = url
        self.ws: Optional[websocket.WebSocketApp] = None
        self.thread: Optional[threading.Thread] = None
        self.connected = False
        self.reconnect_delay = 5
        self.max_reconnect_attempts = 10
        self.reconnect_attempts = 0
        
        # Callback handlers
        self.on_message_callback: Optional[Callable[[Dict], None]] = None
        self.on_error_callback: Optional[Callable[[str], None]] = None
        self.on_connect_callback: Optional[Callable[[], None]] = None
        self.on_disconnect_callback: Optional[Callable[[], None]] = None
        
        # Data storage
        self.latest_prices: Dict[str, float] = {}
        self.latest_update: Optional[Dict] = None
        self.messages: list = []
        self.max_messages = 100
    
    def _on_message(self, ws, message: str):
        """Handle incoming WebSocket messages"""
        try:
            data = json.loads(message)
            self.latest_update = data
            
            # Store message history
            self.messages.append(data)
            if len(self.messages) > self.max_messages:
                self.messages.pop(0)
            
            # Extract price updates
            if data.get("type") == "price_update" and "data" in data:
                self.latest_prices.update(data["data"])
            
            # Call user callback
            if self.on_message_callback:
                self.on_message_callback(data)
                
        except json.JSONDecodeError:
            pass
        except Exception as e:
            if self.on_error_callback:
                self.on_error_callback(f"Message processing error: {str(e)}")
    
    def _on_error(self, ws, error):
        """Handle WebSocket errors"""
        error_msg = str(error)
        if self.on_error_callback:
            self.on_error_callback(error_msg)
    
    def _on_close(self, ws, close_status_code, close_msg):
        """Handle WebSocket connection close"""
        self.connected = False
        if self.on_disconnect_callback:
            self.on_disconnect_callback()
    
    def _on_open(self, ws):
        """Handle WebSocket connection open"""
        self.connected = True
        self.reconnect_attempts = 0
        if self.on_connect_callback:
            self.on_connect_callback()
    
    def connect(
        self,
        on_message: Optional[Callable[[Dict], None]] = None,
        on_error: Optional[Callable[[str], None]] = None,
        on_connect: Optional[Callable[[], None]] = None,
        on_disconnect: Optional[Callable[[], None]] = None
    ):
        """Connect to WebSocket server"""
        self.on_message_callback = on_message
        self.on_error_callback = on_error
        self.on_connect_callback = on_connect
        self.on_disconnect_callback = on_disconnect
        
        self.ws = websocket.WebSocketApp(
            self.url,
            on_message=self._on_message,
            on_error=self._on_error,
            on_close=self._on_close,
            on_open=self._on_open
        )
        
        # Run in background thread
        self.thread = threading.Thread(target=self._run, daemon=True)
        self.thread.start()
    
    def _run(self):
        """Run WebSocket connection with auto-reconnect"""
        while self.reconnect_attempts < self.max_reconnect_attempts:
            try:
                self.ws.run_forever()
                
                if not self.connected:
                    self.reconnect_attempts += 1
                    time.sleep(self.reconnect_delay)
                    
                    # Recreate WebSocket app
                    self.ws = websocket.WebSocketApp(
                        self.url,
                        on_message=self._on_message,
                        on_error=self._on_error,
                        on_close=self._on_close,
                        on_open=self._on_open
                    )
                else:
                    break
                    
            except Exception as e:
                self.reconnect_attempts += 1
                time.sleep(self.reconnect_delay)
    
    def disconnect(self):
        """Disconnect from WebSocket server"""
        if self.ws:
            self.ws.close()
        self.connected = False
    
    def subscribe(self, channel: str, user_id: Optional[str] = None):
        """Subscribe to a specific channel"""
        if self.connected and self.ws:
            message = {
                "type": "subscribe",
                "channel": channel
            }
            if user_id:
                message["user_id"] = user_id
            
            self.ws.send(json.dumps(message))
    
    def unsubscribe(self, channel: str):
        """Unsubscribe from a channel"""
        if self.connected and self.ws:
            message = {
                "type": "unsubscribe",
                "channel": channel
            }
            self.ws.send(json.dumps(message))
    
    def get_latest_prices(self) -> Dict[str, float]:
        """Get the latest price updates"""
        return self.latest_prices.copy()
    
    def get_latest_update(self) -> Optional[Dict]:
        """Get the most recent update"""
        return self.latest_update
    
    def get_messages(self, count: int = 10) -> list:
        """Get recent messages"""
        return self.messages[-count:]
    
    def is_connected(self) -> bool:
        """Check if connected to WebSocket server"""
        return self.connected


# Global WebSocket client instance
ws_client = PositionWebSocket()


def connect_websocket(
    on_message: Optional[Callable[[Dict], None]] = None,
    on_error: Optional[Callable[[str], None]] = None,
    on_connect: Optional[Callable[[], None]] = None,
    on_disconnect: Optional[Callable[[], None]] = None
):
    """Connect to WebSocket server"""
    ws_client.connect(on_message, on_error, on_connect, on_disconnect)


def disconnect_websocket():
    """Disconnect from WebSocket server"""
    ws_client.disconnect()


def get_latest_prices() -> Dict[str, float]:
    """Get latest price updates"""
    return ws_client.get_latest_prices()


def get_latest_update() -> Optional[Dict]:
    """Get most recent update"""
    return ws_client.get_latest_update()


def is_connected() -> bool:
    """Check WebSocket connection status"""
    return ws_client.is_connected()


def subscribe(channel: str, user_id: Optional[str] = None):
    """Subscribe to a channel"""
    ws_client.subscribe(channel, user_id)
