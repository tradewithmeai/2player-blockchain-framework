use anchor_lang::prelude::*;
use skill_escrow::cpi::accounts::SettleOnchain;
use skill_escrow::program::SkillEscrow;
use skill_escrow::Match as EscrowMatch;

declare_id!("TicTa9999999999999999999999999999999999999");

/// On-chain Tic-Tac-Toe game program
///
/// Provides trustless turn-based gameplay:
/// - Validates all moves on-chain
/// - Detects wins and draws
/// - Handles timeouts
/// - Automatically settles to winner via CPI to skill_escrow
#[program]
pub mod ttt_onchain {
    use super::*;

    /// Initialize a new game linked to a match
    ///
    /// # Arguments
    /// * `match_id` - The match ID from skill_escrow
    /// * `timeout_slots` - Number of slots a player has to make a move
    pub fn init_game(
        ctx: Context<InitGame>,
        match_id: [u8; 32],
        timeout_slots: u64,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let clock = Clock::get()?;

        // Verify match is active
        let match_account = &ctx.accounts.match_account;
        require!(
            match_account.status == 1, // Active
            GameError::MatchNotActive
        );

        game.match_id = match_id;
        game.match_pda = ctx.accounts.match_account.key();
        game.player_x = match_account.player1;
        game.player_o = match_account.player2;
        game.board = [0; 9]; // 0 = empty, 1 = X, 2 = O
        game.current_turn = 1; // X starts
        game.winner = None;
        game.status = GameStatus::Active as u8;
        game.move_count = 0;
        game.deadline_slot = clock.slot.checked_add(timeout_slots).unwrap();
        game.timeout_slots = timeout_slots;
        game.bump = ctx.bumps.game;

        emit!(GameInitialized {
            match_id,
            player_x: game.player_x,
            player_o: game.player_o,
        });

        Ok(())
    }

    /// Make a move
    ///
    /// # Arguments
    /// * `position` - Board position (0-8)
    pub fn play(ctx: Context<Play>, position: u8) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let clock = Clock::get()?;

        require!(
            game.status == GameStatus::Active as u8,
            GameError::GameNotActive
        );
        require!(position < 9, GameError::InvalidPosition);
        require!(game.board[position as usize] == 0, GameError::PositionOccupied);

        // Check timeout
        require!(
            clock.slot <= game.deadline_slot,
            GameError::MoveTimeout
        );

        // Verify correct player
        let player = ctx.accounts.player.key();
        let expected_player = if game.current_turn == 1 {
            game.player_x
        } else {
            game.player_o
        };

        require!(player == expected_player, GameError::NotYourTurn);

        // Make move
        game.board[position as usize] = game.current_turn;
        game.move_count += 1;

        // Update deadline for next move
        game.deadline_slot = clock.slot.checked_add(game.timeout_slots).unwrap();

        emit!(MoveMade {
            match_id: game.match_id,
            player,
            position,
            piece: game.current_turn,
        });

        // Check for winner
        if let Some(winner_piece) = check_winner(&game.board) {
            game.status = GameStatus::Finished as u8;
            game.winner = Some(if winner_piece == 1 {
                game.player_x
            } else {
                game.player_o
            });

            emit!(GameFinished {
                match_id: game.match_id,
                winner: game.winner,
                reason: "Win".to_string(),
            });
        } else if game.move_count == 9 {
            // Draw
            game.status = GameStatus::Finished as u8;
            game.winner = None;

            emit!(GameFinished {
                match_id: game.match_id,
                winner: None,
                reason: "Draw".to_string(),
            });
        } else {
            // Toggle turn
            game.current_turn = if game.current_turn == 1 { 2 } else { 1 };
        }

        Ok(())
    }

    /// Resolve game and settle match if finished
    pub fn resolve_if_complete(ctx: Context<Resolve>) -> Result<()> {
        let game = &ctx.accounts.game;

        require!(
            game.status == GameStatus::Finished as u8,
            GameError::GameNotFinished
        );

        if let Some(winner) = game.winner {
            // Settle to winner via CPI
            let cpi_program = ctx.accounts.skill_escrow_program.to_account_info();
            let cpi_accounts = SettleOnchain {
                match_account: ctx.accounts.match_account.to_account_info(),
                escrow_vault: ctx.accounts.escrow_vault.to_account_info(),
                winner_ata: ctx.accounts.winner_ata.to_account_info(),
                fee_vault: ctx.accounts.fee_vault.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            };
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

            skill_escrow::cpi::settle_onchain(cpi_ctx, winner)?;

            emit!(MatchSettled {
                match_id: game.match_id,
                winner,
            });
        } else {
            // Draw - split the pot (handled by escrow program with draw logic)
            // For now, we can settle to player1 or implement draw refund logic
            // This is a design decision for the team
            msg!("Game ended in draw - implement draw settlement logic");
        }

        Ok(())
    }

    /// Claim timeout win if opponent didn't move in time
    pub fn timeout(ctx: Context<Timeout>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let clock = Clock::get()?;

        require!(
            game.status == GameStatus::Active as u8,
            GameError::GameNotActive
        );
        require!(
            clock.slot > game.deadline_slot,
            GameError::NotTimedOut
        );

        let claimant = ctx.accounts.claimant.key();

        // Winner is the player whose turn it ISN'T (the non-timeout player)
        let winner = if game.current_turn == 1 {
            // It's X's turn but they timed out, so O wins
            require!(claimant == game.player_o, GameError::NotYourTimeout);
            game.player_o
        } else {
            // It's O's turn but they timed out, so X wins
            require!(claimant == game.player_x, GameError::NotYourTimeout);
            game.player_x
        };

        game.status = GameStatus::Finished as u8;
        game.winner = Some(winner);

        emit!(GameFinished {
            match_id: game.match_id,
            winner: Some(winner),
            reason: "Timeout".to_string(),
        });

        // Settle to winner via CPI
        let cpi_program = ctx.accounts.skill_escrow_program.to_account_info();
        let cpi_accounts = SettleOnchain {
            match_account: ctx.accounts.match_account.to_account_info(),
            escrow_vault: ctx.accounts.escrow_vault.to_account_info(),
            winner_ata: ctx.accounts.winner_ata.to_account_info(),
            fee_vault: ctx.accounts.fee_vault.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        skill_escrow::cpi::settle_onchain(cpi_ctx, winner)?;

        emit!(MatchSettled {
            match_id: game.match_id,
            winner,
        });

        Ok(())
    }
}

