//! # SKILL Treasury Program
//!
//! Provides a fixed-rate gateway between SOL and SKILL tokens with full backing.
//!
//! ## Overview
//!
//! The treasury program manages the conversion between SOL (Solana's native token)
//! and SKILL (SPL token) at a fixed exchange rate. All SKILL tokens are backed 1:1
//! by SOL held in the treasury vault, ensuring users can always redeem their tokens.
//!
//! ## Security Model
//!
//! - **Fixed-rate conversion**: Rate can only be changed by admin
//! - **Full backing**: All SKILL tokens backed by SOL in vault
//! - **Pause mechanism**: Admin can pause operations for emergencies
//! - **Classic SPL tokens only**: Token program ID is validated
//! - **Checked arithmetic**: All calculations use overflow protection
//! - **PDA-based access**: Treasury operations controlled by PDAs
//!
//! ## Instructions
//!
//! 1. `initialize`: One-time setup of treasury configuration
//! 2. `deposit_sol_and_mint_skill`: Deposit SOL, receive SKILL tokens
//! 3. `redeem_skill_for_sol`: Burn SKILL tokens, receive SOL
//! 4. `update_config`: Update rate/fees (admin only)
//! 5. `set_pause`: Pause/unpause operations (admin only)
//!
//! ## Example Usage
//!
//! ```ignore
//! // Deposit 1 SOL to get SKILL tokens
//! let signature = treasury_program.methods
//!     .deposit_sol_and_mint_skill(1_000_000_000) // 1 SOL
//!     .accounts(...)
//!     .rpc()
//!     .await?;
//! ```

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Burn};

declare_id!("SkTry9999999999999999999999999999999999999");

/// Minimum SOL deposit to prevent dust attacks (0.001 SOL)
const MIN_DEPOSIT_LAMPORTS: u64 = 1_000_000;

/// Minimum SKILL redemption to prevent dust (1 SKILL)
const MIN_REDEMPTION_SKILL: u64 = 1_000_000;

/// Maximum rate change per update (10x) to prevent admin errors
const MAX_RATE_MULTIPLIER: u64 = 10;

/// SOL <-> SKILL treasury program
#[program]
pub mod skill_treasury {
    use super::*;

    /// Initialize the treasury with configuration
    ///
    /// Creates the treasury configuration and sets initial parameters.
    /// Can only be called once. The admin becomes the treasury authority.
    ///
    /// # Arguments
    ///
    /// * `rate` - SKILL tokens per 1 SOL (e.g., 1000 = 1 SOL → 1000 SKILL)
    /// * `fee_bps` - Fee in basis points (e.g., 100 = 1%)
    ///
    /// # Security
    ///
    /// - Validates rate is greater than zero
    /// - Validates fee is less than 100%
    /// - Locks token program to classic SPL
    /// - Mint authority should be transferred to config PDA after initialization
    ///
    /// # Errors
    ///
    /// - `TreasuryError::InvalidRate` if rate is zero
    /// - `TreasuryError::InvalidFeeBps` if fee >= 10000 (100%)
    ///
    /// # Events
    ///
    /// Emits `TreasuryInitialized` with admin, rate, and fee details
    pub fn initialize(ctx: Context<Initialize>, rate: u64, fee_bps: u16) -> Result<()> {
        require!(rate > 0, TreasuryError::InvalidRate);
        require!(fee_bps < 10000, TreasuryError::InvalidFeeBps);

        // Verify classic SPL token program
        require_keys_eq!(
            ctx.accounts.token_program.key(),
            token::ID,
            TreasuryError::InvalidTokenProgram
        );

        let config = &mut ctx.accounts.config;
        config.rate = rate;
        config.fee_bps = fee_bps;
        config.admin = ctx.accounts.admin.key();
        config.skill_mint = ctx.accounts.skill_mint.key();
        config.token_program = ctx.accounts.token_program.key();
        config.treasury_bump = ctx.bumps.config;
        config.paused = false;

        emit!(TreasuryInitialized {
            admin: config.admin,
            rate,
            fee_bps,
        });

        Ok(())
    }

