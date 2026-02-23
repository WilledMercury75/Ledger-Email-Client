#!/usr/bin/env python3
"""
Ledger CLI — Interactive command-line tool for Ledger Mail Client.
Usage: python3 ledger_cli.py [--api URL]
"""

import sys
import json
from ledger_client import LedgerClient


def print_banner():
    print("""
╔══════════════════════════════════════════╗
║     🔐 LEDGER CLI — Encrypted Mail      ║
║        Testing & Automation Tool         ║
╠══════════════════════════════════════════╣
║  Type 'help' for commands               ║
╚══════════════════════════════════════════╝
""")


def print_help():
    print("""
  Available Commands:
  ─────────────────────────────────────
  identity        Show your Ledger ID and peer info
  inbox           List inbox messages
  sent            List sent messages
  send            Send a message (interactive)
  peers           List connected peers
  connect <addr>  Connect to a peer
  settings        Show current settings
  mode <mode>     Set delivery mode (auto/p2p_only/gmail_only)
  gmail-config    Configure Gmail (interactive)
  gmail-fetch     Fetch Gmail messages
  status          Check API connectivity
  help            Show this help
  quit            Exit
""")


def format_json(data):
    print(json.dumps(data, indent=2))


def main():
    api_url = "http://127.0.0.1:8420"
    if "--api" in sys.argv:
        idx = sys.argv.index("--api")
        if idx + 1 < len(sys.argv):
            api_url = sys.argv[idx + 1]

    client = LedgerClient(api_url)
    print_banner()
    print(f"  API: {api_url}")

    # Check connectivity
    resp = client.get_identity()
    if resp.get("success"):
        data = resp.get("data", {})
        print(f"  Connected! Ledger ID: {data.get('ledger_id', 'unknown')}")
    else:
        print(f"  ⚠ Not connected: {resp.get('error', 'unknown')}")
        print("  (Commands will fail until ledger-core is running)")
    print()

    while True:
        try:
            cmd = input("ledger> ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\nGoodbye!")
            break

        if not cmd:
            continue

        parts = cmd.split(maxsplit=1)
        action = parts[0].lower()
        arg = parts[1] if len(parts) > 1 else ""

        if action == "quit" or action == "exit":
            print("Goodbye!")
            break
        elif action == "help":
            print_help()
        elif action == "identity":
            format_json(client.get_identity())
        elif action == "inbox":
            resp = client.list_messages("inbox")
            messages = resp.get("data", [])
            if not messages:
                print("  No messages in inbox.")
            else:
                for m in messages:
                    icon = {"p2p": "🔒", "gmail": "📧", "fallback": "⚠️"}.get(m.get("delivery_method", ""), "❓")
                    print(f"  {icon} {m.get('subject', '(no subject)')}")
                    print(f"     From: {m.get('from_id', '?')}  |  {m.get('id', '')[:8]}...")
        elif action == "sent":
            resp = client.list_messages("sent")
            messages = resp.get("data", [])
            if not messages:
                print("  No sent messages.")
            else:
                for m in messages:
                    print(f"  → {m.get('subject', '(no subject)')} → {m.get('to_id', '?')}")
        elif action == "send":
            to = input("  To (ledger:... or email): ").strip()
            subject = input("  Subject: ").strip()
            body = input("  Body: ").strip()
            mode = input("  Mode (auto/p2p_only/gmail_only) [auto]: ").strip() or "auto"
            resp = client.send_message(to, subject, body, mode)
            if resp.get("success"):
                print(f"  ✅ Sent via {resp['data'].get('delivery_method', '?')}")
            else:
                print(f"  ❌ Failed: {resp.get('error', 'unknown')}")
        elif action == "peers":
            format_json(client.list_peers())
        elif action == "connect":
            if not arg:
                arg = input("  Multiaddr: ").strip()
            format_json(client.connect_peer(arg))
        elif action == "settings":
            format_json(client.get_settings())
        elif action == "mode":
            if arg not in ("auto", "p2p_only", "gmail_only"):
                print("  Usage: mode auto|p2p_only|gmail_only")
            else:
                format_json(client.update_settings(delivery_mode=arg))
        elif action == "gmail-config":
            email = input("  Gmail address: ").strip()
            pw = input("  App password: ").strip()
            format_json(client.configure_gmail(email, pw))
        elif action == "gmail-fetch":
            format_json(client.fetch_gmail())
        elif action == "status":
            resp = client.get_identity()
            if resp.get("success"):
                print("  ✅ Ledger Core is running")
                format_json(resp)
            else:
                print(f"  ❌ Not connected: {resp.get('error')}")
        else:
            print(f"  Unknown command: {action}. Type 'help' for commands.")


if __name__ == "__main__":
    main()
