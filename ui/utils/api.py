"""
REST API Client for Solana Perpetual Position Management Backend
"""

import requests
from typing import Optional, Dict, Any, List
import os
from dotenv import load_dotenv

load_dotenv()

# Configuration
BASE_URL = os.getenv("API_BASE_URL", "http://localhost:3000")


class PositionAPI:
    """Client for Position Management REST API"""
    
    def __init__(self, base_url: str = BASE_URL):
        self.base_url = base_url.rstrip("/")
        self.session = requests.Session()
        self.session.headers.update({
            "Content-Type": "application/json",
            "Accept": "application/json"
        })
    
    def _make_request(
        self, 
        method: str, 
        endpoint: str, 
        data: Optional[Dict] = None,
        params: Optional[Dict] = None
    ) -> Dict[str, Any]:
        """Make HTTP request to the API"""
        url = f"{self.base_url}/api{endpoint}"
        
        try:
            response = self.session.request(
                method=method,
                url=url,
                json=data,
                params=params,
                timeout=30
            )
            response.raise_for_status()
            return response.json()
        except requests.exceptions.ConnectionError:
            return {"success": False, "error": "Connection failed. Is the backend running?"}
        except requests.exceptions.Timeout:
            return {"success": False, "error": "Request timed out"}
        except requests.exceptions.HTTPError as e:
            try:
                error_data = response.json()
                return {"success": False, "error": error_data.get("error", str(e))}
            except:
                return {"success": False, "error": str(e)}
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    # ==================== Health Check ====================
    
    def health_check(self) -> Dict[str, Any]:
        """Check if the API is healthy"""
        return self._make_request("GET", "/health")
    
    def solana_status(self) -> Dict[str, Any]:
        """Check Solana integration status"""
        return self._make_request("GET", "/solana/status")
    
    # ==================== Position Operations ====================
    
    def open_position(
        self,
        user_id: str,
        symbol: str,
        side: str,
        size: float,
        entry_price: float,
        leverage: int,
        stop_loss: Optional[float] = None,
        take_profit: Optional[float] = None
    ) -> Dict[str, Any]:
        """Open a new position"""
        data = {
            "user_id": user_id,
            "symbol": symbol,
            "side": side,
            "size": str(size),
            "entry_price": str(entry_price),
            "leverage": leverage
        }
        
        if stop_loss is not None:
            data["stop_loss"] = str(stop_loss)
        if take_profit is not None:
            data["take_profit"] = str(take_profit)
        
        return self._make_request("POST", "/positions/open", data=data)
    
    def get_position(self, position_id: str) -> Dict[str, Any]:
        """Get a specific position by ID"""
        return self._make_request("GET", f"/positions/{position_id}")
    
    def close_position(self, position_id: str) -> Dict[str, Any]:
        """Close an open position"""
        return self._make_request("DELETE", f"/positions/{position_id}/close")
    
    def modify_position(
        self,
        position_id: str,
        modification_type: str,
        amount: float,
        new_entry_price: Optional[float] = None
    ) -> Dict[str, Any]:
        """Modify an existing position"""
        data = {
            "modification_type": modification_type,
            "amount": str(amount)
        }
        
        if new_entry_price is not None:
            data["new_entry_price"] = str(new_entry_price)
        
        return self._make_request("PUT", f"/positions/{position_id}/modify", data=data)
    
    # ==================== User Operations ====================
    
    def get_user_positions(
        self, 
        user_id: str, 
        limit: int = 50, 
        offset: int = 0
    ) -> Dict[str, Any]:
        """Get all positions for a user"""
        params = {"limit": limit, "offset": offset}
        return self._make_request("GET", f"/users/{user_id}/positions", params=params)
    
    def get_user_stats(self, user_id: str) -> Dict[str, Any]:
        """Get trading statistics for a user"""
        return self._make_request("GET", f"/users/{user_id}/stats")
    
    # ==================== Market Data ====================
    
    def get_funding_rates(self) -> Dict[str, Any]:
        """Get current funding rates for all symbols"""
        return self._make_request("GET", "/funding-rates")
    
    # ==================== Margin & Risk ====================
    
    def calculate_margin(
        self,
        entry_price: float,
        size: float,
        leverage: int
    ) -> Dict[str, Any]:
        """Calculate required margin for a position"""
        data = {
            "entry_price": str(entry_price),
            "size": str(size),
            "leverage": leverage
        }
        return self._make_request("POST", "/margin/calculate", data=data)
    
    def position_health_check(self, position_id: str) -> Dict[str, Any]:
        """Check the health status of a position"""
        data = {"position_id": position_id}
        return self._make_request("POST", "/positions/health-check", data=data)


# Global API client instance
api_client = PositionAPI()


# Convenience functions for direct import
def health_check() -> Dict[str, Any]:
    return api_client.health_check()


def solana_status() -> Dict[str, Any]:
    return api_client.solana_status()


def open_position(
    user_id: str,
    symbol: str,
    side: str,
    size: float,
    entry_price: float,
    leverage: int,
    stop_loss: Optional[float] = None,
    take_profit: Optional[float] = None
) -> Dict[str, Any]:
    return api_client.open_position(
        user_id, symbol, side, size, entry_price, leverage, stop_loss, take_profit
    )


def get_position(position_id: str) -> Dict[str, Any]:
    return api_client.get_position(position_id)


def close_position(position_id: str) -> Dict[str, Any]:
    return api_client.close_position(position_id)


def modify_position(
    position_id: str,
    modification_type: str,
    amount: float,
    new_entry_price: Optional[float] = None
) -> Dict[str, Any]:
    return api_client.modify_position(position_id, modification_type, amount, new_entry_price)


def get_user_positions(user_id: str, limit: int = 50, offset: int = 0) -> Dict[str, Any]:
    return api_client.get_user_positions(user_id, limit, offset)


def get_user_stats(user_id: str) -> Dict[str, Any]:
    return api_client.get_user_stats(user_id)


def get_funding_rates() -> Dict[str, Any]:
    return api_client.get_funding_rates()


def calculate_margin(entry_price: float, size: float, leverage: int) -> Dict[str, Any]:
    return api_client.calculate_margin(entry_price, size, leverage)


def position_health_check(position_id: str) -> Dict[str, Any]:
    return api_client.position_health_check(position_id)