    /// Deposit SOL and mint SKILL tokens
    ///
    /// Transfers SOL from user to treasury vault and mints equivalent SKILL tokens
    /// based on the configured exchange rate. The user's ATA is created if needed.
    ///
    /// # Arguments
    ///
    /// * `lamports` - Amount of SOL to deposit (in lamports, 1 SOL = 1e9 lamports)
    ///
    /// # Security
    ///
    /// - Checks treasury is not paused
    /// - Validates minimum deposit amount
    /// - Uses checked arithmetic to prevent overflow
    /// - Verifies mint matches configuration
    /// - Creates ATA safely via associated token program
    ///
    /// # Formula
    ///
    /// ```text
    /// SKILL_amount = (lamports × rate) ÷ 1_000_000_000
    /// ```
    ///
    /// # Errors
    ///
    /// - `TreasuryError::Paused` if operations are paused
    /// - `TreasuryError::InvalidAmount` if deposit too small or calculation yields zero
    /// - `TreasuryError::MathOverflow` if calculation overflows
    /// - `TreasuryError::InvalidMint` if mint doesn't match config
    ///
    /// # Events
    ///
    /// Emits `SkillMinted` with user, lamports deposited, and SKILL minted
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Deposit 1 SOL at rate 1000 → receive 1000 SKILL
    /// treasury_program.methods
    ///     .deposit_sol_and_mint_skill(1_000_000_000)
    ///     .accounts(...)
    ///     .rpc()
    ///     .await?;
    /// ```
    pub fn deposit_sol_and_mint_skill(
        ctx: Context<DepositSolAndMintSkill>,
        lamports: u64,
    ) -> Result<()> {
        let config = &ctx.accounts.config;

        // Check if paused
        require!(!config.paused, TreasuryError::Paused);

        // Validate minimum deposit
        require!(
            lamports >= MIN_DEPOSIT_LAMPORTS,
            TreasuryError::DepositTooSmall
        );

        // Verify mint matches configuration
        require_keys_eq!(
            ctx.accounts.skill_mint.key(),
            config.skill_mint,
            TreasuryError::InvalidMint
        );

        // Calculate SKILL amount with overflow protection
        let skill_amount = lamports
            .checked_mul(config.rate)
            .ok_or(TreasuryError::MathOverflow)?
            .checked_div(1_000_000_000) // Convert lamports to SOL
            .ok_or(TreasuryError::MathOverflow)?;

        require!(skill_amount > 0, TreasuryError::InvalidAmount);

        // Transfer SOL to treasury vault
        let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.treasury_sol_vault.key(),
            lamports,
        );

