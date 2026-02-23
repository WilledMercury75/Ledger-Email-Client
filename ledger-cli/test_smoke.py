#!/usr/bin/env python3
"""
Ledger Smoke Tests — verifies basic API connectivity and responses.
Run: python3 test_smoke.py [--api http://127.0.0.1:8420]
"""

import sys
import json
from ledger_client import LedgerClient


PASS = "✅"
FAIL = "❌"
results = []


def test(name: str, condition: bool, detail: str = ""):
    status = PASS if condition else FAIL
    results.append((name, condition))
    print(f"  {status} {name}" + (f"  ({detail})" if detail else ""))


def main():
    api_url = "http://127.0.0.1:8420"
    if "--api" in sys.argv:
        idx = sys.argv.index("--api")
        if idx + 1 < len(sys.argv):
            api_url = sys.argv[idx + 1]

    client = LedgerClient(api_url)

    print("╔═══════════════════════════════════════╗")
    print("║     LEDGER SMOKE TESTS                ║")
    print(f"║     API: {api_url:<28s}║")
    print("╚═══════════════════════════════════════╝\n")

    # Test 1: Identity endpoint
    resp = client.get_identity()
    test("GET /api/identity returns success",
         resp.get("success") == True,
         f"ledger_id={resp.get('data', {}).get('ledger_id', 'N/A')[:20]}...")

    # Test 2: Identity has required fields
    data = resp.get("data", {})
    test("Identity has ledger_id",
         "ledger_id" in data and len(data.get("ledger_id", "")) > 0)
    test("Identity has public_key",
         "public_key" in data and len(data.get("public_key", "")) > 0)
    test("Identity has peer_id",
         "peer_id" in data and len(data.get("peer_id", "")) > 0)

    # Test 3: Messages endpoint
    resp = client.list_messages()
    test("GET /api/messages returns success",
         resp.get("success") == True)
    test("Messages response has data array",
         isinstance(resp.get("data"), list))

    # Test 4: Settings endpoint
    resp = client.get_settings()
    test("GET /api/settings returns success",
         resp.get("success") == True)

    # Test 5: Peers endpoint
    resp = client.list_peers()
    test("GET /api/peers returns success",
         resp.get("success") == True)

    # Test 6: Gmail config endpoint
    resp = client.get("/api/gmail/config")
    test("GET /api/gmail/config returns success",
         resp.get("success") == True)

    # Summary
    passed = sum(1 for _, ok in results if ok)
    total = len(results)
    print(f"\n{'='*40}")
    print(f"  Results: {passed}/{total} passed")
    if passed == total:
        print("  🎉 All tests passed!")
    else:
        print(f"  ⚠️  {total - passed} test(s) failed")
    print(f"{'='*40}")

    sys.exit(0 if passed == total else 1)


if __name__ == "__main__":
    main()
