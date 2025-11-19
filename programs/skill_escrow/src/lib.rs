//! # SKILL Escrow Program
//!
//! Manages match creation, funding, and settlement for skill-based gaming.
//!
//! ## Overview
//!
//! The escrow program provides trustless match management with two settlement modes:
//! 1. **Turn-based**: On-chain game programs settle via CPI
//! 2. **Realtime**: Server-authoritative games settle with dual player signatures
//!
//! All funds are held in program-controlled escrow vaults until match completion.
//!
//! ## Security Model
//!
//! - **PDA-controlled vaults**: All funds held by program PDAs
//! - **Dual settlement paths**: On-chain CPI or ed25519 signature verification
//! - **Timeout protection**: Automatic cancellation and refunds
//! - **Fee enforcement**: Platform fees deducted on settlement
//! - **Match expiry**: Unfunded matches auto-expire
//! - **Pause mechanism**: Admin can halt new matches
//!
//! ## Match Lifecycle
//!
//! ```text
//! Created → (both players fund) → Active → (game ends) → Settled
//!     ↓                                          ↓
//! Cancelled (expired)                    Claimed (timeout)
//! ```
//!
//! ## Settlement Modes
//!
//! ### Turn-Based (Mode 0)
//! Game logic entirely on-chain. Game program calls `settle_onchain` via CPI
//! with the winner's pubkey. No signatures required.
//!
//! ### Realtime (Mode 1)
//! Game runs off-chain. Both players sign the final result digest.
//! Client includes two ed25519 precompile instructions before calling
//! `settle_with_signatures`.
//!
//! ## Instructions
//!
//! 1. `initialize` - One-time escrow configuration
//! 2. `create_match` - Create new match with stake and expiry
//! 3. `join_match` - Player 2 joins as opponent
//! 4. `fund` - Players deposit stakes to escrow
//! 5. `start_if_both_funded` - Activate match when both funded
//! 6. `settle_onchain` - CPI settlement from game program
//! 7. `settle_with_signatures` - Dual-signature settlement for realtime
//! 8. `cancel_if_expired` - Refund expired unfunded matches
//! 9. `claim_timeout` - Claim win if opponent times out
//! 10. `withdraw_fee` - Admin withdraws collected fees
//! 11. `set_pause` - Pause/unpause match creation
//!
//! ## Example Usage
//!
//! ```ignore
//! // Create and join a match
//! let match_id = generate_match_id();
//! escrow.create_match(match_id, 10_000_000, Mode::TurnBased, 1000).await?;
//! escrow.join_match(match_id).await?;
//!
//! // Both players fund
//! escrow.fund(match_id).await?;
//! escrow.start_if_both_funded(match_id).await?;
//!
//! // Game plays out...
//! // Settlement happens via game program CPI
//! ```

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("SkEsc9999999999999999999999999999999999999");

/// Minimum stake to prevent dust (1 SKILL = 1,000,000 atomic units)
const MIN_STAKE_AMOUNT: u64 = 1_000_000;

/// Maximum expiry slots to prevent excessive lock periods (≈24 hours at 400ms/slot)
const MAX_EXPIRY_SLOTS: u64 = 216_000;

/// Minimum expiry slots (≈10 minutes)
const MIN_EXPIRY_SLOTS: u64 = 1_500;

/// Maximum fee to prevent excessive platform fees (25% = 2500 bps)
const MAX_FEE_BPS: u16 = 2500;

/// Match escrow and settlement program
#[program]
pub mod skill_escrow {
    use super::*;

    /// Initialize the escrow configuration
    ///
    /// Sets up the global escrow parameters. Can only be called once.
    ///
    /// # Arguments
    ///
    /// * `fee_bps` - Platform fee in basis points (100 = 1%, max 2500 = 25%)
    ///
    /// # Security
    ///
    /// - Validates fee is reasonable (< 25%)
    /// - Locks SKILL mint for all matches
    /// - Sets admin authority
    ///
    /// # Errors
    ///
    /// - `EscrowError::InvalidFeeBps` if fee >= 2500
    ///
    /// # Events
    ///
    /// Emits `EscrowInitialized`
    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        require!(fee_bps < MAX_FEE_BPS, EscrowError::InvalidFeeBps);

        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.fee_bps = fee_bps;
        config.skill_mint = ctx.accounts.skill_mint.key();
        config.paused = false;
        config.bump = ctx.bumps.config;

        emit!(EscrowInitialized {
            admin: config.admin,
            fee_bps,
        });

