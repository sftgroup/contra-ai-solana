//! Core instruction processor for Contra AI on Solana
//!
//! Architecture:
//!   - Single ProgramState PDA holds all configuration
//!   - Mint creates NFT via SPL Token (Metaplex not required — lightweight)
//!   - Payment: SPL token (USDC) → Treasury → forward to Beneficiary
//!   - 24h timelocks: owner, maxSupply, treasury, beneficiary
//!   - Instant: paymentMint, mintPrice, baseUri, pause/unpause

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_token::instruction::{mint_to, transfer};
// use spl_token::state::Pack; // private in v7

use crate::error::ContraError;
use crate::instruction::ContraInstruction;
use crate::pda::{find_mint_counter_pda, find_state_pda, find_treasury_pda};
use crate::state::{ProgramState, TIMELOCK_DURATION, STATE_SEED, TREASURY_SEED};

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = ContraInstruction::try_from_slice(instruction_data)?;
        match instruction {
            ContraInstruction::Initialize { payment_mint, mint_price, max_supply, base_uri, beneficiary } => {
                Self::process_initialize(program_id, accounts, payment_mint, mint_price, max_supply, base_uri, beneficiary)
            }
            ContraInstruction::Mint => {
                Self::process_mint(program_id, accounts)
            }
            ContraInstruction::Pause => Self::process_set_pause(program_id, accounts, true),
            ContraInstruction::Unpause => Self::process_set_pause(program_id, accounts, false),

            ContraInstruction::InitiateOwnerTransfer { new_owner } => {
                Self::process_initiate_owner_transfer(program_id, accounts, &Pubkey::new_from_array(new_owner))
            }
            ContraInstruction::CancelOwnerTransfer => {
                Self::process_cancel_owner_transfer(program_id, accounts)
            }
            ContraInstruction::AcceptOwnership => {
                Self::process_accept_ownership(program_id, accounts)
            }

            ContraInstruction::InitiateMaxSupplyChange { new_max } => {
                Self::process_initiate_max_supply_change(program_id, accounts, new_max)
            }
            ContraInstruction::CancelMaxSupplyChange => {
                Self::process_cancel_max_supply_change(program_id, accounts)
            }
            ContraInstruction::ExecuteMaxSupplyChange => {
                Self::process_execute_max_supply_change(program_id, accounts)
            }

            ContraInstruction::InitiateTreasuryChange { new_treasury } => {
                Self::process_initiate_treasury_change(program_id, accounts, &Pubkey::new_from_array(new_treasury))
            }
            ContraInstruction::CancelTreasuryChange => {
                Self::process_cancel_treasury_change(program_id, accounts)
            }
            ContraInstruction::ExecuteTreasuryChange => {
                Self::process_execute_treasury_change(program_id, accounts)
            }

            ContraInstruction::InitiateBeneficiaryChange { new_beneficiary } => {
                Self::process_initiate_beneficiary_change(program_id, accounts, &Pubkey::new_from_array(new_beneficiary))
            }
            ContraInstruction::CancelBeneficiaryChange => {
                Self::process_cancel_beneficiary_change(program_id, accounts)
            }
            ContraInstruction::ExecuteBeneficiaryChange => {
                Self::process_execute_beneficiary_change(program_id, accounts)
            }

            ContraInstruction::SetPaymentMint { new_mint } => {
                Self::process_set_payment_mint(program_id, accounts, &Pubkey::new_from_array(new_mint))
            }
            ContraInstruction::SetMintPrice { new_price } => {
                Self::process_set_mint_price(program_id, accounts, new_price)
            }
            ContraInstruction::SetBaseUri { new_uri } => {
                Self::process_set_base_uri(program_id, accounts, new_uri)
            }
        }
    }

    // ═══════════════════════════════════════════
    // PDA helpers
    // ═══════════════════════════════════════════

    fn get_state_pda_signer(program_id: &Pubkey) -> (Pubkey, u8) {
        find_state_pda(program_id)
    }

    fn deserialize_state<'a>(state_info: &AccountInfo<'a>) -> Result<ProgramState, ProgramError> {
        let data = state_info.try_borrow_data()?;
        ProgramState::try_from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)
    }

    fn serialize_state(state_info: &AccountInfo, state: &ProgramState) -> ProgramResult {
        let mut data = state_info.try_borrow_mut_data()?;
        state.serialize(&mut data.as_mut())?;
        Ok(())
    }

    fn check_authority(state: &ProgramState, signer: &Pubkey) -> ProgramResult {
        if state.authority != *signer {
            msg!("Unauthorized: signer={} authority={}", signer, state.authority);
            return Err(ContraError::Unauthorized.into());
        }
        Ok(())
    }

    fn get_clock(clock_info: &AccountInfo) -> Result<Clock, ProgramError> {
        Clock::from_account_info(clock_info)
            .or_else(|_| {
                // Fallback for environments where sysvar is passed as account
                Clock::get()
            })
    }

    // ═══════════════════════════════════════════
    // Initialize
    // ═══════════════════════════════════════════
    // Accounts:
    //   0. [signer, writable] Payer (becomes authority)
    //   1. [writable]        State PDA
    //   2. []                System program
    //   3. []                SPL Token program
    //   4. []                Rent sysvar

    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        payment_mint: [u8; 32],
        mint_price: u64,
        max_supply: u64,
        base_uri: String,
        beneficiary: [u8; 32],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;
        let _token_program = next_account_info(accounts_iter)?;
        let _rent_info = next_account_info(accounts_iter)?;

        // Verify state PDA
        let (state_pda, bump) = find_state_pda(program_id);
        if state_pda != *state_info.key {
            msg!("Invalid state PDA: expected={} got={}", state_pda, state_info.key);
            return Err(ProgramError::InvalidSeeds);
        }

        // Check if already initialized (P1: prevent re-initialization attack)
        if state_info.data_len() > 0 {
            // Try to deserialize — if it succeeds with version>0, already initialized
            if let Ok(existing) = Self::deserialize_state(state_info) {
                if existing.version > 0 {
                    msg!("Already initialized: version={} authority={}", existing.version, existing.authority);
                    return Err(ProgramError::AccountAlreadyInitialized);
                }
            }
        }

        // Validate inputs
        if mint_price == 0 {
            return Err(ContraError::InvalidMintPrice.into());
        }
        if max_supply == 0 {
            return Err(ContraError::InvalidMaxSupply.into());
        }

        let payment_mint_key = Pubkey::new_from_array(payment_mint);

        // Treasury = PDA (contra_treasury seed), beneficiary = provided at init
        let (treasury_pda, _) = find_treasury_pda(program_id);
        let beneficiary_key = Pubkey::new_from_array(beneficiary);
        if beneficiary_key == Pubkey::default() {
            msg!("Invalid beneficiary: zero address");
            return Err(ContraError::InvalidBeneficiaryAddress.into());
        }

        let state = ProgramState::new(
            *payer.key,
            payment_mint_key,
            mint_price,
            max_supply,
            treasury_pda,      // treasury: program PDA
            beneficiary_key,   // beneficiary: explicit from deployer
            base_uri,
            bump,
        );

        // Create PDA account
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(ProgramState::LEN);

        let seeds: &[&[u8]] = &[STATE_SEED, &[bump]];
        let signer_seeds = &[&seeds[..]];

        invoke_signed(
            &solana_program::system_instruction::create_account(
                payer.key,
                state_info.key,
                required_lamports,
                ProgramState::LEN as u64,
                program_id,
            ),
            &[payer.clone(), state_info.clone(), system_program.clone()],
            signer_seeds,
        )?;

        Self::serialize_state(state_info, &state)?;

        msg!(
            "Contra AI initialized: authority={} payment_mint={} price={} maxSupply={}",
            payer.key, payment_mint_key, mint_price, max_supply
        );
        Ok(())
    }

    // ═══════════════════════════════════════════
    // Mint
    // ═══════════════════════════════════════════
    // Accounts:
    //   0. [signer, writable] Payer (minter)
    //   1. [writable]        State PDA
    //   2. [writable]        Payer's payment token account (source)
    //   3. [writable]        Treasury PDA's payment ATA (destination, signed by program)
    //   4. [writable]        Beneficiary's payment token account (forward dest)
    //   5. [writable]        NFT mint account (new, PDA)
    //   6. [writable]        Payer's NFT token account (ATA)
    //   7. []                Token Program
    //   8. []                Associated Token Program
    //   9. []                System Program
    //  10. []                Rent sysvar
    //  11. []                Clock sysvar

    fn process_mint(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;
        let payer_token = next_account_info(accounts_iter)?;
        let treasury_token = next_account_info(accounts_iter)?;
        let beneficiary_token = next_account_info(accounts_iter)?;
        let nft_mint = next_account_info(accounts_iter)?;
        let nft_token = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let ata_program = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;
        let _rent_info = next_account_info(accounts_iter)?;
        let _clock_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;

        // ── P0 Account Validation (Security Audit Fix) ──

        // 1. Verify state PDA
        let (expected_state_pda, _) = find_state_pda(program_id);
        if *state_info.key != expected_state_pda {
            msg!("Invalid state PDA: expected={} got={}", expected_state_pda, state_info.key);
            return Err(ProgramError::InvalidSeeds);
        }

        // 2. Verify token_program
        if *token_program.key != spl_token::id() {
            msg!("Invalid token program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // 3. Verify associated token program
        if *ata_program.key != spl_associated_token_account::id() {
            msg!("Invalid ATA program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // 4. Verify payer_token is payer's ATA for payment_mint
        let expected_payer_ata = spl_associated_token_account::get_associated_token_address(
            payer.key,
            &state.payment_mint,
        );
        if *payer_token.key != expected_payer_ata {
            msg!("Invalid payer token account: expected={} got={}", expected_payer_ata, payer_token.key);
            return Err(ContraError::InvalidPaymentToken.into());
        }

        // 5. Verify treasury_token is treasury PDA's ATA for payment_mint
        let expected_treasury_ata = spl_associated_token_account::get_associated_token_address(
            &state.treasury,
            &state.payment_mint,
        );
        if *treasury_token.key != expected_treasury_ata {
            msg!("Invalid treasury token account: expected={} got={}", expected_treasury_ata, treasury_token.key);
            return Err(ContraError::InvalidTreasury.into());
        }

        // 5b. Create treasury PDA's ATA if it doesn't exist yet (first mint)
        if treasury_token.data_is_empty() {
            let (treasury_pda, treasury_bump) = find_treasury_pda(program_id);
            let create_treasury_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
                payer.key,
                &treasury_pda,
                &state.payment_mint,
                token_program.key,
            );
            invoke(
                &create_treasury_ata_ix,
                &[
                    payer.clone(),
                    treasury_token.clone(),
                    token_program.clone(),
                    ata_program.clone(),
                    system_program.clone(),
                ],
            )?;
        }

        // 6. Verify beneficiary_token is beneficiary's ATA for payment_mint
        let expected_beneficiary_ata = spl_associated_token_account::get_associated_token_address(
            &state.beneficiary,
            &state.payment_mint,
        );
        if *beneficiary_token.key != expected_beneficiary_ata {
            msg!("Invalid beneficiary token account: expected={} got={}", expected_beneficiary_ata, beneficiary_token.key);
            return Err(ContraError::InvalidBeneficiary.into());
        }

        // 7. Verify nft_mint is the correct PDA for this token_id
        let (expected_nft_mint, _) = find_mint_counter_pda(program_id, state.total_minted + 1);
        if *nft_mint.key != expected_nft_mint {
            msg!("Invalid NFT mint PDA: expected={} got={}", expected_nft_mint, nft_mint.key);
            return Err(ProgramError::InvalidSeeds);
        }

        if state.paused {
            return Err(ContraError::Paused.into());
        }
        if state.total_minted >= state.max_supply {
            return Err(ContraError::SoldOut.into());
        }

        // Increment counter
        let token_id = state.total_minted + 1;
        state.total_minted = token_id;

        // Transfer payment from minter → treasury
        let transfer_ix = transfer(
            token_program.key,
            payer_token.key,
            treasury_token.key,
            payer.key,
            &[],
            state.mint_price,
        )?;
        invoke(
            &transfer_ix,
            &[
                payer_token.clone(),
                treasury_token.clone(),
                payer.clone(),
                token_program.clone(),
            ],
        )?;

        // Forward payment from treasury → beneficiary
        // Treasury is a PDA ("contra_treasury" seed), so the program can sign transfers.
        // Create beneficiary ATA if it doesn't exist.
        let (treasury_pda, treasury_bump) = find_treasury_pda(program_id);
        if beneficiary_token.key != treasury_token.key {
            // Check if beneficiary ATA exists; if not, create it
            if beneficiary_token.data_is_empty() {
                let create_beneficiary_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
                    payer.key,
                    &state.beneficiary,
                    &state.payment_mint,
                    token_program.key,
                );
                invoke(
                    &create_beneficiary_ata_ix,
                    &[
                        payer.clone(),
                        beneficiary_token.clone(),
                        state_info.clone(), // beneficiary = rent payer? no — payer pays rent
                        system_program.clone(),
                        token_program.clone(),
                        ata_program.clone(),
                    ],
                )?;
            }

            let forward_ix = transfer(
                token_program.key,
                treasury_token.key,
                beneficiary_token.key,
                &treasury_pda,
                &[],
                state.mint_price,
            )?;
            let treasury_seeds: &[&[u8]] = &[TREASURY_SEED, &[treasury_bump]];
            let forward_signer_seeds = &[&treasury_seeds[..]];

            invoke_signed(
                &forward_ix,
                &[
                    treasury_token.clone(),
                    beneficiary_token.clone(),
                    state_info.clone(),
                    token_program.clone(),
                ],
                forward_signer_seeds,
            )?;
        }

        // Mint NFT to minter
        // Create mint PDA for deterministic token mint address
        let (mint_pda, mint_bump) = find_mint_counter_pda(program_id, token_id);
        let seeds: &[&[u8]] = &[crate::state::MINT_COUNTER_SEED, &token_id.to_le_bytes(), &[mint_bump]];
        let signer_seeds = &[&seeds[..]];

        // Create the NFT mint account
        let rent = Rent::get()?;
        let mint_space = 165usize; // spl_token::state::Mint::LEN
        let mint_lamports = rent.minimum_balance(mint_space);

        invoke_signed(
            &solana_program::system_instruction::create_account(
                payer.key,
                nft_mint.key,
                mint_lamports,
                mint_space as u64,
                token_program.key,
            ),
            &[
                payer.clone(),
                nft_mint.clone(),
                system_program.clone(),
                token_program.clone(),
            ],
            signer_seeds,
        )?;

        // Initialize the mint (decimals=0 for NFT)
        let init_mint_ix = spl_token::instruction::initialize_mint(
            token_program.key,
            nft_mint.key,
            &mint_pda,
            Some(&mint_pda),
            0,
        )?;
        invoke(
            &init_mint_ix,
            &[nft_mint.clone(), nft_mint.clone(), token_program.clone()],
        )?;

        // Create ATA for minter
        let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
            payer.key,
            payer.key,
            nft_mint.key,
            token_program.key,
        );
        invoke(
            &create_ata_ix,
            &[
                payer.clone(),
                nft_token.clone(),
                payer.clone(),
                nft_mint.clone(),
                system_program.clone(),
                token_program.clone(),
                ata_program.clone(),
            ],
        )?;

        // Mint 1 token (NFT) to minter
        let mint_to_ix = mint_to(
            token_program.key,
            nft_mint.key,
            nft_token.key,
            &mint_pda,
            &[],
            1,
        )?;
        invoke_signed(
            &mint_to_ix,
            &[
                nft_mint.clone(),
                nft_token.clone(),
                state_info.clone(),
                token_program.clone(),
            ],
            signer_seeds,
        )?;

        Self::serialize_state(state_info, &state)?;

        msg!("Minted ContraNFT #{} to {}", token_id, payer.key);
        Ok(())
    }

    // ═══════════════════════════════════════════
    // Pause / Unpause
    // ═══════════════════════════════════════════

    fn process_set_pause(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        paused: bool,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        state.paused = paused;
        Self::serialize_state(state_info, &state)?;

        msg!("Pause set to: {}", paused);
        Ok(())
    }

    // ═══════════════════════════════════════════
    // Owner Transfer — 24h Timelock
    // ═══════════════════════════════════════════

    fn process_initiate_owner_transfer(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_owner: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;
        let clock_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if *new_owner == Pubkey::default() {
            return Err(ProgramError::InvalidArgument);
        }

        let clock = Self::get_clock(clock_info)?;
        let now = clock.unix_timestamp;

        state.pending_owner = *new_owner;
        state.pending_owner_deadline = now + TIMELOCK_DURATION;

        Self::serialize_state(state_info, &state)?;
        msg!(
            "Owner transfer initiated: pending_owner={} deadline={}",
            new_owner, state.pending_owner_deadline
        );
        Ok(())
    }

    fn process_cancel_owner_transfer(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if state.pending_owner == Pubkey::default() {
            return Err(ContraError::NoPendingChange.into());
        }

        state.pending_owner = Pubkey::default();
        state.pending_owner_deadline = 0;

        Self::serialize_state(state_info, &state)?;
        msg!("Owner transfer cancelled");
        Ok(())
    }

    fn process_accept_ownership(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let new_owner = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;
        let clock_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;

        if *new_owner.key != state.pending_owner {
            return Err(ContraError::NotPendingOwner.into());
        }

        let clock = Self::get_clock(clock_info)?;
        let now = clock.unix_timestamp;

        if now < state.pending_owner_deadline {
            return Err(ContraError::TimelockNotExpired.into());
        }

        let old_owner = state.authority;
        state.authority = state.pending_owner;
        state.pending_owner = Pubkey::default();
        state.pending_owner_deadline = 0;

        Self::serialize_state(state_info, &state)?;
        msg!("Ownership transferred from {} to {}", old_owner, state.authority);
        Ok(())
    }

    // ═══════════════════════════════════════════
    // Max Supply — 24h Timelock
    // ═══════════════════════════════════════════

    fn process_initiate_max_supply_change(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_max: u64,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;
        let clock_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if new_max < state.total_minted {
            return Err(ContraError::InvalidMaxSupply.into());
        }

        let clock = Self::get_clock(clock_info)?;
        let now = clock.unix_timestamp;

        state.pending_max_supply = new_max;
        state.pending_max_supply_deadline = now + TIMELOCK_DURATION;

        Self::serialize_state(state_info, &state)?;
        msg!("Max supply change initiated: {} (deadline={})", new_max, state.pending_max_supply_deadline);
        Ok(())
    }

    fn process_cancel_max_supply_change(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if state.pending_max_supply == 0 {
            return Err(ContraError::NoPendingChange.into());
        }

        state.pending_max_supply = 0;
        state.pending_max_supply_deadline = 0;

        Self::serialize_state(state_info, &state)?;
        msg!("Max supply change cancelled");
        Ok(())
    }

    fn process_execute_max_supply_change(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let _executor = next_account_info(accounts_iter)?; // anyone can execute
        let state_info = next_account_info(accounts_iter)?;
        let clock_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;

        if state.pending_max_supply == 0 {
            return Err(ContraError::NoPendingChange.into());
        }

        let clock = Self::get_clock(clock_info)?;
        let now = clock.unix_timestamp;

        if now < state.pending_max_supply_deadline {
            return Err(ContraError::TimelockNotExpired.into());
        }

        let old_max = state.max_supply;
        state.max_supply = state.pending_max_supply;
        state.pending_max_supply = 0;
        state.pending_max_supply_deadline = 0;

        Self::serialize_state(state_info, &state)?;
        msg!("Max supply updated: {} → {}", old_max, state.max_supply);
        Ok(())
    }

    // ═══════════════════════════════════════════
    // Treasury — 24h Timelock
    // ═══════════════════════════════════════════

    fn process_initiate_treasury_change(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_treasury: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;
        let clock_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if *new_treasury == Pubkey::default() {
            return Err(ContraError::InvalidTreasuryAddress.into());
        }

        let clock = Self::get_clock(clock_info)?;
        let now = clock.unix_timestamp;

        state.pending_treasury = *new_treasury;
        state.pending_treasury_deadline = now + TIMELOCK_DURATION;

        Self::serialize_state(state_info, &state)?;
        msg!("Treasury change initiated: {} (deadline={})", new_treasury, state.pending_treasury_deadline);
        Ok(())
    }

    fn process_cancel_treasury_change(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if state.pending_treasury == Pubkey::default() {
            return Err(ContraError::NoPendingChange.into());
        }

        state.pending_treasury = Pubkey::default();
        state.pending_treasury_deadline = 0;

        Self::serialize_state(state_info, &state)?;
        msg!("Treasury change cancelled");
        Ok(())
    }

    fn process_execute_treasury_change(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let _executor = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;
        let clock_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;

        if state.pending_treasury == Pubkey::default() {
            return Err(ContraError::NoPendingChange.into());
        }

        let clock = Self::get_clock(clock_info)?;
        let now = clock.unix_timestamp;

        if now < state.pending_treasury_deadline {
            return Err(ContraError::TimelockNotExpired.into());
        }

        let old = state.treasury;
        state.treasury = state.pending_treasury;
        state.pending_treasury = Pubkey::default();
        state.pending_treasury_deadline = 0;

        Self::serialize_state(state_info, &state)?;
        msg!("Treasury updated: {} → {}", old, state.treasury);
        Ok(())
    }

    // ═══════════════════════════════════════════
    // Beneficiary — 24h Timelock
    // ═══════════════════════════════════════════

    fn process_initiate_beneficiary_change(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_beneficiary: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;
        let clock_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if *new_beneficiary == Pubkey::default() {
            return Err(ContraError::InvalidBeneficiaryAddress.into());
        }

        let clock = Self::get_clock(clock_info)?;
        let now = clock.unix_timestamp;

        state.pending_beneficiary = *new_beneficiary;
        state.pending_beneficiary_deadline = now + TIMELOCK_DURATION;

        Self::serialize_state(state_info, &state)?;
        msg!("Beneficiary change initiated: {} (deadline={})", new_beneficiary, state.pending_beneficiary_deadline);
        Ok(())
    }

    fn process_cancel_beneficiary_change(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if state.pending_beneficiary == Pubkey::default() {
            return Err(ContraError::NoPendingChange.into());
        }

        state.pending_beneficiary = Pubkey::default();
        state.pending_beneficiary_deadline = 0;

        Self::serialize_state(state_info, &state)?;
        msg!("Beneficiary change cancelled");
        Ok(())
    }

    fn process_execute_beneficiary_change(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let _executor = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;
        let clock_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;

        if state.pending_beneficiary == Pubkey::default() {
            return Err(ContraError::NoPendingChange.into());
        }

        let clock = Self::get_clock(clock_info)?;
        let now = clock.unix_timestamp;

        if now < state.pending_beneficiary_deadline {
            return Err(ContraError::TimelockNotExpired.into());
        }

        let old = state.beneficiary;
        state.beneficiary = state.pending_beneficiary;
        state.pending_beneficiary = Pubkey::default();
        state.pending_beneficiary_deadline = 0;

        Self::serialize_state(state_info, &state)?;
        msg!("Beneficiary updated: {} → {}", old, state.beneficiary);
        Ok(())
    }

    // ═══════════════════════════════════════════
    // Payment Mint — Instant (authority only)
    // ═══════════════════════════════════════════

    fn process_set_payment_mint(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_mint: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if *new_mint == Pubkey::default() {
            return Err(ProgramError::InvalidArgument);
        }

        let old = state.payment_mint;
        state.payment_mint = *new_mint;
        Self::serialize_state(state_info, &state)?;
        msg!("Payment mint updated: {} → {}", old, state.payment_mint);
        Ok(())
    }

    // ═══════════════════════════════════════════
    // Mint Price — Instant (authority only)
    // ═══════════════════════════════════════════

    fn process_set_mint_price(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_price: u64,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        if new_price == 0 {
            return Err(ContraError::InvalidMintPrice.into());
        }

        let old = state.mint_price;
        state.mint_price = new_price;
        Self::serialize_state(state_info, &state)?;
        msg!("Mint price updated: {} → {}", old, state.mint_price);
        Ok(())
    }

    // ═══════════════════════════════════════════
    // No-timelock: Base URI (instant)
    // ═══════════════════════════════════════════

    fn process_set_base_uri(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_uri: String,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let authority = next_account_info(accounts_iter)?;
        let state_info = next_account_info(accounts_iter)?;

        let mut state = Self::deserialize_state(state_info)?;
        Self::check_authority(&state, authority.key)?;

        let uri_bytes = new_uri.as_bytes();
        let len = uri_bytes.len().min(128);
        state.base_uri = [0u8; 128];
        state.base_uri[..len].copy_from_slice(&uri_bytes[..len]);
        state.base_uri_len = len as u8;

        Self::serialize_state(state_info, &state)?;
        msg!("Base URI updated to: {}", new_uri);
        Ok(())
    }
}
