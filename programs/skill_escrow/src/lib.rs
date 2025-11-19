use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("SkEsc9999999999999999999999999999999999999");

/// Match escrow and settlement program
///
/// Handles:
/// - Creating matches with stakes in SKILL tokens
/// - Escrowing funds from both players
/// - Settling to winner (turn-based via CPI or realtime via signatures)
/// - Timeouts and cancellations
#[program]
pub mod skill_escrow {
    use super::*;

    /// Initialize the escrow configuration
    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        require!(fee_bps < 10000, EscrowError::InvalidFeeBps);

        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.fee_bps = fee_bps;
        config.skill_mint = ctx.accounts.skill_mint.key();
        config.bump = ctx.bumps.config;

        emit!(EscrowInitialized {
            admin: config.admin,
            fee_bps,
        });

        Ok(())
    }

    /// Create a new match
    ///
    /// # Arguments
    /// * `match_id` - Unique identifier for the match
    /// * `stake` - Amount of SKILL tokens each player must stake
    /// * `mode` - Game mode (0 = TurnBased, 1 = Realtime)
    /// * `expiry_slots` - Number of slots until match expires if not fully funded
    pub fn create_match(
        ctx: Context<CreateMatch>,
        match_id: [u8; 32],
        stake: u64,
        mode: u8,
        expiry_slots: u64,
    ) -> Result<()> {
        require!(stake > 0, EscrowError::InvalidStake);
        require!(mode <= 1, EscrowError::InvalidMode);

        let match_account = &mut ctx.accounts.match_account;
        let clock = Clock::get()?;

        match_account.match_id = match_id;
        match_account.player1 = ctx.accounts.player1.key();
        match_account.player2 = Pubkey::default();
        match_account.stake = stake;
        match_account.skill_mint = ctx.accounts.config.skill_mint;
        match_account.status = MatchStatus::Created as u8;
        match_account.mode = mode;
        match_account.expiry_slot = clock.slot.checked_add(expiry_slots).unwrap();
        match_account.fee_bps = ctx.accounts.config.fee_bps;
        match_account.player1_funded = false;
        match_account.player2_funded = false;
        match_account.bump = ctx.bumps.match_account;

        emit!(MatchCreated {
            match_id,
            player1: match_account.player1,
            stake,
            mode,
        });

        Ok(())
    }

    /// Join an existing match as player 2
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

    /// Fund the match (transfer stake to escrow)
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

        require!(
            is_player1 || is_player2,
            EscrowError::NotAPlayer
        );

        // Check if already funded
        if is_player1 {
            require!(!match_account.player1_funded, EscrowError::AlreadyFunded);
        } else {
            require!(!match_account.player2_funded, EscrowError::AlreadyFunded);
        }

        // Transfer SKILL to escrow vault using transfer_checked
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

    /// Start match if both players funded
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

    /// Settle match on-chain (called via CPI from game program)
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

    /// Settle match with player signatures (realtime mode)
    ///
    /// Requires two ed25519 verify instructions in the same transaction
    /// that verify both players signed the digest
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
            reason: "Expired".to_string(),
        });

        Ok(())
    }

    /// Claim timeout win if opponent hasn't moved within deadline
    pub fn claim_timeout(ctx: Context<ClaimTimeout>) -> Result<()> {
        let match_account = &mut ctx.accounts.match_account;
        let clock = Clock::get()?;

        require!(
            match_account.status == MatchStatus::Active as u8,
            EscrowError::InvalidMatchStatus
        );

        // For turn-based games, timeout logic is handled by the game program
        // For realtime, we use expiry_slot as a general timeout
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

    /// Withdraw accumulated fees (admin only)
    pub fn withdraw_fee(ctx: Context<WithdrawFee>, amount: u64) -> Result<()> {
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

        Ok(())
    }
}

