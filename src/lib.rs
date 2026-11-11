//! Substreams module extracting transfer actions from Solana blocks.
//!
//! Handled instructions (top-level and inner/CPI instructions):
//! - System Program `Transfer` (lamports, native SOL) and `TransferWithSeed`
//! - SPL Token / Token-2022 `Transfer` and `TransferChecked` (raw token units)

mod pb;

use pb::transfers::v1::{Transfer, Transfers};
use substreams_solana::pb::sf::solana::r#type::v1::Block;

const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    data.get(offset..offset + 8)
        .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

#[substreams::handlers::map]
fn map_transfers(block: Block) -> Result<Transfers, substreams::errors::Error> {
    let mut transfers = Vec::new();

    // `walk_instructions` only visits successful transactions and resolves
    // account indexes (including address lookup tables) to addresses.
    for ix in block.walk_instructions() {
        let program = ix.program_id().to_string();
        let data: &[u8] = ix.data();
        let accounts = ix.accounts();

        // (from_index, to_index, amount)
        let parsed: Option<(usize, usize, u64)> = match program.as_str() {
            SYSTEM_PROGRAM if data.len() >= 12 => {
                match u32::from_le_bytes(data[0..4].try_into().unwrap()) {
                    2 => read_u64(data, 4).map(|a| (0, 1, a)),  // Transfer
                    11 => read_u64(data, 4).map(|a| (0, 2, a)), // TransferWithSeed
                    _ => None,
                }
            }
            TOKEN_PROGRAM | TOKEN_2022_PROGRAM if !data.is_empty() => match data[0] {
                3 => read_u64(data, 1).map(|a| (0, 1, a)),  // Transfer
                12 => read_u64(data, 1).map(|a| (0, 2, a)), // TransferChecked
                _ => None,
            },
            _ => None,
        };

        if let Some((from_idx, to_idx, amount)) = parsed {
            if let (Some(from), Some(to)) = (accounts.get(from_idx), accounts.get(to_idx)) {
                transfers.push(Transfer {
                    from: from.to_string(),
                    to: to.to_string(),
                    amount: amount as f64,
                });
            }
        }
    }

    Ok(Transfers { transfers })
}
