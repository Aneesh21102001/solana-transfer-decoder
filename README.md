# Solana Transfer Extraction — Substreams

Extracts transfers from Solana blocks **385870151–385870156** and outputs one JSON object per transfer:

```json
{"from": "8ryouP8NbMv2EKL9V5xiAq5EEt5hKwnT9DDZHjqgTTWR", "to": "DduRAGKHQ5AN1BR3HGUW7ymL8MrTao7sQwZyoYCeSU69", "amount": 2829556537}
```

Result of the run: **3,974 transfers**, see `transfers.jsonl`.

## Files
| File | Purpose |
|---|---|
| `solana-transfers-v0.1.0.spkg` | The built Substreams package |
| `substreams.yaml` | Manifest (module `map_transfers`, input `sf.solana.type.v1.Block`) |
| `proto/transfers.proto` | Output schema (`Transfers { repeated Transfer }`) |
| `src/lib.rs` | Rust map module |
| `src/pb.rs` | prost bindings for the proto (hand-written, so no `protoc` needed) |
| `run.ps1` | Streams the block range and flattens output to one JSON line per transfer |
| `transfers.jsonl` | Output from the run |

## How it works
`map_transfers` walks every instruction (top-level **and** inner/CPI) of every successful transaction, with account indexes resolved (including address lookup tables), and decodes:

| Program | Instruction | from / to accounts | amount |
|---|---|---|---|
| System (`1111…1111`) | `Transfer` (2) | 0 → 1 | lamports, u64 LE at bytes 4..12 |
| System | `TransferWithSeed` (11) | 0 → 2 | lamports |
| SPL Token / Token-2022 | `Transfer` (3) | 0 → 1 | raw units, u64 LE at bytes 1..9 |
| SPL Token / Token-2022 | `TransferChecked` (12) | 0 → 2 | raw units |

Addresses are base58-encoded.

## Design notes
- **`amount` is a `double` in the proto.** ProtoJSON renders `uint64` as a quoted string (`"325253"`), but the required format is a bare integer. A double renders as `325253` and is exact up to 2^53.
- For SPL token transfers, `from`/`to` are **token accounts** (what the instruction references), not owner wallets.
- Uses `substreams` 0.6 / `substreams-solana` 0.14 / `prost` 0.13. The newer 0.8/0.17 crates switched to a new protobuf stack (`buffa`).

## Build & run (Windows)
Prerequisites: Rust with the `wasm32-unknown-unknown` target, and the Substreams CLI. There's no Windows release binary, so the CLI was built from source with Go: `go build ./cmd/substreams` in the substreams repo.

```powershell
cargo build --target wasm32-unknown-unknown --release
substreams pack substreams.yaml
$env:SUBSTREAMS_API_KEY = "<key from https://thegraph.market>"
.\run.ps1
```

Or run it directly. The stop block is exclusive:
```
substreams run solana-transfers-v0.1.0.spkg map_transfers -e mainnet.sol.streamingfast.io:443 -s 385870151 -t 385870157 -o jsonl
```