        anchor_lang::solana_program::program::invoke(
            &transfer_ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.treasury_sol_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Mint SKILL to user with PDA authority
        let seeds = &[b"config".as_ref(), &[config.treasury_bump]];
        let signer = &[&seeds[..]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.skill_mint.to_account_info(),
            to: ctx.accounts.user_skill_ata.to_account_info(),
            authority: ctx.accounts.config.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::mint_to(cpi_ctx, skill_amount)?;

        emit!(SkillMinted {
            user: ctx.accounts.user.key(),
            lamports,
            skill_amount,
        });

        Ok(())
    }

    /// Redeem SKILL tokens for SOL
    ///
    /// Burns SKILL tokens from user's account and transfers equivalent SOL
    /// from treasury vault to user's wallet based on the exchange rate.
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount of SKILL tokens to redeem (atomic units, 6 decimals)
    ///
    /// # Security
    ///
    /// - Checks treasury is not paused
    /// - Validates minimum redemption amount
    /// - Verifies sufficient SOL liquidity in vault
    /// - Burns tokens before transferring SOL (checks-effects-interactions)
    /// - Uses checked arithmetic
    /// - Validates mint matches configuration
    ///
    /// # Formula
    ///
    /// ```text
    /// SOL_lamports = (SKILL_amount × 1_000_000_000) ÷ rate
    /// ```
    ///
    /// # Errors
    ///
    /// - `TreasuryError::Paused` if operations are paused
    /// - `TreasuryError::RedemptionTooSmall` if amount below minimum
    /// - `TreasuryError::InvalidAmount` if calculation yields zero
    /// - `TreasuryError::InsufficientLiquidity` if vault lacks SOL
    /// - `TreasuryError::MathOverflow` if calculation overflows
    /// - `TreasuryError::InvalidMint` if mint doesn't match config
    ///
    /// # Events
    ///
    /// Emits `SkillRedeemed` with user, SKILL burned, and lamports received
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Redeem 500 SKILL at rate 1000 → receive 0.5 SOL
    /// treasury_program.methods
    ///     .redeem_skill_for_sol(500_000_000)
    ///     .accounts(...)
    ///     .rpc()
    ///     .await?;
    /// ```
    pub fn redeem_skill_for_sol(
        ctx: Context<RedeemSkillForSol>,
        amount: u64,
    ) -> Result<()> {
        let config = &ctx.accounts.config;

        // Check if paused
        require!(!config.paused, TreasuryError::Paused);

        // Validate minimum redemption
        require!(
            amount >= MIN_REDEMPTION_SKILL,
            TreasuryError::RedemptionTooSmall
        );

        // Verify mint matches configuration
        require_keys_eq!(
            ctx.accounts.skill_mint.key(),
            config.skill_mint,
            TreasuryError::InvalidMint
        );

        // Calculate SOL owed with overflow protection
        let lamports_owed = amount
            .checked_mul(1_000_000_000) // Convert to lamports
            .ok_or(TreasuryError::MathOverflow)?
            .checked_div(config.rate)
            .ok_or(TreasuryError::MathOverflow)?;

        require!(lamports_owed > 0, TreasuryError::InvalidAmount);

        // Check treasury has sufficient liquidity
        let vault_balance = ctx.accounts.treasury_sol_vault.lamports();
        require!(
            vault_balance >= lamports_owed,
            TreasuryError::InsufficientLiquidity
        );

        // Burn SKILL from user (do this BEFORE transferring SOL)
        let cpi_accounts = Burn {
            mint: ctx.accounts.skill_mint.to_account_info(),
            from: ctx.accounts.user_skill_ata.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::burn(cpi_ctx, amount)?;

        // Transfer SOL from treasury vault to user
        **ctx.accounts.treasury_sol_vault.try_borrow_mut_lamports()? = vault_balance
            .checked_sub(lamports_owed)
            .ok_or(TreasuryError::MathOverflow)?;

        **ctx.accounts.user.try_borrow_mut_lamports()? = ctx
            .accounts
            .user
            .lamports()
            .checked_add(lamports_owed)
            .ok_or(TreasuryError::MathOverflow)?;

        emit!(SkillRedeemed {
            user: ctx.accounts.user.key(),
            skill_amount: amount,
            lamports: lamports_owed,
        });

        Ok(())
    }

    /// Update treasury configuration
    ///
    /// Allows admin to adjust exchange rate and fees. Rate changes are limited
    /// to prevent accidental misconfigurations.
    ///
    /// # Arguments
    ///
    /// * `rate` - New exchange rate (optional)
    /// * `fee_bps` - New fee in basis points (optional)
    ///
    /// # Security
    ///
    /// - Only callable by admin (enforced by `has_one` constraint)
    /// - Rate changes limited to MAX_RATE_MULTIPLIER to prevent errors
    /// - Validates new values before applying
    ///
    /// # Errors
    ///
    /// - `TreasuryError::InvalidRate` if new rate is zero or too different
    /// - `TreasuryError::InvalidFeeBps` if new fee >= 100%
    ///
    /// # Events
    ///
    /// Emits `ConfigUpdated` with new rate and fee
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        rate: Option<u64>,
        fee_bps: Option<u16>,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let old_rate = config.rate;

        if let Some(new_rate) = rate {
            require!(new_rate > 0, TreasuryError::InvalidRate);

            // Prevent drastic rate changes that might be errors
            let max_new_rate = old_rate.checked_mul(MAX_RATE_MULTIPLIER).unwrap_or(u64::MAX);
            let min_new_rate = old_rate.checked_div(MAX_RATE_MULTIPLIER).unwrap_or(1);

            require!(
                new_rate <= max_new_rate && new_rate >= min_new_rate,
                TreasuryError::RateChangeTooLarge
            );

            config.rate = new_rate;
        }

        if let Some(new_fee_bps) = fee_bps {
            require!(new_fee_bps < 10000, TreasuryError::InvalidFeeBps);
            config.fee_bps = new_fee_bps;
        }

        emit!(ConfigUpdated {
            rate: config.rate,
            fee_bps: config.fee_bps,
        });

        Ok(())
    }

    /// Set pause state
    ///
    /// Allows admin to pause or unpause treasury operations for emergency situations.
    /// When paused, deposits and redemptions are blocked.
    ///
    /// # Arguments
    ///
    /// * `paused` - True to pause, false to unpause
    ///
    /// # Security
    ///
    /// - Only callable by admin
    /// - Does not affect existing balances or config
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
// Account Validation Structs
// ============================================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Treasury configuration PDA
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    /// Treasury SOL vault (PDA to hold backing SOL)
    #[account(
        mut,
        seeds = [b"treasury_sol_vault"],
        bump
    )]
    /// CHECK: PDA that holds backing SOL, validated by seeds
    pub treasury_sol_vault: AccountInfo<'info>,

    /// SKILL token mint (admin should transfer authority to config PDA after init)
    #[account(mut)]
    pub skill_mint: Account<'info, Mint>,

    /// Treasury admin
    #[account(mut)]
    pub admin: Signer<'info>,

    /// SPL Token program (classic)
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct DepositSolAndMintSkill<'info> {
    /// Treasury configuration
    #[account(
        seeds = [b"config"],
        bump,
    )]
    pub config: Account<'info, Config>,

    /// Treasury SOL vault
    /// CHECK: PDA that holds backing SOL, validated by seeds
    #[account(
        mut,
        seeds = [b"treasury_sol_vault"],
        bump
    )]
    pub treasury_sol_vault: AccountInfo<'info>,

    /// SKILL token mint (must match config)
    #[account(
        mut,
        address = config.skill_mint,
        mint::authority = config,
    )]
    pub skill_mint: Account<'info, Mint>,

    /// User's SKILL token account (created if needed)
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = skill_mint,
        associated_token::authority = user
    )]
    pub user_skill_ata: Account<'info, TokenAccount>,

    /// User making the deposit
    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RedeemSkillForSol<'info> {
    /// Treasury configuration
    #[account(
        seeds = [b"config"],
        bump,
    )]
    pub config: Account<'info, Config>,

    /// Treasury SOL vault
    /// CHECK: PDA that holds backing SOL, validated by seeds
    #[account(
        mut,
        seeds = [b"treasury_sol_vault"],
        bump
    )]
    pub treasury_sol_vault: AccountInfo<'info>,

    /// SKILL token mint (must match config)
    #[account(
        mut,
        address = config.skill_mint
    )]
    pub skill_mint: Account<'info, Mint>,

    /// User's SKILL token account
    #[account(
        mut,
        associated_token::mint = skill_mint,
        associated_token::authority = user,
        constraint = user_skill_ata.amount >= MIN_REDEMPTION_SKILL @ TreasuryError::InsufficientBalance
    )]
    pub user_skill_ata: Account<'info, TokenAccount>,

    /// User redeeming SKILL
    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    /// Treasury configuration (admin check via has_one)
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.treasury_bump,
        has_one = admin
    )]
    pub config: Account<'info, Config>,

    /// Admin authority
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetPause<'info> {
    /// Treasury configuration (admin check via has_one)
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.treasury_bump,
        has_one = admin
    )]
    pub config: Account<'info, Config>,

    /// Admin authority
    pub admin: Signer<'info>,
}

