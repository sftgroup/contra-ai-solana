//! Instruction definitions — on-chain serialization via Borsh

use borsh::{BorshDeserialize, BorshSerialize};

/// All instructions for the Contra AI program
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum ContraInstruction {
    // ───── Initialization ─────
    /// Initialize the program state
    /// Accounts: [payer, state_pda, authority]
    Initialize {
        /// Payment token mint (USDC / SPL token used for mint payment)
        payment_mint: [u8; 32],
        /// Mint price in token base units
        mint_price: u64,
        /// Maximum supply of NFTs
        max_supply: u64,
        /// Base URI prefix for token metadata
        base_uri: String,
        /// Final beneficiary address (receives forwarded funds)
        beneficiary: [u8; 32],
    },

    // ───── Mint ─────
    /// Mint a Contra NFT
    /// Accounts: [payer, state_pda, nft_mint, nft_token, ...]
    Mint,

    // ───── Pause / Unpause ─────
    Pause,
    Unpause,

    // ───── 24h Timelock: Owner Transfer ─────
    InitiateOwnerTransfer { new_owner: [u8; 32] },
    CancelOwnerTransfer,
    AcceptOwnership,

    // ───── 24h Timelock: Max Supply ─────
    InitiateMaxSupplyChange { new_max: u64 },
    CancelMaxSupplyChange,
    ExecuteMaxSupplyChange,

    // ───── 24h Timelock: Treasury ─────
    InitiateTreasuryChange { new_treasury: [u8; 32] },
    CancelTreasuryChange,
    ExecuteTreasuryChange,

    // ───── 24h Timelock: Beneficiary ─────
    InitiateBeneficiaryChange { new_beneficiary: [u8; 32] },
    CancelBeneficiaryChange,
    ExecuteBeneficiaryChange,

    // ───── 24h Timelock: Payment Mint ─────
    InitiatePaymentMintChange { new_mint: [u8; 32] },
    CancelPaymentMintChange,
    ExecutePaymentMintChange,

    // ───── 24h Timelock: Mint Price ─────
    InitiateMintPriceChange { new_price: u64 },
    CancelMintPriceChange,
    ExecuteMintPriceChange,

    // ───── No-timelock (instant) ─────
    SetBaseUri { new_uri: String },
}
