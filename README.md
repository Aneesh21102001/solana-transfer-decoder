# Solana Transfer Decoder

Rust + Substreams project that extracts Solana transfer activity from mainnet blocks and exports the results as JSONL.

## What it does

This decoder processes blocks `385870151` through `385870156` and captures:
- native SOL transfers via the System Program
- SPL Token transfers
- Token-2022 transfers
- top-level and inner/CPI instructions
- account resolution through address lookup tables

The included run produced `3,974` transfer events in `transfers.jsonl`.

## Output format

Each line in `transfers.jsonl` is a single JSON object like this:

```json
{"from": "8ryouP8NbMv2EKL9V5xiAq5EEt5hKwnT9DDZHjqgTTWR", "to": "DduRAGKHQ5AN1BR3HGUW7ymL8MrTao7sQwZyoYCeSU69", "amount": 2829556537}
```

## Project files

- `src/lib.rs` - Rust map module that decodes transfers
- `src/pb.rs` - hand-written Prost bindings for the protobuf schema
- `proto/transfers.proto` - output schema
- `substreams.yaml` - Substreams manifest
- `run.ps1` - runs the decoder and flattens output to JSONL
- `transfers.jsonl` - output generated from the block range above
- `solana-transfers-v0.1.0.spkg` - packaged Substreams artifact

## How it works

`map_transfers` walks every instruction in each successful transaction and decodes:

| Program | Instruction | Accounts used | Amount source |
|---|---|---|---|
| System Program | `Transfer` | 0 -> 1 | lamports at bytes 4..12 |
| System Program | `TransferWithSeed` | 0 -> 2 | lamports at bytes 4..12 |
| SPL Token / Token-2022 | `Transfer` | 0 -> 1 | token amount at bytes 1..9 |
| SPL Token / Token-2022 | `TransferChecked` | 0 -> 2 | token amount at bytes 1..9 |

Addresses are written as base58 strings.

## Notes

- For SPL token transfers, `from` and `to` are token accounts, not wallet owners.
- `amount` is stored as `double` in the proto so JSON output keeps a bare integer instead of a quoted string.

## Build and run

Requirements:
- Rust with the `wasm32-unknown-unknown` target
- Substreams CLI

```powershell
cargo build --target wasm32-unknown-unknown --release
substreams pack substreams.yaml
$env:SUBSTREAMS_API_KEY = "<your key>"
.\run.ps1
```

To run directly:

```powershell
substreams run solana-transfers-v0.1.0.spkg map_transfers -e mainnet.sol.streamingfast.io:443 -s 385870151 -t 385870157 -o jsonl
```