// ============================================================================
// State Accounts
// ============================================================================

/// Treasury configuration account
///
/// Stores the exchange rate, fees, and operational parameters for the treasury.
/// This account is a PDA controlled by the program.
#[account]
#[derive(InitSpace)]
pub struct Config {
    /// Exchange rate: SKILL tokens per 1 SOL
    /// Example: 1000 means 1 SOL = 1000 SKILL
    pub rate: u64,

    /// Fee in basis points (100 = 1%, 10000 = 100%)
    /// Currently not applied but reserved for future use
    pub fee_bps: u16,

    /// Admin pubkey (can update config and pause)
    pub admin: Pubkey,

    /// SKILL mint address
    pub skill_mint: Pubkey,

    /// Token program ID (locked to classic SPL)
    pub token_program: Pubkey,

    /// Config PDA bump seed
    pub treasury_bump: u8,

    /// Pause state (true = operations paused)
    pub paused: bool,
}

// ============================================================================
// Events
// ============================================================================

/// Emitted when treasury is initialized
#[event]
pub struct TreasuryInitialized {
    pub admin: Pubkey,
    pub rate: u64,
    pub fee_bps: u16,
}

/// Emitted when SKILL tokens are minted
#[event]
pub struct SkillMinted {
    pub user: Pubkey,
    pub lamports: u64,
    pub skill_amount: u64,
}

