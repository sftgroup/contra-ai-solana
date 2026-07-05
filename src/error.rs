//! Error types for Contra AI

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ContraError {
    /// Owner-only operation
    #[error("Only the owner can perform this action")]
    Unauthorized,

    /// NFT sale is paused
    #[error("Minting is paused")]
    Paused,

    /// All tokens have been minted
    #[error("Max supply reached — sold out")]
    SoldOut,

    /// Invalid payment token account
    #[error("Invalid payment token account")]
    InvalidPaymentToken,

    /// Invalid treasury token account
    #[error("Invalid treasury token account")]
    InvalidTreasury,

    /// Invalid beneficiary token account
    #[error("Invalid beneficiary token account")]
    InvalidBeneficiary,

    /// Arithmetic overflow
    #[error("Arithmetic overflow")]
    Overflow,

    /// Timelock has not expired yet
    #[error("Timelock has not expired")]
    TimelockNotExpired,

    /// No pending timelock change
    #[error("No pending change")]
    NoPendingChange,

    /// Not the pending owner
    #[error("Not the pending owner")]
    NotPendingOwner,

    /// Invalid max supply (below total minted)
    #[error("Max supply must be >= total minted")]
    InvalidMaxSupply,

    /// Invalid mint price (zero)
    #[error("Mint price must be non-zero")]
    InvalidMintPrice,

    /// Invalid treasury address
    #[error("Invalid treasury address")]
    InvalidTreasuryAddress,

    /// Invalid beneficiary address
    #[error("Invalid beneficiary address")]
    InvalidBeneficiaryAddress,

    /// Arithmetic underflow
    #[error("Arithmetic underflow")]
    Underflow,
}

impl From<ContraError> for ProgramError {
    fn from(e: ContraError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