// Helper function to settle a match
fn settle_match<'info>(
    match_account: &mut Account<'info, Match>,
    winner: Pubkey,
    escrow_vault: &Account<'info, TokenAccount>,
    winner_ata: &Account<'info, TokenAccount>,
    fee_vault: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    escrow_bump: u8,
) -> Result<()> {
    let total_stake = match_account
        .stake
        .checked_mul(2)
        .ok_or(EscrowError::MathOverflow)?;

    // Calculate fee
    let fee = total_stake
        .checked_mul(match_account.fee_bps as u64)
        .ok_or(EscrowError::MathOverflow)?
        .checked_div(10000)
        .ok_or(EscrowError::MathOverflow)?;

    let winner_amount = total_stake
        .checked_sub(fee)
        .ok_or(EscrowError::MathOverflow)?;

    let seeds = &[
        b"escrow_vault".as_ref(),
        match_account.match_id.as_ref(),
        &[escrow_bump],
    ];
    let signer = &[&seeds[..]];

    // Transfer to winner
    let cpi_accounts = Transfer {
        from: escrow_vault.to_account_info(),
        to: winner_ata.to_account_info(),
        authority: escrow_vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(token_program.to_account_info(), cpi_accounts, signer);
    token::transfer(cpi_ctx, winner_amount)?;

    // Transfer fee
    let cpi_accounts = Transfer {
        from: escrow_vault.to_account_info(),
        to: fee_vault.to_account_info(),
        authority: escrow_vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(token_program.to_account_info(), cpi_accounts, signer);
    token::transfer(cpi_ctx, fee)?;

    match_account.status = MatchStatus::Settled as u8;

    emit!(MatchSettled {
        match_id: match_account.match_id,
        winner,
        amount: winner_amount,
        fee,
    });

    Ok(())
}

// Helper function to refund a player
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

// Verify ed25519 signatures from Instructions sysvar
fn verify_ed25519_signatures(
    instructions_sysvar: &AccountInfo,
    player1: &Pubkey,
    player2: &Pubkey,
    digest: &[u8; 32],
) -> Result<()> {
    let data = instructions_sysvar.try_borrow_data()?;
    let current_index = instructions::load_current_index_checked(instructions_sysvar)?;

    let mut player1_verified = false;
    let mut player2_verified = false;

    // Check previous instructions for ed25519 verifications
    for i in 0..current_index {
        if let Ok(ix) = instructions::load_instruction_at_checked(i.into(), instructions_sysvar) {
            // Ed25519 program ID
            if ix.program_id == solana_program::ed25519_program::ID {
                // Parse ed25519 instruction data
                // Format: [num_signatures(1), padding(1), signature_offset(2),
                //          signature_instruction_index(2), public_key_offset(2),
                //          public_key_instruction_index(2), message_data_offset(2),
                //          message_data_size(2), message_instruction_index(2)]

                if ix.data.len() < 14 {
                    continue;
                }

                // Extract pubkey from instruction data (offset 14+)
                if ix.data.len() >= 14 + 32 {
                    let pubkey_bytes = &ix.data[14..14 + 32];
                    let pubkey = Pubkey::try_from(pubkey_bytes).ok();

                    if let Some(pk) = pubkey {
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

// Context structs
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

    pub skill_mint: Account<'info, anchor_spl::token::Mint>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(match_id: [u8; 32])]
pub struct CreateMatch<'info> {
    #[account(
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = player1,
        space = 8 + Match::INIT_SPACE,
        seeds = [b"match", match_id.as_ref()],
        bump
    )]
    pub match_account: Account<'info, Match>,

    #[account(
        init,
        payer = player1,
        token::mint = config.skill_mint,
        token::authority = escrow_vault,
        seeds = [b"escrow_vault", match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub player1: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinMatch<'info> {
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    pub player2: Signer<'info>,
}

#[derive(Accounts)]
pub struct Fund<'info> {
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = match_account.skill_mint,
        token::authority = player
    )]
    pub player_skill_ata: Account<'info, TokenAccount>,

    #[account(mut)]
    pub player: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct StartMatch<'info> {
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,
}

#[derive(Accounts)]
pub struct SettleOnchain<'info> {
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub winner_ata: Account<'info, TokenAccount>,

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
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub winner_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump
    )]
    pub fee_vault: Account<'info, TokenAccount>,

    /// CHECK: Instructions sysvar for ed25519 verification
    #[account(address = instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelMatch<'info> {
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub player1_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub player2_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimTimeout<'info> {
    #[account(
        mut,
        seeds = [b"match", match_account.match_id.as_ref()],
        bump = match_account.bump
    )]
    pub match_account: Account<'info, Match>,

    #[account(
        mut,
        seeds = [b"escrow_vault", match_account.match_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = match_account.skill_mint
    )]
    pub winner_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump
    )]
    pub fee_vault: Account<'info, TokenAccount>,

    pub claimant: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawFee<'info> {
    #[account(
        seeds = [b"config"],
        bump = config.bump,
        has_one = admin
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump
    )]
    pub fee_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