/// Emitted when SKILL tokens are redeemed
#[event]
pub struct SkillRedeemed {
    pub user: Pubkey,
    pub skill_amount: u64,
    pub lamports: u64,
}

/// Emitted when configuration is updated
#[event]
pub struct ConfigUpdated {
    pub rate: u64,
    pub fee_bps: u16,
}

/// Emitted when pause state changes
#[event]
pub struct PauseStateChanged {
    pub paused: bool,
}

// ============================================================================
// Errors
// ============================================================================

/// Treasury program error codes
#[error_code]
pub enum TreasuryError {
    /// 6000 - Exchange rate must be greater than zero
    #[msg("Exchange rate must be greater than zero")]
    InvalidRate,

    /// 6001 - Fee basis points must be less than 10000 (100%)
    #[msg("Fee basis points must be less than 10000 (100%)")]
    InvalidFeeBps,

    /// 6002 - Amount must be greater than zero
    #[msg("Amount must be greater than zero")]
    InvalidAmount,

    /// 6003 - Mathematical operation caused an overflow
    #[msg("Mathematical operation caused an overflow")]
    MathOverflow,

    /// 6004 - Treasury vault lacks sufficient SOL for redemption
    #[msg("Insufficient liquidity in treasury vault. Please try a smaller amount or wait for more deposits.")]
    InsufficientLiquidity,

    /// 6005 - Deposit amount below minimum threshold
    #[msg("Deposit too small. Minimum deposit is 0.001 SOL")]
    DepositTooSmall,

    /// 6006 - Redemption amount below minimum threshold
    #[msg("Redemption too small. Minimum redemption is 1 SKILL")]
    RedemptionTooSmall,

    /// 6007 - Operations are currently paused by admin
    #[msg("Treasury operations are currently paused")]
    Paused,

    /// 6008 - Provided mint does not match configured SKILL mint
    #[msg("Invalid mint. Must use configured SKILL mint.")]
    InvalidMint,

    /// 6009 - Token program must be classic SPL, not Token-2022
    #[msg("Invalid token program. Must use classic SPL Token program.")]
    InvalidTokenProgram,

    /// 6010 - Rate change exceeds maximum allowed multiplier
    #[msg("Rate change too large. Maximum 10x change per update.")]
    RateChangeTooLarge,

    /// 6011 - User's token account has insufficient balance
    #[msg("Insufficient SKILL token balance")]
    InsufficientBalance,
}