// Check for winner
fn check_winner(board: &[u8; 9]) -> Option<u8> {
    // Winning combinations
    let lines = [
        [0, 1, 2], // Rows
        [3, 4, 5],
        [6, 7, 8],
        [0, 3, 6], // Columns
        [1, 4, 7],
        [2, 5, 8],
        [0, 4, 8], // Diagonals
        [2, 4, 6],
    ];

    for line in lines.iter() {
        let [a, b, c] = *line;
        if board[a] != 0 && board[a] == board[b] && board[b] == board[c] {
            return Some(board[a]);
        }
    }

    None
}

// Context structs
#[derive(Accounts)]
#[instruction(match_id: [u8; 32])]
pub struct InitGame<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Game::INIT_SPACE,
        seeds = [b"game", match_id.as_ref()],
        bump
    )]
    pub game: Account<'info, Game>,

    #[account(
        seeds = [b"match", match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    pub match_account: Account<'info, EscrowMatch>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Play<'info> {
    #[account(
        mut,
        seeds = [b"game", game.match_id.as_ref()],
        bump = game.bump
    )]
    pub game: Account<'info, Game>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct Resolve<'info> {
    #[account(
        seeds = [b"game", game.match_id.as_ref()],
        bump = game.bump
    )]
    pub game: Account<'info, Game>,

    #[account(
        mut,
        seeds = [b"match", game.match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    pub match_account: Account<'info, EscrowMatch>,

    #[account(
        mut,
        seeds = [b"escrow_vault", game.match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    /// CHECK: Validated by skill_escrow
    pub escrow_vault: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Validated by skill_escrow
    pub winner_ata: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump,
        seeds::program = skill_escrow::ID
    )]
    /// CHECK: Validated by skill_escrow
    pub fee_vault: AccountInfo<'info>,

    pub skill_escrow_program: Program<'info, SkillEscrow>,

    /// CHECK: Token program
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Timeout<'info> {
    #[account(
        mut,
        seeds = [b"game", game.match_id.as_ref()],
        bump = game.bump
    )]
    pub game: Account<'info, Game>,

    #[account(
        mut,
        seeds = [b"match", game.match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    pub match_account: Account<'info, EscrowMatch>,

    #[account(
        mut,
        seeds = [b"escrow_vault", game.match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    /// CHECK: Validated by skill_escrow
    pub escrow_vault: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Validated by skill_escrow
    pub winner_ata: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump,
        seeds::program = skill_escrow::ID
    )]
    /// CHECK: Validated by skill_escrow
    pub fee_vault: AccountInfo<'info>,

    pub claimant: Signer<'info>,

    pub skill_escrow_program: Program<'info, SkillEscrow>,

    /// CHECK: Token program
    pub token_program: AccountInfo<'info>,
}

// Account structs
#[account]
#[derive(InitSpace)]
pub struct Game {
    pub match_id: [u8; 32],
    pub match_pda: Pubkey,
    pub player_x: Pubkey,
    pub player_o: Pubkey,
    pub board: [u8; 9],
    pub current_turn: u8, // 1 = X, 2 = O
    #[max_len(1)]
    pub winner: Option<Pubkey>,
    pub status: u8, // 0 = Active, 1 = Finished
    pub move_count: u8,
    pub deadline_slot: u64,
    pub timeout_slots: u64,
    pub bump: u8,
}

// Enums
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum GameStatus {
    Active = 0,
    Finished = 1,
}

// Events
#[event]
pub struct GameInitialized {
    pub match_id: [u8; 32],
    pub player_x: Pubkey,
    pub player_o: Pubkey,
}

#[event]
pub struct MoveMade {
    pub match_id: [u8; 32],
    pub player: Pubkey,
    pub position: u8,
    pub piece: u8,
}

#[event]
pub struct GameFinished {
    pub match_id: [u8; 32],
    pub winner: Option<Pubkey>,
    pub reason: String,
}

#[event]
pub struct MatchSettled {
    pub match_id: [u8; 32],
    pub winner: Pubkey,
}

// Errors
#[error_code]
pub enum GameError {
    #[msg("Match is not active")]
    MatchNotActive,
    #[msg("Game is not active")]
    GameNotActive,
    #[msg("Invalid position (must be 0-8)")]
    InvalidPosition,
    #[msg("Position already occupied")]
    PositionOccupied,
    #[msg("Not your turn")]
    NotYourTurn,
    #[msg("Game is not finished")]
    GameNotFinished,
    #[msg("Move timeout")]
    MoveTimeout,
    #[msg("Game has not timed out yet")]
    NotTimedOut,
    #[msg("You cannot claim this timeout")]
    NotYourTimeout,
}