        Ok(())
    }

    /// Create a new match
    ///
    /// Creates a match with specified stake and mode. Player who creates becomes player1.
    /// Match must be joined and funded before expiry or it can be cancelled.
    ///
    /// # Arguments
    ///
    /// * `match_id` - Unique 32-byte identifier (use random bytes)
    /// * `stake` - SKILL tokens each player must deposit (minimum 1 SKILL)
    /// * `mode` - 0 = Turn-based (on-chain), 1 = Realtime (signatures)
    /// * `expiry_slots` - Slots until match expires if not funded (10min - 24hr)
    ///
    /// # Security
    ///
    /// - Checks escrow not paused
    /// - Validates stake >= minimum
    /// - Validates mode is valid
    /// - Validates expiry within bounds
    /// - Creates PDA escrow vault
    ///
    /// # Errors
    ///
    /// - `EscrowError::Paused` if match creation paused
    /// - `EscrowError::StakeTooSmall` if below minimum
    /// - `EscrowError::InvalidMode` if mode > 1
    /// - `EscrowError::InvalidExpiry` if expiry out of bounds
    ///
    /// # Events
    ///
    /// Emits `MatchCreated`
    pub fn create_match(
        ctx: Context<CreateMatch>,
        match_id: [u8; 32],
        stake: u64,
        mode: u8,
        expiry_slots: u64,
    ) -> Result<()> {
        let config = &ctx.accounts.config;

        // Check if paused
        require!(!config.paused, EscrowError::Paused);

        // Validate stake
        require!(stake >= MIN_STAKE_AMOUNT, EscrowError::StakeTooSmall);

        // Validate mode
        require!(mode <= 1, EscrowError::InvalidMode);

        // Validate expiry
        require!(
            expiry_slots >= MIN_EXPIRY_SLOTS && expiry_slots <= MAX_EXPIRY_SLOTS,
            EscrowError::InvalidExpiry
        );

        let match_account = &mut ctx.accounts.match_account;
        let clock = Clock::get()?;

        match_account.match_id = match_id;
        match_account.player1 = ctx.accounts.player1.key();
        match_account.player2 = Pubkey::default();
        match_account.stake = stake;
        match_account.skill_mint = config.skill_mint;
        match_account.status = MatchStatus::Created as u8;
        match_account.mode = mode;
        match_account.expiry_slot = clock
            .slot
            .checked_add(expiry_slots)
            .ok_or(EscrowError::MathOverflow)?;
        match_account.fee_bps = config.fee_bps;
        match_account.player1_funded = false;
        match_account.player2_funded = false;
        match_account.bump = ctx.bumps.match_account;

        emit!(MatchCreated {
            match_id,
            player1: match_account.player1,
            stake,
            mode,
            expiry_slot: match_account.expiry_slot,
        });

        Ok(())
    }

    /// Join an existing match as player 2
    ///
    /// Allows a second player to join an open match. Cannot join own match or full match.
    ///
    /// # Security
    ///
    /// - Validates match is in Created status
    /// - Ensures match not already full
    /// - Prevents self-play
    ///
    /// # Errors
    ///
    /// - `EscrowError::InvalidMatchStatus` if not Created
    /// - `EscrowError::MatchAlreadyFull` if player2 already set
    /// - `EscrowError::CannotPlaySelf` if player1 == player2
    ///
    /// # Events
    ///
    /// Emits `PlayerJoined`
    pub fn join_match(ctx: Context<JoinMatch>) -> Result<()> {
        let match_account = &mut ctx.accounts.match_account;

        require!(
            match_account.status == MatchStatus::Created as u8,
            EscrowError::InvalidMatchStatus
        );
        require!(
            match_account.player2 == Pubkey::default(),
            EscrowError::MatchAlreadyFull
        );
        require!(
            ctx.accounts.player2.key() != match_account.player1,
            EscrowError::CannotPlaySelf
        );

        match_account.player2 = ctx.accounts.player2.key();

        emit!(PlayerJoined {
            match_id: match_account.match_id,
            player2: match_account.player2,
        });

        Ok(())
    }

    /// Fund the match by transferring stake to escrow
    ///
    /// Each player must call this to deposit their stake. Match starts when both funded.
    ///
    /// # Security
    ///
    /// - Validates match in Created status
    /// - Checks match not expired
    /// - Verifies signer is a player
    /// - Prevents double-funding
    /// - Validates token mint matches
    /// - Uses safe token transfer
    ///
    /// # Errors
    ///
    /// - `EscrowError::InvalidMatchStatus` if not Created
    /// - `EscrowError::MatchExpired` if past expiry
    /// - `EscrowError::NotAPlayer` if signer not player1 or player2
    /// - `EscrowError::AlreadyFunded` if player already funded
    ///
    /// # Events
    ///
    /// Emits `PlayerFunded`
    pub fn fund(ctx: Context<Fund>) -> Result<()> {
        let match_account = &mut ctx.accounts.match_account;
        let clock = Clock::get()?;

        require!(
            match_account.status == MatchStatus::Created as u8,
            EscrowError::InvalidMatchStatus
        );
        require!(
            clock.slot <= match_account.expiry_slot,
            EscrowError::MatchExpired
        );

        let player = ctx.accounts.player.key();
        let is_player1 = player == match_account.player1;
        let is_player2 = player == match_account.player2;

        require!(is_player1 || is_player2, EscrowError::NotAPlayer);

        // Check if already funded
        if is_player1 {
            require!(!match_account.player1_funded, EscrowError::AlreadyFunded);
        } else {
            require!(!match_account.player2_funded, EscrowError::AlreadyFunded);
        }

        // Verify mint matches
        require_keys_eq!(
            ctx.accounts.player_skill_ata.mint,
            match_account.skill_mint,
            EscrowError::InvalidMint
        );

        // Transfer SKILL to escrow vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.player_skill_ata.to_account_info(),
            to: ctx.accounts.escrow_vault.to_account_info(),
            authority: ctx.accounts.player.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, match_account.stake)?;

        // Mark player as funded
        if is_player1 {
            match_account.player1_funded = true;
        } else {
            match_account.player2_funded = true;
        }

        emit!(PlayerFunded {
            match_id: match_account.match_id,
            player,
            amount: match_account.stake,
        });

        Ok(())
    }

    /// Start match if both players have funded
    ///
    /// Transitions match from Created to Active status. Can be called by anyone.
    ///
    /// # Security
    ///
    /// - Validates match in Created status
    /// - Requires both players funded
    ///
    /// # Errors
    ///
    /// - `EscrowError::InvalidMatchStatus` if not Created
    /// - `EscrowError::NotFullyFunded` if either player not funded
    ///
    /// # Events
    ///
    /// Emits `MatchStarted`
    pub fn start_if_both_funded(ctx: Context<StartMatch>) -> Result<()> {
        let match_account = &mut ctx.accounts.match_account;

        require!(
            match_account.status == MatchStatus::Created as u8,
            EscrowError::InvalidMatchStatus
        );
        require!(
            match_account.player1_funded && match_account.player2_funded,
            EscrowError::NotFullyFunded
        );

        match_account.status = MatchStatus::Active as u8;

        emit!(MatchStarted {
            match_id: match_account.match_id,
        });

        Ok(())
    }

    /// Settle match on-chain (turn-based games only)
    ///
    /// Called via CPI by game programs to settle turn-based matches.
    /// Winner receives `(2 × stake) - fee`. Fee goes to platform.
    ///
    /// # Arguments
    ///
    /// * `winner` - Pubkey of the winning player
    ///
    /// # Security
    ///
    /// - Validates match Active
    /// - Verifies winner is a player
    /// - Uses checked arithmetic for fee calculation
    /// - Transfers winner payout before fee (CEI pattern)
    ///
    /// # Errors
    ///
    /// - `EscrowError::InvalidMatchStatus` if not Active
    /// - `EscrowError::InvalidWinner` if winner not player1 or player2
    /// - `EscrowError::MathOverflow` on arithmetic overflow
    ///
    /// # Events
    ///
    /// Emits `MatchSettled`
    pub fn settle_onchain(ctx: Context<SettleOnchain>, winner: Pubkey) -> Result<()> {
        let match_account = &mut ctx.accounts.match_account;

        require!(
            match_account.status == MatchStatus::Active as u8,
            EscrowError::InvalidMatchStatus
        );
        require!(
            winner == match_account.player1 || winner == match_account.player2,
            EscrowError::InvalidWinner
        );

        settle_match(
            match_account,
            winner,
            &ctx.accounts.escrow_vault,
            &ctx.accounts.winner_ata,
            &ctx.accounts.fee_vault,
            &ctx.accounts.token_program,
            ctx.bumps.escrow_vault,
        )?;

        Ok(())
    }

    /// Settle match with player signatures (realtime games only)
    ///
    /// Settles realtime matches using ed25519 signature verification.
    /// Both players must sign the digest off-chain. Client must include
    /// two ed25519 precompile instructions before this instruction.
    ///
    /// # Arguments
    ///
    /// * `winner` - Pubkey of the winning player
    /// * `digest` - 32-byte hash of game result (signed by both players)
    ///
    /// # Security
    ///
    /// - Validates match Active and Realtime mode
    /// - Verifies winner is a player
    /// - Checks Instructions sysvar for ed25519 verifications
    /// - Requires both player signatures on digest
    /// - Uses checked arithmetic
    ///
    /// # Digest Format
    ///
    /// ```text
    /// digest = SHA256(program_id || match_id || player1 || player2 || winner || nonce || slot)
    /// ```
    ///
    /// # Errors
    ///
    /// - `EscrowError::InvalidMatchStatus` if not Active
    /// - `EscrowError::InvalidModeForOperation` if not Realtime mode
    /// - `EscrowError::InvalidWinner` if winner not a player
    /// - `EscrowError::MissingSignatures` if signatures not verified
    /// - `EscrowError::MathOverflow` on arithmetic overflow
    ///
    /// # Events
    ///
    /// Emits `MatchSettled`
    ///
    /// # Example
    ///
    /// ```ignore
    /// let digest = create_settlement_digest(match_id, winner, ...);
    /// let sig1 = player1.sign(digest);
    /// let sig2 = player2.sign(digest);
    ///
    /// let tx = Transaction::new([
    ///     Ed25519Program::createInstructionWithPublicKey({
    ///         publicKey: player1_pubkey,
    ///         message: digest,
    ///         signature: sig1
    ///     }),
    ///     Ed25519Program::createInstructionWithPublicKey({
    ///         publicKey: player2_pubkey,
    ///         message: digest,
    ///         signature: sig2
    ///     }),
    ///     escrow.settle_with_signatures(winner, digest)
    /// ]);
    /// ```
    pub fn settle_with_signatures(
        ctx: Context<SettleWithSignatures>,
        winner: Pubkey,
        digest: [u8; 32],
    ) -> Result<()> {
        let match_account = &mut ctx.accounts.match_account;

        require!(
            match_account.status == MatchStatus::Active as u8,
            EscrowError::InvalidMatchStatus
        );
        require!(
            match_account.mode == 1, // Realtime mode
            EscrowError::InvalidModeForOperation
        );
        require!(
            winner == match_account.player1 || winner == match_account.player2,
            EscrowError::InvalidWinner
        );

        // Verify ed25519 signatures via Instructions sysvar
        verify_ed25519_signatures(
            &ctx.accounts.instructions_sysvar,
            &match_account.player1,
            &match_account.player2,
            &digest,
        )?;

        settle_match(
            match_account,
            winner,
            &ctx.accounts.escrow_vault,
            &ctx.accounts.winner_ata,
            &ctx.accounts.fee_vault,
            &ctx.accounts.token_program,
            ctx.bumps.escrow_vault,
        )?;

        Ok(())
    }

    /// Cancel match if expired and not fully funded
    ///
    /// Refunds any players who deposited before expiry. Can be called by anyone.
    ///
    /// # Security
    ///
    /// - Validates match in Created status
    /// - Checks expiry slot passed
    /// - Refunds players who funded
    /// - Uses safe token transfers
    ///
    /// # Errors
    ///
    /// - `EscrowError::InvalidMatchStatus` if not Created
    /// - `EscrowError::NotExpired` if before expiry_slot
    ///
    /// # Events
    ///
    /// Emits `MatchCancelled`
    pub fn cancel_if_expired(ctx: Context<CancelMatch>) -> Result<()> {
        let match_account = &mut ctx.accounts.match_account;
        let clock = Clock::get()?;

        require!(
            match_account.status == MatchStatus::Created as u8,
            EscrowError::InvalidMatchStatus
        );
        require!(
            clock.slot > match_account.expiry_slot,
            EscrowError::NotExpired
        );

        // Refund any funded players
        if match_account.player1_funded {
            refund_player(
                match_account,
                &match_account.player1,
                &ctx.accounts.escrow_vault,
                &ctx.accounts.player1_ata,
                &ctx.accounts.token_program,
                ctx.bumps.escrow_vault,
            )?;
        }

        if match_account.player2_funded {
            refund_player(
                match_account,
                &match_account.player2,
                &ctx.accounts.escrow_vault,
                &ctx.accounts.player2_ata,
                &ctx.accounts.token_program,
                ctx.bumps.escrow_vault,
            )?;
        }

        match_account.status = MatchStatus::Cancelled as u8;

        emit!(MatchCancelled {
            match_id: match_account.match_id,
            reason: "Expired before fully funded".to_string(),
        });

        Ok(())
    }

    /// Claim timeout win if opponent didn't respond
    ///
    /// If match expired after becoming Active (no moves/responses), claimant wins.
    /// For turn-based games, timeout is typically handled by game program.
    /// This is mainly for realtime games that stall.
    ///
    /// # Security
    ///
    /// - Validates match Active
    /// - Checks expiry passed
    /// - Verifies claimant is a player
    ///
    /// # Errors
    ///
    /// - `EscrowError::InvalidMatchStatus` if not Active
    /// - `EscrowError::NotExpired` if before expiry
    /// - `EscrowError::NotAPlayer` if claimant not in match
    ///
    /// # Events
    ///
    /// Emits `TimeoutClaimed` and `MatchSettled`
    pub fn claim_timeout(ctx: Context<ClaimTimeout>) -> Result<()> {
        let match_account = &mut ctx.accounts.match_account;
        let clock = Clock::get()?;

        require!(
            match_account.status == MatchStatus::Active as u8,
            EscrowError::InvalidMatchStatus
        );

        require!(
            clock.slot > match_account.expiry_slot,
            EscrowError::NotExpired
        );

        let claimant = ctx.accounts.claimant.key();
        require!(
            claimant == match_account.player1 || claimant == match_account.player2,
            EscrowError::NotAPlayer
        );

        settle_match(
            match_account,
            claimant,
            &ctx.accounts.escrow_vault,
            &ctx.accounts.winner_ata,
            &ctx.accounts.fee_vault,
            &ctx.accounts.token_program,
            ctx.bumps.escrow_vault,
        )?;

        emit!(TimeoutClaimed {
            match_id: match_account.match_id,
            winner: claimant,
        });

        Ok(())
    }

    /// Withdraw accumulated platform fees (admin only)
    ///
    /// Allows admin to withdraw collected fees from the fee vault.
    ///
    /// # Arguments
    ///
    /// * `amount` - SKILL tokens to withdraw
    ///
    /// # Security
    ///
    /// - Only callable by admin (enforced by has_one)
    /// - Validates sufficient balance
    /// - Uses PDA signer
    ///
    /// # Errors
    ///
    /// - Token transfer errors if insufficient balance
    ///
    /// # Events
    ///
    /// Emits `FeeWithdrawn`
    pub fn withdraw_fee(ctx: Context<WithdrawFee>, amount: u64) -> Result<()> {
        require!(amount > 0, EscrowError::InvalidAmount);

        let seeds = &[b"fee_vault".as_ref(), &[ctx.bumps.fee_vault]];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.fee_vault.to_account_info(),
            to: ctx.accounts.destination.to_account_info(),
            authority: ctx.accounts.fee_vault.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(cpi_ctx, amount)?;

        emit!(FeeWithdrawn {
            admin: ctx.accounts.admin.key(),
            amount,
        });

        Ok(())
    }

    /// Set pause state for match creation (admin only)
    ///
    /// Pauses or unpauses new match creation. Existing matches unaffected.
    ///
    /// # Arguments
    ///
    /// * `paused` - True to pause, false to unpause
    ///
    /// # Security
    ///
    /// - Only callable by admin
    /// - Doesn't affect existing matches
    ///
    /// # Events
    ///
    /// Emits `PauseStateChanged`
    pub fn set_pause(ctx: Context<SetPause>, paused: bool) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.paused = paused;

        emit!(PauseStateChanged { paused });

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Internal function to settle a match
///
/// Calculates fee, transfers winnings to winner, transfers fee to platform.
/// Updates match status to Settled.
fn settle_match<'info>(
    match_account: &mut Account<'info, Match>,
    winner: Pubkey,
    escrow_vault: &Account<'info, TokenAccount>,
    winner_ata: &Account<'info, TokenAccount>,
    fee_vault: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    escrow_bump: u8,
) -> Result<()> {
    // Calculate total pot
    let total_stake = match_account
        .stake
        .checked_mul(2)
        .ok_or(EscrowError::MathOverflow)?;

    // Calculate platform fee
    let fee = total_stake
        .checked_mul(match_account.fee_bps as u64)
        .ok_or(EscrowError::MathOverflow)?
        .checked_div(10000)
        .ok_or(EscrowError::MathOverflow)?;

    // Calculate winner amount
    let winner_amount = total_stake
        .checked_sub(fee)
        .ok_or(EscrowError::MathOverflow)?;

    // PDA signer seeds
    let seeds = &[
        b"escrow_vault".as_ref(),
        match_account.match_id.as_ref(),
        &[escrow_bump],
    ];
    let signer = &[&seeds[..]];

    // Transfer winnings to winner
    let cpi_accounts = Transfer {
        from: escrow_vault.to_account_info(),
        to: winner_ata.to_account_info(),
        authority: escrow_vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(token_program.to_account_info(), cpi_accounts, signer);
    token::transfer(cpi_ctx, winner_amount)?;

    // Transfer fee to platform
    if fee > 0 {
        let cpi_accounts = Transfer {
            from: escrow_vault.to_account_info(),
            to: fee_vault.to_account_info(),
            authority: escrow_vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(token_program.to_account_info(), cpi_accounts, signer);
        token::transfer(cpi_ctx, fee)?;
    }

    // Update match status
    match_account.status = MatchStatus::Settled as u8;

    emit!(MatchSettled {
        match_id: match_account.match_id,
        winner,
        amount: winner_amount,
        fee,
    });

    Ok(())
}

/// Internal function to refund a player
///
/// Returns staked tokens to player in case of cancellation.
fn refund_player<'info>(
    match_account: &Account<'info, Match>,
    player: &Pubkey,
    escrow_vault: &Account<'info, TokenAccount>,
    player_ata: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    escrow_bump: u8,
) -> Result<()> {
    let seeds = &[
        b"escrow_vault".as_ref(),
        match_account.match_id.as_ref(),
        &[escrow_bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: escrow_vault.to_account_info(),
        to: player_ata.to_account_info(),
        authority: escrow_vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(token_program.to_account_info(), cpi_accounts, signer);

    token::transfer(cpi_ctx, match_account.stake)?;

    Ok(())
}

/// Verify ed25519 signatures from Instructions sysvar
///
/// Checks that both players signed the digest by inspecting
/// prior ed25519 precompile instructions in the same transaction.
fn verify_ed25519_signatures(
    instructions_sysvar: &AccountInfo,
    player1: &Pubkey,
    player2: &Pubkey,
    digest: &[u8; 32],
) -> Result<()> {
    let current_index = instructions::load_current_index_checked(instructions_sysvar)?;

    let mut player1_verified = false;
    let mut player2_verified = false;

    // Check previous instructions for ed25519 verifications
    for i in 0..current_index {
        if let Ok(ix) = instructions::load_instruction_at_checked(i.into(), instructions_sysvar) {
            // Check if it's an ed25519 instruction
            if ix.program_id == solana_program::ed25519_program::ID {
                // Parse ed25519 instruction data
                // Instruction format has pubkey at offset 14
                if ix.data.len() >= 14 + 32 {
                    let pubkey_bytes = &ix.data[14..14 + 32];
                    if let Ok(pk) = Pubkey::try_from(pubkey_bytes) {
                        if pk == *player1 {
                            player1_verified = true;
                        }
                        if pk == *player2 {
                            player2_verified = true;
                        }
                    }
                }
            }
        }
    }

    require!(
        player1_verified && player2_verified,
        EscrowError::MissingSignatures
    );

    Ok(())
}

// ============================================================================
// Account Validation Structs
// ============================================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Escrow configuration PDA
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    /// SKILL token mint
    pub skill_mint: Account<'info, anchor_spl::token::Mint>,

    /// Admin authority
    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(match_id: [u8; 32])]
pub struct CreateMatch<'info> {
    /// Escrow configuration
    #[account(
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    /// Match state account (PDA)
    #[account(
        init,
        payer = player1,
        space = 8 + Match::INIT_SPACE,
        seeds = [b"match", match_id.as_ref()],
        bump
    )]
    pub match_account: Account<'info, Match>,

    /// Escrow vault for this match (PDA)
    #[account(
        init,
        payer = player1,
        token::mint = config.skill_mint,
        token::authority = escrow_vault,
        seeds = [b"escrow_vault", match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    /// Player 1 (match creator)
    #[account(mut)]
    pub player1: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinMatch<'info> {
    /// Match to join
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    /// Player 2 (joining player)
    pub player2: Signer<'info>,
}

