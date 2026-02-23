# Ledger — Architecture Document

## System Overview

Ledger is a hybrid decentralized encrypted mail client. It combines peer-to-peer encrypted messaging with a Gmail IMAP/SMTP bridge, providing seamless encrypted communication between Ledger users and standard email interop with Gmail users.

## System Boundaries

```
┌─────────────────────────────────────────────────────────┐
│                    Ledger System                         │
│                                                          │
│  ┌──────────────┐   REST API    ┌──────────────────┐   │
│  │  C# Avalonia  │◄────────────►│   Rust Core       │   │
│  │  Desktop UI   │  :8420       │   (ledger-core)   │   │
│  └──────────────┘               │                    │   │
│                                  │  ┌──────────────┐ │   │
│  ┌──────────────┐               │  │  Crypto       │ │   │
│  │  Java Plugin  │◄─────────────│  │  Ed25519      │ │   │
│  │  Engine       │  REST API    │  │  X25519       │ │   │
│  └──────────────┘               │  │  ChaCha20     │ │   │
│                                  │  └──────────────┘ │   │
│  ┌──────────────┐               │                    │   │
│  │  Python CLI   │◄─────────────│  ┌──────────────┐ │   │
│  │  Tools        │  REST API    │  │  libp2p       │ │   │
│  └──────────────┘               │  │  Gossipsub    │ │   │
│                                  │  │  Kademlia DHT │ │   │
│                                  │  └──────────────┘ │   │
│                                  │                    │   │
│                                  │  ┌──────────────┐ │   │
│                                  │  │  Gmail Bridge │ │   │
│                                  │  │  IMAP/SMTP   │ │   │
│                                  │  └──────────────┘ │   │
│                                  │                    │   │
│                                  │  ┌──────────────┐ │   │
│                                  │  │  SQLite Store │ │   │
│                                  │  └──────────────┘ │   │
│                                  └──────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## IPC Protocol

**Protocol:** HTTP REST (JSON)
**Bind Address:** `127.0.0.1:8420` (configurable via `--port`)
**Rationale:** REST avoids protobuf codegen across Rust/C#/Java/Python. Curl-debuggable. All clients use standard HTTP libraries.

### API Endpoints

| Endpoint | Method | Description |
|---|---|---|
| `/api/identity` | GET | Get Ledger ID and public key |
| `/api/messages` | GET | List all messages (query: folder, unread) |
| `/api/messages` | POST | Send a message |
| `/api/messages/{id}` | GET | Get single message |
| `/api/messages/{id}` | DELETE | Delete message |
| `/api/peers` | GET | List connected peers |
| `/api/peers` | POST | Connect to a peer by multiaddr |
| `/api/gmail/config` | GET | Get Gmail config status |
| `/api/gmail/config` | POST | Set Gmail credentials |
| `/api/gmail/fetch` | POST | Fetch new Gmail messages |
| `/api/gmail/send` | POST | Send via Gmail SMTP |
| `/api/settings` | GET | Get all settings |
| `/api/settings` | PUT | Update settings |

## Ports

| Port | Protocol | Purpose |
|---|---|---|
| 8420 | HTTP | REST API (localhost only) |
| 9420 | TCP+Noise | libp2p swarm (public) |

## Data Flows

### 1. P2P Send (Ledger ↔ Ledger)

```
1. User composes message in UI
2. UI POSTs to /api/messages {to: "ledger:<id>", subject, body}
3. Rust core:
   a. Looks up recipient public key (contacts DB or DHT)
   b. Performs X25519 DH key agreement
   c. Encrypts body with ChaCha20-Poly1305
   d. Signs envelope with sender's Ed25519 key
   e. Sends via libp2p request-response protocol
4. Recipient node:
   a. Receives encrypted envelope
   b. Verifies Ed25519 signature
   c. Decrypts with own X25519 private key
   d. Stores plaintext in SQLite (folder: inbox)
5. Recipient UI polls /api/messages → sees new message
```

### 2. Gmail Send (Ledger → Gmail user)

```
1. User composes message to regular email address
2. UI POSTs to /api/messages {to: "user@gmail.com", subject, body}
3. Rust core:
   a. Detects non-Ledger recipient
   b. Composes standard MIME message
   c. Sends via SMTP (lettre) to smtp.gmail.com:587
   d. Auth: user's App Password
   e. Stores copy in SQLite (folder: sent, method: gmail)
```

### 3. Gmail Receive (Gmail user → Ledger)

```
1. Rust core periodically polls IMAP (or uses IDLE)
2. Fetches new messages from imap.gmail.com:993
3. Stores in SQLite (folder: inbox, method: gmail)
4. UI polls /api/messages → shows with Gmail badge
```

### 4. Fallback (P2P fails → Gmail encrypted relay)

```
1. User sends to Ledger ID, mode = "auto"
2. Rust core tries P2P delivery → peer unreachable
3. Rust core tries DHT storage → stores for later retrieval
4. Rust core also sends via Gmail:
   a. Encrypts full message payload
   b. Base64-encodes as attachment
   c. Subject: "[Ledger Encrypted Fallback]"
   d. Sends via SMTP to recipient's registered email
5. Recipient's Ledger client:
   a. Fetches Gmail via IMAP
   b. Detects "[Ledger Encrypted Fallback]" prefix
   c. Extracts and decrypts attachment
   d. Stores as normal message (method: fallback)
```

## Cryptographic Design

| Operation | Algorithm | Library |
|---|---|---|
| Identity signing | Ed25519 | `ed25519-dalek` |
| Key exchange | X25519 Diffie-Hellman | `x25519-dalek` |
| Symmetric encryption | ChaCha20-Poly1305 | `chacha20poly1305` |
| Key derivation | HKDF-SHA256 | `hkdf` |
| Hashing | SHA-256 | `sha2` |

### Message Envelope Format

```json
{
  "id": "uuid-v4",
  "from": "ledger:<sender_pubkey_base58>",
  "to": "ledger:<recipient_pubkey_base58>",
  "timestamp": 1707700000,
  "ephemeral_pubkey": "<base64>",
  "encrypted_body": "<base64>",
  "signature": "<base64>",
  "nonce": "<base64>"
}
```

## Delivery Mode Settings

| Mode | Behavior |
|---|---|
| `p2p_only` | Only send via P2P. Fail if peer unreachable. |
| `gmail_only` | Only send via Gmail SMTP. |
| `auto` (default) | Try P2P → DHT → Gmail fallback |

## Technology Stack

| Component | Technology | Version |
|---|---|---|
| Core Engine | Rust | 1.75+ |
| P2P Networking | libp2p | 0.53+ |
| Desktop UI | C# / .NET 8 / Avalonia | 11 |
| Plugin System | Java / Maven | 21 |
| CLI Tools | Python | 3.10+ |
| Local Storage | SQLite | 3.x |
| Transport Security | Noise Protocol | via libp2p |
