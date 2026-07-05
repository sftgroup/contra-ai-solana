//! PDA helper utilities

use solana_program::pubkey::Pubkey;

use crate::state::{MINT_COUNTER_SEED, STATE_SEED};

/// Derive the program state PDA
pub fn find_state_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STATE_SEED], program_id)
}

/// Derive the NFT mint counter PDA for a given token index
pub fn find_mint_counter_pda(program_id: &Pubkey, token_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MINT_COUNTER_SEED, &token_id.to_le_bytes()],
        program_id,
    )
}