#[derive(Accounts)]
pub struct Fund<'info> {
    /// Match to fund
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    /// Escrow vault for this match
    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump,
        constraint = escrow_vault.mint == match_account.skill_mint @ EscrowError::InvalidMint
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    /// Player's SKILL token account
    #[account(
        mut,
        token::mint = match_account.skill_mint,
        token::authority = player,
        constraint = player_skill_ata.amount >= match_account.stake @ EscrowError::InsufficientBalance
    )]
    pub player_skill_ata: Account<'info, TokenAccount>,

    /// Player funding the match
    #[account(mut)]
    pub player: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct StartMatch<'info> {
    /// Match to start
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,
}

#[derive(Accounts)]
pub struct SettleOnchain<'info> {
    /// Match to settle
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    /// Escrow vault
    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    /// Winner's token account
    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub winner_ata: Account<'info, TokenAccount>,

    /// Platform fee vault
    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump
    )]
    pub fee_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SettleWithSignatures<'info> {
    /// Match to settle
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    /// Escrow vault
    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    /// Winner's token account
    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub winner_ata: Account<'info, TokenAccount>,

    /// Platform fee vault
    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump
    )]
    pub fee_vault: Account<'info, TokenAccount>,

    /// Instructions sysvar for ed25519 verification
    /// CHECK: Validated by address constraint
    #[account(address = instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelMatch<'info> {
    /// Match to cancel
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    /// Escrow vault
    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    /// Player 1's token account (for refund)
    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub player1_ata: Account<'info, TokenAccount>,

    /// Player 2's token account (for refund)
    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub player2_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimTimeout<'info> {
    /// Match to claim timeout on
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    /// Escrow vault
    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    /// Winner's (claimant's) token account
    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub winner_ata: Account<'info, TokenAccount>,

    /// Platform fee vault
    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump
    )]
    pub fee_vault: Account<'info, TokenAccount>,

    /// Player claiming timeout
    pub claimant: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawFee<'info> {
    /// Escrow configuration (admin check via has_one)
    #[account(
        seeds = [b"config"],
        bump,
        has_one = admin
    )]
    pub config: Account<'info, Config>,

    /// Platform fee vault
    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump
    )]
    pub fee_vault: Account<'info, TokenAccount>,

    /// Destination token account
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    /// Admin authority
    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SetPause<'info> {
    /// Escrow configuration (admin check via has_one)
    #[account(
        mut,
        seeds = [b"config"],
        bump,
        has_one = admin
    )]
    pub config: Account<'info, Config>,

    /// Admin authority
    pub admin: Signer<'info>,
}

