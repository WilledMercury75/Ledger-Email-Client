"""
Ledger CLI — Python Testing & Automation Toolkit
HTTP client for the Ledger Core REST API.
"""

import json
import urllib.request
import urllib.error
from typing import Optional


class LedgerClient:
    """HTTP client for Ledger Core REST API."""

    def __init__(self, base_url: str = "http://127.0.0.1:8420"):
        self.base_url = base_url.rstrip("/")

    def _request(self, method: str, path: str, data: dict = None) -> dict:
        url = f"{self.base_url}{path}"
        body = json.dumps(data).encode() if data else None
        req = urllib.request.Request(url, data=body, method=method)
        req.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                return json.loads(resp.read().decode())
        except urllib.error.HTTPError as e:
            return {"success": False, "error": f"HTTP {e.code}: {e.reason}"}
        except urllib.error.URLError as e:
            return {"success": False, "error": f"Connection failed: {e.reason}"}

    def get(self, path: str) -> dict:
        return self._request("GET", path)

    def post(self, path: str, data: dict = None) -> dict:
        return self._request("POST", path, data)

    def put(self, path: str, data: dict = None) -> dict:
        return self._request("PUT", path, data)

    def delete(self, path: str) -> dict:
        return self._request("DELETE", path)

    # ── High-level methods ──

    def get_identity(self) -> dict:
        return self.get("/api/identity")

    def list_messages(self, folder: Optional[str] = None) -> dict:
        path = f"/api/messages?folder={folder}" if folder else "/api/messages"
        return self.get(path)

    def send_message(self, to: str, subject: str, body: str, mode: str = "auto") -> dict:
        return self.post("/api/messages", {
            "to": to, "subject": subject, "body": body, "mode": mode
        })

    def delete_message(self, msg_id: str) -> dict:
        return self.delete(f"/api/messages/{msg_id}")

    def list_peers(self) -> dict:
        return self.get("/api/peers")

    def connect_peer(self, multiaddr: str) -> dict:
        return self.post("/api/peers", {"multiaddr": multiaddr})

    def get_settings(self) -> dict:
        return self.get("/api/settings")

    def update_settings(self, delivery_mode: str = None, tor_enabled: bool = None) -> dict:
        data = {}
        if delivery_mode: data["delivery_mode"] = delivery_mode
        if tor_enabled is not None: data["tor_enabled"] = tor_enabled
        return self.put("/api/settings", data)

    def configure_gmail(self, email: str, app_password: str) -> dict:
        return self.post("/api/gmail/config", {
            "email": email, "app_password": app_password
        })

    def fetch_gmail(self) -> dict:
        return self.post("/api/gmail/fetch")

    def add_contact(self, ledger_id: str, public_key: str,
                    display_name: str = None, gmail_address: str = None) -> dict:
        data = {"ledger_id": ledger_id, "public_key": public_key}
        if display_name: data["display_name"] = display_name
        if gmail_address: data["gmail_address"] = gmail_address
        return self.post("/api/contacts", data)