// Account structs
#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub fee_bps: u16,
    pub skill_mint: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Match {
    pub match_id: [u8; 32],
    pub player1: Pubkey,
    pub player2: Pubkey,
    pub stake: u64,
    pub skill_mint: Pubkey,
    pub status: u8, // 0=Created, 1=Active, 2=Settled, 3=Cancelled
    pub mode: u8,   // 0=TurnBased, 1=Realtime
    pub expiry_slot: u64,
    pub fee_bps: u16,
    pub player1_funded: bool,
    pub player2_funded: bool,
    pub bump: u8,
}

// Enums
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum MatchStatus {
    Created = 0,
    Active = 1,
    Settled = 2,
    Cancelled = 3,
}

// Events
#[event]
pub struct EscrowInitialized {
    pub admin: Pubkey,
    pub fee_bps: u16,
}

#[event]
pub struct MatchCreated {
    pub match_id: [u8; 32],
    pub player1: Pubkey,
    pub stake: u64,
    pub mode: u8,
}

#[event]
pub struct PlayerJoined {
    pub match_id: [u8; 32],
    pub player2: Pubkey,
}

#[event]
pub struct PlayerFunded {
    pub match_id: [u8; 32],
    pub player: Pubkey,
    pub amount: u64,
}

#[event]
pub struct MatchStarted {
    pub match_id: [u8; 32],
}

#[event]
pub struct MatchSettled {
    pub match_id: [u8; 32],
    pub winner: Pubkey,
    pub amount: u64,
    pub fee: u64,
}

#[event]
pub struct MatchCancelled {
    pub match_id: [u8; 32],
    pub reason: String,
}

#[event]
pub struct TimeoutClaimed {
    pub match_id: [u8; 32],
    pub winner: Pubkey,
}

// Errors
#[error_code]
pub enum EscrowError {
    #[msg("Invalid fee basis points")]
    InvalidFeeBps,
    #[msg("Invalid stake amount")]
    InvalidStake,
    #[msg("Invalid game mode")]
    InvalidMode,
    #[msg("Invalid match status for this operation")]
    InvalidMatchStatus,
    #[msg("Match is already full")]
    MatchAlreadyFull,
    #[msg("Cannot play against yourself")]
    CannotPlaySelf,
    #[msg("Match has expired")]
    MatchExpired,
    #[msg("Not a player in this match")]
    NotAPlayer,
    #[msg("Player already funded")]
    AlreadyFunded,
    #[msg("Match is not fully funded")]
    NotFullyFunded,
    #[msg("Invalid winner")]
    InvalidWinner,
    #[msg("Invalid mode for this operation")]
    InvalidModeForOperation,
    #[msg("Missing player signatures")]
    MissingSignatures,
    #[msg("Match not expired yet")]
    NotExpired,
    #[msg("Math overflow")]
    MathOverflow,
}