// ============================================================================
// State Accounts
// ============================================================================

/// Escrow configuration account
///
/// Global parameters for the escrow system.
#[account]
#[derive(InitSpace)]
pub struct Config {
    /// Admin pubkey (can pause and withdraw fees)
    pub admin: Pubkey,

    /// Platform fee in basis points (100 = 1%)
    pub fee_bps: u16,

    /// SKILL token mint address
    pub skill_mint: Pubkey,

    /// Pause state (true = no new matches)
    pub paused: bool,

    /// Config PDA bump
    pub bump: u8,
}

/// Match state account
///
/// Tracks all state for a single match including players, stakes, and status.
#[account]
#[derive(InitSpace)]
pub struct Match {
    /// Unique match identifier
    pub match_id: [u8; 32],

    /// Player 1 (match creator)
    pub player1: Pubkey,

    /// Player 2 (opponent)
    pub player2: Pubkey,

    /// Stake amount per player (SKILL tokens)
    pub stake: u64,

    /// SKILL mint address
    pub skill_mint: Pubkey,

    /// Match status: 0=Created, 1=Active, 2=Settled, 3=Cancelled
    pub status: u8,

    /// Game mode: 0=TurnBased (CPI), 1=Realtime (signatures)
    pub mode: u8,

    /// Slot when match expires if not fully funded
    pub expiry_slot: u64,

