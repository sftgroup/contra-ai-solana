//! Program state PDA — stored in a single account to hold all config

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Seeds for the program-derived state account
pub const STATE_SEED: &[u8] = b"contra_state";

/// Seeds for the treasury PDA (holds SPL token payments before forwarding)
pub const TREASURY_SEED: &[u8] = b"contra_treasury";

/// NFT mint counter seed (per token)
pub const MINT_COUNTER_SEED: &[u8] = b"contra_mint";

/// Timelock duration: 24 hours in seconds
pub const TIMELOCK_DURATION: i64 = 86400;

/// Main program state
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProgramState {
    /// Version for future upgrades
    pub version: u8,
    /// Program authority (owner)
    pub authority: Pubkey,
    /// Payment token mint (e.g., USDC SPL token)
    pub payment_mint: Pubkey,
    /// Mint price in token base units
    pub mint_price: u64,
    /// Maximum NFT supply
    pub max_supply: u64,
    /// Total NFTs minted so far
    pub total_minted: u64,
    /// Treasury address (receives payments)
    pub treasury: Pubkey,
    /// Final beneficiary address (receives forwarded funds)
    pub beneficiary: Pubkey,
    /// Base URI for token metadata
    pub base_uri: [u8; 128],
    pub base_uri_len: u8,
    /// Paused state
    pub paused: bool,
    /// Bump seed for PDA derivation
    pub bump: u8,

    // ───── Timelock: Owner Transfer ─────
    pub pending_owner: Pubkey,
    pub pending_owner_deadline: i64,

    // ───── Timelock: Max Supply ─────
    pub pending_max_supply: u64,
    pub pending_max_supply_deadline: i64,

    // ───── Timelock: Treasury ─────
    pub pending_treasury: Pubkey,
    pub pending_treasury_deadline: i64,

    // ───── Timelock: Beneficiary ─────
    pub pending_beneficiary: Pubkey,
    pub pending_beneficiary_deadline: i64,

    // ───── Timelock: Payment Mint ─────
    pub pending_payment_mint: Pubkey,
    pub pending_payment_mint_deadline: i64,

    // ───── Timelock: Mint Price ─────
    pub pending_mint_price: u64,
    pub pending_mint_price_deadline: i64,
}

impl ProgramState {
    /// Compute space required for serialized state
    pub const LEN: usize = 1      // version
        + 32    // authority
        + 32    // payment_mint
        + 8     // mint_price
        + 8     // max_supply
        + 8     // total_minted
        + 32    // treasury
        + 32    // beneficiary
        + 128   // base_uri
        + 1     // base_uri_len
        + 1     // paused
        + 1     // bump
        + 32    // pending_owner
        + 8     // pending_owner_deadline
        + 8     // pending_max_supply
        + 8     // pending_max_supply_deadline
        + 32    // pending_treasury
        + 8     // pending_treasury_deadline
        + 32    // pending_beneficiary
        + 8     // pending_beneficiary_deadline
        + 32    // pending_payment_mint
        + 8     // pending_payment_mint_deadline
        + 8     // pending_mint_price
        + 8;    // pending_mint_price_deadline

    /// Create a new program state
    pub fn new(
        authority: Pubkey,
        payment_mint: Pubkey,
        mint_price: u64,
        max_supply: u64,
        treasury: Pubkey,
        beneficiary: Pubkey,
        base_uri: String,
        bump: u8,
    ) -> Self {
        let mut base_uri_bytes = [0u8; 128];
        let uri_len = base_uri.len().min(128);
        base_uri_bytes[..uri_len].copy_from_slice(&base_uri.as_bytes()[..uri_len]);

        Self {
            version: 1,
            authority,
            payment_mint,
            mint_price,
            max_supply,
            total_minted: 0,
            treasury,
            beneficiary,
            base_uri: base_uri_bytes,
            base_uri_len: uri_len as u8,
            paused: false,
            bump,
            pending_owner: Pubkey::default(),
            pending_owner_deadline: 0,
            pending_max_supply: 0,
            pending_max_supply_deadline: 0,
            pending_treasury: Pubkey::default(),
            pending_treasury_deadline: 0,
            pending_beneficiary: Pubkey::default(),
            pending_beneficiary_deadline: 0,
            pending_payment_mint: Pubkey::default(),
            pending_payment_mint_deadline: 0,
            pending_mint_price: 0,
            pending_mint_price_deadline: 0,
        }
    }

    pub fn get_base_uri(&self) -> String {
        String::from_utf8_lossy(&self.base_uri[..self.base_uri_len as usize]).to_string()
    }
}
