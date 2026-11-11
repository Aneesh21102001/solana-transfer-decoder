# Submission — Substreams Assignment: Solana Transfer Extraction

## Summary

| Success criterion | Status | Evidence |
|---|---|---|
| 1. Build a valid SPKG file | ✅ Done | `solana-transfers-v0.1.0.spkg` (334 KB). `substreams pack` and `substreams info` succeed |
| 2. Run against blocks 385870151–385870156 | ✅ Done | Streamed from `mainnet.sol.streamingfast.io:443`, start 385870151, stop 385870157 (exclusive) |
| 3. Output in the required format | ✅ Done | `transfers.jsonl`: **3,974 transfers**. All 3,974 lines pass a strict format check |

Sample output (first lines of `transfers.jsonl`):
```json
{"from": "8ryouP8NbMv2EKL9V5xiAq5EEt5hKwnT9DDZHjqgTTWR", "to": "DduRAGKHQ5AN1BR3HGUW7ymL8MrTao7sQwZyoYCeSU69", "amount": 2829556537}
{"from": "DduRAGKHQ5AN1BR3HGUW7ymL8MrTao7sQwZyoYCeSU69", "to": "GxQjn5aZBuwjtwHnDYWmJNdWniZq84SjtRQjv7VohuMM", "amount": 2829556537}
{"from": "81K49VTk8souN8HVfFvcyHVPrEyHs5ztqusV9B8ZxN3p", "to": "73GKo6QFZWR32m2JqzPoYSYaeptTiV2UGWjnhbLvymbw", "amount": 2655512236891}
```
One JSON object per transfer. A block with N transfers yields N lines. `amount` is a bare integer, not a string.

**Total time:** about 45 minutes, from first reading the task to a verified run.
**AI tool used:** Google **Antigravity**, an agentic AI coding assistant that runs commands in my terminal and edits files.

---

## 1. Making sense of the assignment

Before writing any code, I broke the problem into concepts:

- **Solana:** a blockchain. Data comes in *blocks* (slots). Each block holds *transactions*, and each transaction holds *instructions*. An instruction is a call to a program with a list of account addresses and raw bytes.
- **A "transfer"** is a specific instruction:
  - **Native SOL:** System Program (`11111111111111111111111111111111`) instruction `Transfer`.
  - **Tokens:** SPL Token (`TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`) and Token-2022 (`TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`) instructions `Transfer` / `TransferChecked`.
  - Transfers can also happen **inside** other programs (CPI / inner instructions, e.g. a DEX swap moving tokens), so these must be included.
- **Substreams:** a StreamingFast/The Graph technology. You write a Rust function (a *map module*) that receives one block and returns a protobuf message. It compiles to **WebAssembly** and is bundled with a `substreams.yaml` manifest and `.proto` schemas into an **`.spkg` package**. A provider's servers run the package against blockchain data.
- **What I had to produce:** a Rust map module `Block → Transfers`, a proto schema with `from`, `to`, `amount`, a manifest, the `.spkg`, and a run over 6 blocks.

## 2. Step-by-step execution

### Step 1 — Install Rust
- Installed **rustup** with the **`x86_64-pc-windows-gnu`** host toolchain and the **`wasm32-unknown-unknown`** target:
  ```
  rustup-init.exe -y --default-host x86_64-pc-windows-gnu --profile minimal --target wasm32-unknown-unknown
  ```
- **Why GNU instead of the default MSVC toolchain:** MSVC needs Visual Studio Build Tools, a multi-GB install, for its linker. The GNU toolchain ships its own linker, which is enough for build scripts and proc macros. The final `.wasm` needs no system linker at all.

### Step 2 — Install the Substreams CLI
- The docs point to GitHub releases. **The latest release (v1.26.0) ships only Linux and macOS binaries, with no Windows build.**
- Options I considered: install WSL/Ubuntu, which needs admin rights and a reboot, or **build from source**. Go 1.25 was already installed, so I built from source:
  ```
  git clone --depth 1 --branch v1.26.0 https://github.com/streamingfast/substreams.git
  go build -o %USERPROFILE%\.cargo\bin\substreams.exe ./cmd/substreams
  ```
- Result: a native Windows `substreams.exe`. No VM or WSL needed.

### Step 3 — Get a free API key
- Created a free account at **https://thegraph.market** (The Graph Market / StreamingFast) and generated an API key.
- Passed it through the `SUBSTREAMS_API_KEY` environment variable. It's never written to any file. `run.ps1` prompts for it with hidden input if it isn't set.

### Step 4 — Develop the Substreams module

**Project layout**
```
solana-transfers/
├── Cargo.toml            # Rust crate (cdylib → .wasm)
├── substreams.yaml       # manifest
├── proto/transfers.proto # output schema
├── src/lib.rs            # map module (transfer decoding)
├── src/pb.rs             # prost structs matching the proto
└── run.ps1 / RUN.bat     # run + flatten output
```

**Manifest (`substreams.yaml`):** one `map` module, `map_transfers`. Input is the raw Solana block `sf.solana.type.v1.Block`. Output is `proto:transfers.v1.Transfers`. `initialBlock: 385870151`, `network: solana`.

**Protobuf schema**
```proto
message Transfers { repeated Transfer transfers = 1; }   // many transfers per block
message Transfer  { string from = 1; string to = 2; double amount = 3; }
```

**Rust logic (`src/lib.rs`):**
1. `block.walk_instructions()` from `substreams-solana` visits every instruction of every **successful** transaction, both top-level and inner/CPI. Failed transactions move no funds, so they're skipped. Account indexes are already resolved to addresses, including versioned-transaction **address lookup tables**.
2. Each instruction is decoded by program ID and data bytes (all integers little-endian):

