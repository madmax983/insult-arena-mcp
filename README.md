# Insult Arena MCP

[![CI](https://github.com/madmax983/insult-arena-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/madmax983/insult-arena-mcp/actions/workflows/ci.yml)

LLM vs LLM Monkey Island-style insult sword fighting over [MCP](https://modelcontextprotocol.io).

> "You fight like a dairy farmer!"
> "How appropriate. You fight like a cow."

## What is this?

An MCP server that lets AI models engage in the classic insult sword fighting mini-game from **The Secret of Monkey Island**. Connect multiple Claude instances (or any MCP-compatible AI) and watch them duel with wit!

## Features

- 16 classic Monkey Island insults and comebacks
- Turn-based dueling (first to 3 exchange wins)
- SSE transport for multi-client support
- Works with Claude Desktop, Claude Code, and other MCP clients

## Quick Start

### 1. Start the Arena Server

```bash
cargo run
```

You'll see:
```
⚔️  INSULT SWORD FIGHTING ARENA
================================
📡 Starting SSE server on http://localhost:3000
🔗 Connect clients to: http://localhost:3000/sse
🎭 Waiting for challengers...
🚀 Arena is LIVE!
```

### 2. Connect Your AI

#### Claude Code

Add to your `.mcp.json`:

```json
{
  "mcpServers": {
    "insult-arena": {
      "command": "cmd",
      "args": ["/c", "npx", "-y", "mcp-remote", "http://localhost:3000/sse"]
    }
  }
}
```

(On Mac/Linux, use `"command": "npx"` and `"args": ["-y", "mcp-remote", "http://localhost:3000/sse"]`)

#### Claude Desktop

Add to your Claude Desktop config:

```json
{
  "mcpServers": {
    "insult-arena": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "http://localhost:3000/sse"]
    }
  }
}
```

### 3. Duel!

Tell Claude: *"Start an insult sword fighting duel and play as the Challenger!"*

## How It Works

The duel follows classic Monkey Island rules:

1. **Challenger** throws an insult
2. **Defender** must respond with the correct witty comeback
3. If the comeback matches → Defender parries and becomes the attacker
4. If the comeback fails → Attacker scores a point and attacks again
5. First to 3 points wins!

## Available Tools

| Tool | Description |
|------|-------------|
| `start_duel` | Start a new duel |
| `register_as_challenger` | Register as the Challenger (attacks first) |
| `register_as_defender` | Register as the Defender (responds to insults) |
| `get_duel_state` | Check current scores and whose turn it is |
| `list_insults` | See all 16 classic insults |
| `throw_insult` | Attack with an insult (must be from the list) |
| `respond` | Counter with a comeback |
| `get_hint` | Get a hint for the current insult |

## Turn Notifications

The server broadcasts `notifications/turn` to all connected clients after each action. This enables autonomous LLM-vs-LLM dueling - each AI receives a notification when it's their turn to act!

## Example Duel

```
Challenger vs Defender - First to 3 wins!

🗣️  INSULT: "You fight like a dairy farmer!"
💬 COMEBACK: "How appropriate. You fight like a cow."
   ✨ PARRIED! Defender wins the exchange!

🗣️  INSULT: "Every enemy I've met I've annihilated!"
💬 COMEBACK: "With your breath, I'm sure they all suffocated."
   ✨ PARRIED! Defender wins the exchange!

🗣️  INSULT: "Nobody's ever drawn blood from me and nobody ever will!"
💬 COMEBACK: "You run THAT fast?"
   ✨ PARRIED! Defender wins the exchange!

🏆 DUEL OVER! Defender WINS! (Score: 0-3)
```

## Building

```bash
cargo build --release
```

## Credits

Insults from **The Secret of Monkey Island** (1990) by Lucasfilm Games.

Built with [rust-mcp-sdk](https://github.com/rust-mcp-stack/rust-mcp-sdk).

## License

MIT
