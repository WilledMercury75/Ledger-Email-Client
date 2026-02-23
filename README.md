# 🔐 Ledger — Decentralized Encrypted Mail Client

A hybrid peer-to-peer and Gmail bridge email client with end-to-end encryption, built with a polyglot architecture (Rust core, C# desktop UI, Java plugins, Python CLI).

## Architecture Overview

```
┌────────────────────────────────────────────────────┐
│                 Ledger Desktop UI                  │
│              (C# / Avalonia — MVVM)                │
└────────────────────┬───────────────────────────────┘
                     │ REST API (HTTP 127.0.0.1:8420)
┌────────────────────▼───────────────────────────────┐
│                  Ledger Core                       │
│                  (Rust / Actix-web)                 │
│  ┌──────────┐ ┌──────────┐ ┌───────────────────┐  │
│  │  Crypto   │ │  P2P     │ │  Gmail Bridge     │  │
│  │ Ed25519   │ │ libp2p   │ │  IMAP/SMTP        │  │
│  │ X25519    │ │ Kademlia │ │  Fallback Router  │  │
│  │ ChaCha20  │ │ mDNS     │ │  App Password     │  │
│  └──────────┘ └──────────┘ └───────────────────┘  │
│  ┌──────────┐ ┌──────────────────────────────────┐│
│  │  SQLite   │ │  REST API Endpoints              ││
│  │  Store    │ │  /api/identity|messages|peers|…   ││
│  └──────────┘ └──────────────────────────────────┘│
└────────────────────────────────────────────────────┘
         ┌───────────────┐  ┌───────────────┐
         │ Java Plugins   │  │ Python CLI    │
         │ SpamFilter     │  │ ledger_cli.py │
         │ AutoTagger     │  │ test_smoke.py │
         └───────────────┘  └───────────────┘
```

## Quick Start

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.70+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| .NET SDK | 8.0+ | `wget https://dot.net/v1/dotnet-install.sh && sh dotnet-install.sh --channel 8.0` |
| Java | 17+ | `sudo apt install openjdk-17-jdk` |
| Python | 3.10+ | Pre-installed on most systems |
| Build tools | — | `sudo apt install build-essential pkg-config libssl-dev` |

### Build & Run

**1. Rust Core (required first):**
```bash
cd ledger-core
cargo build --release
cargo run --release          # Starts API on 127.0.0.1:8420
# or with custom ports:
cargo run --release -- --api-port 8420 --p2p-port 9420
```

**2. C# Desktop UI:**
```bash
cd ledger-ui
dotnet build
dotnet run                   # Opens the desktop client
```

**3. Java Plugin Engine:**
```bash
cd ledger-plugins
mvn compile
mvn exec:java -Dexec.mainClass="com.ledger.plugins.PluginEngine"
```

**4. Python CLI:**
```bash
cd ledger-cli
python3 ledger_cli.py               # Interactive REPL
python3 test_smoke.py                # Run smoke tests
python3 ledger_cli.py --api http://127.0.0.1:8420
```

## API Reference

All endpoints are on `http://127.0.0.1:8420`.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/identity` | Your Ledger ID, public key, peer ID |
| GET | `/api/messages?folder=inbox` | List messages (inbox/sent/drafts) |
| POST | `/api/messages` | Send message `{to, subject, body, mode}` |
| DELETE | `/api/messages/{id}` | Delete a message |
| GET | `/api/peers` | List connected P2P peers |
| POST | `/api/peers` | Connect to peer `{multiaddr}` |
| GET | `/api/settings` | Get delivery mode, Tor toggle |
| PUT | `/api/settings` | Update settings |
| GET | `/api/gmail/config` | Gmail configuration status |
| POST | `/api/gmail/config` | Set Gmail credentials `{email, app_password}` |
| POST | `/api/gmail/fetch` | Fetch new Gmail messages |
| POST | `/api/gmail/send` | Send via Gmail `{to, subject, body}` |
| GET | `/api/contacts` | List contacts |
| POST | `/api/contacts` | Add contact `{ledger_id, public_key, ...}` |

## Delivery Modes

| Mode | Behavior |
|------|----------|
| `auto` (default) | Try P2P → DHT → Gmail fallback |
| `p2p_only` | P2P direct + DHT only, never use Gmail |
| `gmail_only` | Send everything through Gmail SMTP |

## Cryptography

- **Identity**: Ed25519 keypair (stored in `~/.ledger/identity.key`)
- **Key Exchange**: X25519 Diffie-Hellman with ephemeral keys
- **Encryption**: ChaCha20-Poly1305 (AEAD)
- **Key Derivation**: HKDF-SHA256
- **Ledger ID**: `ledger:` + first 32 chars of hex-encoded public key

## Project Structure

```
Ledger-Email-Client/
├── ledger-core/          # Rust — Core engine
│   └── src/
│       ├── main.rs       # Entry point + API server
│       ├── api/          # REST endpoints
│       ├── crypto/       # Ed25519, X25519, ChaCha20
│       ├── dht/          # Kademlia DHT storage
│       ├── fallback/     # P2P→DHT→Gmail routing
│       ├── gmail/        # IMAP/SMTP bridge
│       ├── models/       # Data structures
│       ├── p2p/          # libp2p swarm + protocols
│       └── store/        # SQLite persistence
├── ledger-ui/            # C# — Avalonia desktop UI
│   ├── Views/            # AXAML views
│   ├── ViewModels/       # MVVM ViewModels
│   ├── Services/         # API client
│   └── Styles/           # Dark theme
├── ledger-plugins/       # Java — Plugin engine
│   └── src/main/java/com/ledger/plugins/
│       ├── PluginEngine.java     # Plugin runner
│       ├── MessagePlugin.java    # Plugin interface
│       ├── LedgerApiClient.java  # API client
│       └── plugins/              # Built-in plugins
├── ledger-cli/           # Python — CLI toolkit
│   ├── ledger_cli.py     # Interactive REPL
│   ├── ledger_client.py  # API client
│   └── test_smoke.py     # Smoke tests
├── docs/
│   └── ARCHITECTURE.md   # Technical architecture docs
└── scripts/              # Platform setup scripts
```

## License

MIT