| Program | Instruction | Discriminator | Amount bytes | from → to accounts |
|---|---|---|---|---|
| System | `Transfer` | u32 = 2 | u64 at [4..12] (lamports) | 0 → 1 |
| System | `TransferWithSeed` | u32 = 11 | u64 at [4..12] | 0 → 2 |
| SPL Token / Token-2022 | `Transfer` | u8 = 3 | u64 at [1..9] (raw units) | 0 → 1 |
| SPL Token / Token-2022 | `TransferChecked` | u8 = 12 | u64 at [1..9] | 0 → 2 (account 1 is the mint) |

3. Addresses are base58-encoded. Malformed or short instructions are skipped safely: bounds-checked reads, no panics.

### Step 5 — Build the SPKG
```
cargo build --target wasm32-unknown-unknown --release   # → solana_transfers.wasm (168 KB)
substreams pack substreams.yaml                         # → solana-transfers-v0.1.0.spkg (334 KB)
substreams info solana-transfers-v0.1.0.spkg            # validates module, input and output types
```

### Step 6 — Run and verify
```
substreams run solana-transfers-v0.1.0.spkg map_transfers ^
  -e mainnet.sol.streamingfast.io:443 -s 385870151 -t 385870157 -o jsonl
```
- The stop block is **exclusive**, so `-t 385870157` covers 385870151–385870156 inclusive.
- `run.ps1` flattens the per-block output (`{"@block":…, "@data":{"transfers":[…]}}`) into **one JSON line per transfer** in the exact required spacing, and saves it to `transfers.jsonl`.
- **Verification:**
  - All 3,974 lines match the regex `^\{"from": "<base58 32-44>", "to": "<base58 32-44>", "amount": \d+\}$`.
  - 2,115 unique senders.
  - The largest amount is about 4.6×10¹⁵, below 2⁵³, so every amount is exact (see the `double` note below).

---

## 3. Challenges and how I solved them

| # | Problem | How I diagnosed it | Fix |
|---|---|---|---|
| 1 | Rust, Substreams CLI and WSL all missing on Windows | Checked each tool's version | Installed the Rust GNU toolchain, avoiding Visual Studio |
| 2 | **No Windows binary** for the Substreams CLI | Listed the release assets through the GitHub API | Built the CLI from source with Go |
| 3 | The newest crates (`substreams` 0.8 / `substreams-solana` 0.17) moved from `prost` to a brand-new protobuf library, **`buffa`** | Read the crates' source in the cargo registry | Pinned the established prost-based family: `substreams` 0.6.4, `substreams-solana` 0.14.3, `prost` 0.13. A brand-new library under time pressure was too risky |
| 4 | Build error: *"stream did not contain valid UTF-8"* | PowerShell's `Out-File` had written a placeholder `lib.rs` as UTF-16 | Re-saved the file as UTF-8 without BOM |
| 5 | **`amount` came out as a string.** Proto3 JSON (ProtoJSON) renders `uint64` as `"325253"`, but the spec needs a bare integer `325253` | Read the Substreams CLI source: output uses Go `protojson` | Declared `amount` as `double`, which ProtoJSON prints as `325253`. Exact up to 2⁵³ (about 9×10¹⁵), and all 3,974 amounts here are below that |
| 6 | Avoid needing `protoc`/`buf` for code generation | — | Hand-wrote the small prost structs in `src/pb.rs` to mirror `transfers.proto`. The `.proto` is still packaged in the SPKG for consumers |

## 4. How I used AI (Antigravity)

- **Understanding:** had it explain Solana's block/transaction/instruction model, Substreams modules and the SPKG format, and how instruction data is laid out.
- **Execution:** it ran installs, builds and the run in my terminal, so I could iterate quickly. When something failed, we read the actual error or source code instead of guessing. Examples: inspecting the GitHub release assets, the `substreams-solana` crate API, and how the CLI encodes JSON.
- **Judgment calls I made with it:** the GNU toolchain over MSVC, building the CLI from source over installing WSL, pinning stable crates over the newest ones, and `double` for exact JSON integers.
- **Cross-checking:** I also asked ChatGPT for a solution and compared the two. ChatGPT's had never been built, depended on WSL/Ubuntu, and contained a compile error: `type` used unescaped in a Rust path, where it must be `r#type`. I kept the version I had actually built and verified.

## 5. Assumptions and open questions

- **The example transfer** (`Eq1c3uNu… → 4XpSNfo…, 325253`) does **not** appear in my output for these blocks. `Eq1c3uNu…` does appear, but only as a recipient in a different transfer. My reading is that the example shows the *format* and isn't necessarily from this range. Other possible interpretations:
  - For SPL tokens, `from`/`to` could mean the **owner wallets** instead of the **token accounts** named in the instruction. My module reports the token accounts, i.e. the accounts the instruction actually debits and credits.
  - The example may count other instruction types (mint, burn, or other programs).

  Both are small changes: the owner wallet for `from` is the instruction's authority account, and the destination owner is available from the transaction's token-balance metadata.
- **Units:** SOL amounts are in **lamports** (1 SOL = 10⁹ lamports). Token amounts are in **raw base units** (not divided by decimals), which matches the integer amount in the spec.
- **Scope:** successful transactions only, since failed ones move no funds. Vote transactions contain no transfer instructions, so they produce nothing.

## 6. How to reproduce
```powershell
# Build (needs Rust + wasm32 target + substreams CLI)
cargo build --target wasm32-unknown-unknown --release
substreams pack substreams.yaml

# Run (prompts for the API key, writes transfers.jsonl)
.\run.ps1
```