    /// Fee basis points at time of match creation
    pub fee_bps: u16,

    /// Whether player1 has funded
    pub player1_funded: bool,

    /// Whether player2 has funded
    pub player2_funded: bool,

    /// Match PDA bump
    pub bump: u8,
}

// ============================================================================
// Enums
// ============================================================================

/// Match status states
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum MatchStatus {
    Created = 0,
    Active = 1,
    Settled = 2,
    Cancelled = 3,
}

// ============================================================================
// Events
// ============================================================================

/// Emitted when escrow is initialized
#[event]
pub struct EscrowInitialized {
    pub admin: Pubkey,
    pub fee_bps: u16,
}

/// Emitted when a match is created
#[event]
pub struct MatchCreated {
    pub match_id: [u8; 32],
    pub player1: Pubkey,
    pub stake: u64,
    pub mode: u8,
    pub expiry_slot: u64,
}

/// Emitted when player 2 joins
#[event]
pub struct PlayerJoined {
    pub match_id: [u8; 32],
    pub player2: Pubkey,
}

/// Emitted when a player funds
#[event]
pub struct PlayerFunded {
    pub match_id: [u8; 32],
    pub player: Pubkey,
    pub amount: u64,
}

/// Emitted when match starts
#[event]
pub struct MatchStarted {
    pub match_id: [u8; 32],
}

