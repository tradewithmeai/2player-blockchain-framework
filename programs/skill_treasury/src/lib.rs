use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Burn};

declare_id!("SkTry9999999999999999999999999999999999999");

/// SOL <-> SKILL treasury program
///
/// Provides a fixed-rate gateway for users to:
/// - Deposit SOL and mint SKILL tokens
/// - Redeem SKILL tokens for SOL
#[program]
pub mod skill_treasury {
    use super::*;

    /// Initialize the treasury with configuration
    ///
    /// # Arguments
    /// * `rate` - How many SKILL tokens per 1 SOL (e.g., 1000 = 1 SOL = 1000 SKILL)
    /// * `fee_bps` - Fee in basis points for operations (e.g., 100 = 1%)
    pub fn initialize(ctx: Context<Initialize>, rate: u64, fee_bps: u16) -> Result<()> {
        require!(rate > 0, TreasuryError::InvalidRate);
        require!(fee_bps < 10000, TreasuryError::InvalidFeeBps);

        let config = &mut ctx.accounts.config;
        config.rate = rate;
        config.fee_bps = fee_bps;
        config.admin = ctx.accounts.admin.key();
        config.skill_mint = ctx.accounts.skill_mint.key();
        config.token_program = ctx.accounts.token_program.key();
        config.treasury_bump = ctx.bumps.config;

        emit!(TreasuryInitialized {
            admin: config.admin,
            rate,
            fee_bps,
        });

        Ok(())
    }

    /// Deposit SOL and mint SKILL tokens
    ///
    /// # Arguments
    /// * `lamports` - Amount of SOL (in lamports) to deposit
    pub fn deposit_sol_and_mint_skill(
        ctx: Context<DepositSolAndMintSkill>,
        lamports: u64,
    ) -> Result<()> {
        require!(lamports > 0, TreasuryError::InvalidAmount);

        let config = &ctx.accounts.config;

        // Calculate SKILL amount (rate is SKILL per SOL)
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

        // Mint SKILL to user
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
    /// # Arguments
    /// * `amount` - Amount of SKILL tokens to redeem
    pub fn redeem_skill_for_sol(
        ctx: Context<RedeemSkillForSol>,
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, TreasuryError::InvalidAmount);

        let config = &ctx.accounts.config;

        // Calculate SOL owed (rate is SKILL per SOL)
        let lamports_owed = amount
            .checked_mul(1_000_000_000) // Convert to lamports
            .ok_or(TreasuryError::MathOverflow)?
            .checked_div(config.rate)
            .ok_or(TreasuryError::MathOverflow)?;

        require!(lamports_owed > 0, TreasuryError::InvalidAmount);

        // Check treasury has enough SOL
        require!(
            ctx.accounts.treasury_sol_vault.lamports() >= lamports_owed,
            TreasuryError::InsufficientLiquidity
        );

        // Burn SKILL from user
        let cpi_accounts = Burn {
            mint: ctx.accounts.skill_mint.to_account_info(),
            from: ctx.accounts.user_skill_ata.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::burn(cpi_ctx, amount)?;

        // Transfer SOL from treasury vault to user
        **ctx.accounts.treasury_sol_vault.try_borrow_mut_lamports()? -= lamports_owed;
        **ctx.accounts.user.try_borrow_mut_lamports()? += lamports_owed;

        emit!(SkillRedeemed {
            user: ctx.accounts.user.key(),
            skill_amount: amount,
            lamports: lamports_owed,
        });

        Ok(())
    }

    /// Update configuration (admin only)
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        rate: Option<u64>,
        fee_bps: Option<u16>,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;

        if let Some(new_rate) = rate {
            require!(new_rate > 0, TreasuryError::InvalidRate);
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
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"treasury_sol_vault"],
        bump
    )]
    /// CHECK: PDA that holds backing SOL
    pub treasury_sol_vault: AccountInfo<'info>,

    #[account(mut)]
    pub skill_mint: Account<'info, Mint>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct DepositSolAndMintSkill<'info> {
    #[account(
        seeds = [b"config"],
        bump = config.treasury_bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"treasury_sol_vault"],
        bump
    )]
    /// CHECK: PDA that holds backing SOL
    pub treasury_sol_vault: AccountInfo<'info>,

    #[account(
        mut,
        address = config.skill_mint
    )]
    pub skill_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = skill_mint,
        associated_token::authority = user
    )]
    pub user_skill_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RedeemSkillForSol<'info> {
    #[account(
        seeds = [b"config"],
        bump = config.treasury_bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"treasury_sol_vault"],
        bump
    )]
    /// CHECK: PDA that holds backing SOL
    pub treasury_sol_vault: AccountInfo<'info>,

    #[account(
        mut,
        address = config.skill_mint
    )]
    pub skill_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = skill_mint,
        associated_token::authority = user
    )]
    pub user_skill_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.treasury_bump,
        has_one = admin
    )]
    pub config: Account<'info, Config>,

    pub admin: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    /// Exchange rate: SKILL tokens per 1 SOL
    pub rate: u64,
    /// Fee in basis points (100 = 1%)
    pub fee_bps: u16,
    /// Admin pubkey
    pub admin: Pubkey,
    /// SKILL mint address
    pub skill_mint: Pubkey,
    /// Token program ID (locked to classic SPL)
    pub token_program: Pubkey,
    /// PDA bump
    pub treasury_bump: u8,
}

#[event]
pub struct TreasuryInitialized {
    pub admin: Pubkey,
    pub rate: u64,
    pub fee_bps: u16,
}

#[event]
pub struct SkillMinted {
    pub user: Pubkey,
    pub lamports: u64,
    pub skill_amount: u64,
}

#[event]
pub struct SkillRedeemed {
    pub user: Pubkey,
    pub skill_amount: u64,
    pub lamports: u64,
}

#[event]
pub struct ConfigUpdated {
    pub rate: u64,
    pub fee_bps: u16,
}

#[error_code]
pub enum TreasuryError {
    #[msg("Invalid rate")]
    InvalidRate,
    #[msg("Invalid fee basis points")]
    InvalidFeeBps,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Insufficient liquidity in treasury")]
    InsufficientLiquidity,
}