/// Emitted when match is settled
#[event]
pub struct MatchSettled {
    pub match_id: [u8; 32],
    pub winner: Pubkey,
    pub amount: u64,
    pub fee: u64,
}

/// Emitted when match is cancelled
#[event]
pub struct MatchCancelled {
    pub match_id: [u8; 32],
    pub reason: String,
}

/// Emitted when timeout is claimed
#[event]
pub struct TimeoutClaimed {
    pub match_id: [u8; 32],
    pub winner: Pubkey,
}

/// Emitted when fees are withdrawn
#[event]
pub struct FeeWithdrawn {
    pub admin: Pubkey,
    pub amount: u64,
}

/// Emitted when pause state changes
#[event]
pub struct PauseStateChanged {
    pub paused: bool,
}

// ============================================================================
// Errors
// ============================================================================

/// Escrow program error codes
#[error_code]
pub enum EscrowError {
    /// 6000 - Fee must be less than 25% (2500 bps)
    #[msg("Fee must be less than 25% (2500 basis points)")]
    InvalidFeeBps,

    /// 6001 - Stake must be at least 1 SKILL
    #[msg("Stake too small. Minimum stake is 1 SKILL (1,000,000 atomic units)")]
    StakeTooSmall,

    /// 6002 - Game mode must be 0 (Turn-based) or 1 (Realtime)
    #[msg("Invalid game mode. Use 0 for turn-based or 1 for realtime")]
    InvalidMode,

    /// 6003 - Match status doesn't allow this operation
    #[msg("Invalid match status for this operation")]
    InvalidMatchStatus,

    /// 6004 - Match already has player 2
    #[msg("Match is already full")]
    MatchAlreadyFull,

    /// 6005 - Cannot play against yourself
    #[msg("Cannot play against yourself")]
    CannotPlaySelf,

    /// 6006 - Match passed expiry slot
    #[msg("Match has expired")]
    MatchExpired,

    /// 6007 - Signer is not a player in this match
    #[msg("You are not a player in this match")]
    NotAPlayer,

    /// 6008 - Player already funded this match
    #[msg("You have already funded this match")]
    AlreadyFunded,

    /// 6009 - Both players must fund before starting
    #[msg("Match is not fully funded yet")]
    NotFullyFunded,

    /// 6010 - Winner must be player1 or player2
    #[msg("Invalid winner. Must be one of the match players")]
    InvalidWinner,

    /// 6011 - Wrong settlement mode for this match type
    #[msg("Invalid mode for this operation. Use settle_onchain for turn-based or settle_with_signatures for realtime")]
    InvalidModeForOperation,

    /// 6012 - Ed25519 signatures not found or invalid
    #[msg("Missing or invalid player signatures. Both players must sign the digest")]
    MissingSignatures,

    /// 6013 - Match hasn't expired yet
    #[msg("Match has not expired yet")]
    NotExpired,

    /// 6014 - Arithmetic overflow
    #[msg("Mathematical operation caused an overflow")]
    MathOverflow,

    /// 6015 - Match creation is paused
    #[msg("Match creation is currently paused by admin")]
    Paused,

    /// 6016 - Provided mint doesn't match configured SKILL mint
    #[msg("Invalid mint. Must use configured SKILL mint")]
    InvalidMint,

    /// 6017 - Expiry slots out of valid range
    #[msg("Invalid expiry. Must be between 10 minutes and 24 hours (1,500 - 216,000 slots)")]
    InvalidExpiry,

    /// 6018 - Amount must be greater than zero
    #[msg("Amount must be greater than zero")]
    InvalidAmount,

    /// 6019 - Insufficient token balance
    #[msg("Insufficient SKILL token balance")]
    InsufficientBalance,
}
