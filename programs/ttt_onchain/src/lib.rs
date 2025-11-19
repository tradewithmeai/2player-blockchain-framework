//! # Tic-Tac-Toe On-Chain Game Program
//!
//! Provides trustless, turn-based Tic-Tac-Toe gameplay on Solana.
//!
//! ## Overview
//!
//! This program validates all moves on-chain, detects wins/draws, handles timeouts,
//! and automatically settles matches via CPI to the `skill_escrow` program.
//!
//! ## Game Flow
//!
//! ```text
//! 1. init_game    - Create game linked to active escrow match
//! 2. play         - Players alternate making moves (X starts)
//! 3. Game ends:
//!    - Win: 3 in a row (horizontal/vertical/diagonal)
//!    - Draw: Board full, no winner
//!    - Timeout: Player doesn't move within deadline
//! 4. resolve_if_complete - Settle match to winner via CPI
//! ```
//!
//! ## Security Model
//!
//! - **On-chain validation**: All moves verified on-chain (position, turn, timeout)
//! - **Timeout protection**: Players must move within `timeout_slots` or forfeit
//! - **CPI settlement**: Winner determined on-chain, settled via escrow CPI
//! - **Bounded timeouts**: Min 1 minute, max 6 hours per move
//! - **Immutable game state**: Games linked to specific escrow matches
//!
//! ## Board Layout
//!
//! ```text
//! 0 | 1 | 2
//! ---------
//! 3 | 4 | 5
//! ---------
//! 6 | 7 | 8
//! ```
//!
//! ## Example Usage
//!
//! ```typescript
//! // 1. Create and fund match in escrow program
//! await escrow.createMatch(matchId, stake, mode=0, expiry);
//! await escrow.fund(matchId, player1);
//! await escrow.fund(matchId, player2);
//!
//! // 2. Initialize game
//! await ttt.initGame(matchId, timeoutSlots=9000); // ~1 hour per move
//!
//! // 3. Play moves
//! await ttt.play(4);  // X plays center
//! await ttt.play(0);  // O plays top-left
//! // ... continue until win/draw/timeout
//!
//! // 4. Settle
//! await ttt.resolveIfComplete(); // CPI to escrow.settle_onchain
//! ```

use anchor_lang::prelude::*;
use skill_escrow::cpi::accounts::SettleOnchain;
use skill_escrow::program::SkillEscrow;
use skill_escrow::Match as EscrowMatch;

declare_id!("TicTa9999999999999999999999999999999999999");

// ============================================================================
// Security Constants
// ============================================================================

/// Minimum timeout per move (150 slots ≈ 1 minute at 400ms/slot)
const MIN_TIMEOUT_SLOTS: u64 = 150;

/// Maximum timeout per move (54,000 slots ≈ 6 hours)
const MAX_TIMEOUT_SLOTS: u64 = 54_000;

/// Classic SPL Token program ID
const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

// ============================================================================
// Program Instructions
// ============================================================================

#[program]
pub mod ttt_onchain {
    use super::*;

    /// Initialize a new Tic-Tac-Toe game linked to an active escrow match.
    ///
    /// # Arguments
    ///
    /// * `match_id` - The match ID from skill_escrow (must be Active status)
    /// * `timeout_slots` - Number of slots a player has to make each move
    ///
    /// # Security
    ///
    /// - Validates match is in Active status
    /// - Enforces timeout bounds (MIN_TIMEOUT_SLOTS to MAX_TIMEOUT_SLOTS)
    /// - Uses checked arithmetic for deadline calculation
    /// - Game PDA derived from match_id prevents duplicate games
    ///
    /// # Errors
    ///
    /// - `MatchNotActive`: Match not in Active status
    /// - `InvalidTimeout`: timeout_slots out of bounds
    /// - `MathOverflow`: Deadline calculation overflow
    ///
    /// # Events
    ///
    /// - `GameInitialized`: Emitted with match_id and player addresses
    ///
    /// # Example
    ///
    /// ```typescript
    /// await program.methods
    ///   .initGame(matchId, 9000) // ~1 hour per move
    ///   .accounts({
    ///     game: gamePda,
    ///     matchAccount: matchPda,
    ///     payer: player1.publicKey,
    ///   })
    ///   .rpc();
    /// ```
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

        // Validate timeout bounds
        require!(
            timeout_slots >= MIN_TIMEOUT_SLOTS && timeout_slots <= MAX_TIMEOUT_SLOTS,
            GameError::InvalidTimeout
        );

        // Initialize game state
        game.match_id = match_id;
        game.match_pda = ctx.accounts.match_account.key();
        game.player_x = match_account.player1;
        game.player_o = match_account.player2;
        game.board = [0; 9]; // 0 = empty, 1 = X, 2 = O
        game.current_turn = 1; // X starts
        game.winner = None;
        game.status = GameStatus::Active as u8;
        game.move_count = 0;
        game.timeout_slots = timeout_slots;
        game.bump = ctx.bumps.game;

        // Set initial deadline using checked arithmetic
        game.deadline_slot = clock
            .slot
            .checked_add(timeout_slots)
            .ok_or(GameError::MathOverflow)?;

        emit!(GameInitialized {
            match_id,
            player_x: game.player_x,
            player_o: game.player_o,
            timeout_slots,
        });

        Ok(())
    }

    /// Make a move on the game board.
    ///
    /// # Arguments
    ///
    /// * `position` - Board position (0-8, see board layout in module docs)
    ///
    /// # Security
    ///
    /// - Validates game is Active
    /// - Validates position is in bounds (0-8)
    /// - Validates position is empty
    /// - Validates correct player's turn
    /// - Validates move made before deadline
    /// - Uses checked arithmetic for next deadline
    ///
    /// # Errors
    ///
    /// - `GameNotActive`: Game already finished
    /// - `InvalidPosition`: Position >= 9
    /// - `PositionOccupied`: Square already has a piece
    /// - `NotYourTurn`: Wrong player attempting move
    /// - `MoveTimeout`: Current slot > deadline_slot
    /// - `MathOverflow`: Next deadline calculation overflow
    ///
    /// # Events
    ///
    /// - `MoveMade`: Emitted for every valid move
    /// - `GameFinished`: Emitted when game ends (win or draw)
    ///
    /// # Example
    ///
    /// ```typescript
    /// // X plays center (position 4)
    /// await program.methods
    ///   .play(4)
    ///   .accounts({
    ///     game: gamePda,
    ///     player: playerX.publicKey,
    ///   })
    ///   .signers([playerX])
    ///   .rpc();
    /// ```
    pub fn play(ctx: Context<Play>, position: u8) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let clock = Clock::get()?;

        // Validate game is active
        require!(
            game.status == GameStatus::Active as u8,
            GameError::GameNotActive
        );

        // Validate position
        require!(position < 9, GameError::InvalidPosition);
        require!(
            game.board[position as usize] == 0,
            GameError::PositionOccupied
        );

        // Check timeout BEFORE making move
        require!(clock.slot <= game.deadline_slot, GameError::MoveTimeout);

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

        // Update deadline for next move using checked arithmetic
        game.deadline_slot = clock
            .slot
            .checked_add(game.timeout_slots)
            .ok_or(GameError::MathOverflow)?;

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
            // Draw - board full with no winner
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

    /// Resolve game and settle match if finished.
    ///
    /// This instruction calls the escrow program via CPI to settle funds to the winner.
    ///
    /// # Security
    ///
    /// - Validates game is Finished
    /// - Validates winner is one of the match players
    /// - Validates token program ID
    /// - CPI to escrow handles all fund transfers
    ///
    /// # Errors
    ///
    /// - `GameNotFinished`: Game still Active
    /// - `InvalidTokenProgram`: Wrong token program provided
    /// - Plus any errors from escrow.settle_onchain CPI
    ///
    /// # Events
    ///
    /// - `MatchSettled`: Emitted after successful CPI settlement
    ///
    /// # Draw Handling
    ///
    /// Currently logs a message for draws. Future implementations should
    /// call escrow with draw logic (e.g., refund both players minus fees).
    ///
    /// # Example
    ///
    /// ```typescript
    /// await program.methods
    ///   .resolveIfComplete()
    ///   .accounts({
    ///     game: gamePda,
    ///     matchAccount: matchPda,
    ///     escrowVault: vaultPda,
    ///     winnerAta: winnerTokenAccount,
    ///     feeVault: feeVaultPda,
    ///     skillEscrowProgram: escrowProgramId,
    ///     tokenProgram: TOKEN_PROGRAM_ID,
    ///   })
    ///   .rpc();
    /// ```
    pub fn resolve_if_complete(ctx: Context<Resolve>) -> Result<()> {
        let game = &ctx.accounts.game;

        // Validate game is finished
        require!(
            game.status == GameStatus::Finished as u8,
            GameError::GameNotFinished
        );

        // Validate token program
        require!(
            ctx.accounts.token_program.key().to_string() == SPL_TOKEN_PROGRAM_ID,
            GameError::InvalidTokenProgram
        );

        if let Some(winner) = game.winner {
            // Settle to winner via CPI to escrow
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
            // TODO: Implement draw settlement via escrow CPI
            // Options: refund both players minus fees, or split pot 50/50
            msg!("Game ended in draw - implement draw settlement logic in future version");
        }

        Ok(())
    }

    /// Claim timeout win if opponent didn't move within deadline.
    ///
    /// The non-timeout player (whose turn it ISN'T) can claim victory if the
    /// current player fails to move before the deadline.
    ///
    /// # Security
    ///
    /// - Validates game is Active
    /// - Validates deadline passed (slot > deadline_slot)
    /// - Validates claimant is the non-timeout player
    /// - Validates token program ID
    /// - Automatically settles to timeout winner via CPI
    ///
    /// # Errors
    ///
    /// - `GameNotActive`: Game already finished
    /// - `NotTimedOut`: Deadline not reached yet
    /// - `NotYourTimeout`: Claimant is the timeout player (can't claim own timeout)
    /// - `InvalidTokenProgram`: Wrong token program provided
    ///
    /// # Events
    ///
    /// - `GameFinished`: Emitted with reason="Timeout"
    /// - `MatchSettled`: Emitted after CPI settlement
    ///
    /// # Example
    ///
    /// ```typescript
    /// // It's X's turn but they timed out, so O claims victory
    /// await program.methods
    ///   .timeout()
    ///   .accounts({
    ///     game: gamePda,
    ///     matchAccount: matchPda,
    ///     escrowVault: vaultPda,
    ///     winnerAta: playerOTokenAccount,
    ///     feeVault: feeVaultPda,
    ///     claimant: playerO.publicKey,
    ///     skillEscrowProgram: escrowProgramId,
    ///     tokenProgram: TOKEN_PROGRAM_ID,
    ///   })
    ///   .signers([playerO])
    ///   .rpc();
    /// ```
    pub fn timeout(ctx: Context<Timeout>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let clock = Clock::get()?;

        // Validate game is active
        require!(
            game.status == GameStatus::Active as u8,
            GameError::GameNotActive
        );

        // Validate timeout occurred
        require!(clock.slot > game.deadline_slot, GameError::NotTimedOut);

        // Validate token program
        require!(
            ctx.accounts.token_program.key().to_string() == SPL_TOKEN_PROGRAM_ID,
            GameError::InvalidTokenProgram
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

        // Settle to winner via CPI to escrow
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if there's a winner on the board.
///
/// # Arguments
///
/// * `board` - The 3x3 game board represented as [u8; 9]
///
/// # Returns
///
/// * `Some(1)` - X wins
/// * `Some(2)` - O wins
/// * `None` - No winner yet
///
/// # Algorithm
///
/// Checks all 8 winning combinations:
/// - 3 rows (0-1-2, 3-4-5, 6-7-8)
/// - 3 columns (0-3-6, 1-4-7, 2-5-8)
/// - 2 diagonals (0-4-8, 2-4-6)
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

// ============================================================================
// Account Validation Structs
// ============================================================================

/// Initialize a new game
#[derive(Accounts)]
#[instruction(match_id: [u8; 32])]
pub struct InitGame<'info> {
    /// Game account - PDA derived from match_id
    #[account(
        init,
        payer = payer,
        space = 8 + Game::INIT_SPACE,
        seeds = [b"game", match_id.as_ref()],
        bump
    )]
    pub game: Account<'info, Game>,

    /// Match account from skill_escrow - must be Active
    #[account(
        seeds = [b"match", match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    pub match_account: Account<'info, EscrowMatch>,

    /// Payer for game account creation
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program for account creation
    pub system_program: Program<'info, System>,
}

/// Make a move
#[derive(Accounts)]
pub struct Play<'info> {
    /// Game account - must be Active
    #[account(
        mut,
        seeds = [b"game", game.match_id.as_ref()],
        bump = game.bump
    )]
    pub game: Account<'info, Game>,

    /// Player making the move - verified against current_turn
    pub player: Signer<'info>,
}

/// Resolve finished game
#[derive(Accounts)]
pub struct Resolve<'info> {
    /// Game account - must be Finished
    #[account(
        seeds = [b"game", game.match_id.as_ref()],
        bump = game.bump
    )]
    pub game: Account<'info, Game>,

    /// Match account from escrow - for CPI
    #[account(
        mut,
        seeds = [b"match", game.match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    pub match_account: Account<'info, EscrowMatch>,

    /// Escrow vault - holds staked SKILL tokens
    #[account(
        mut,
        seeds = [b"escrow_vault", game.match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    /// CHECK: Validated by skill_escrow in CPI
    pub escrow_vault: AccountInfo<'info>,

    /// Winner's token account - receives payout
    #[account(mut)]
    /// CHECK: Validated by skill_escrow in CPI
    pub winner_ata: AccountInfo<'info>,

    /// Fee vault - receives platform fee
    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump,
        seeds::program = skill_escrow::ID
    )]
    /// CHECK: Validated by skill_escrow in CPI
    pub fee_vault: AccountInfo<'info>,

    /// Escrow program for CPI
    pub skill_escrow_program: Program<'info, SkillEscrow>,

    /// Token program - must be classic SPL
    /// CHECK: Validated in instruction
    pub token_program: AccountInfo<'info>,
}

/// Claim timeout win
#[derive(Accounts)]
pub struct Timeout<'info> {
    /// Game account - must be Active and past deadline
    #[account(
        mut,
        seeds = [b"game", game.match_id.as_ref()],
        bump = game.bump
    )]
    pub game: Account<'info, Game>,

    /// Match account from escrow - for CPI
    #[account(
        mut,
        seeds = [b"match", game.match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    pub match_account: Account<'info, EscrowMatch>,

    /// Escrow vault - holds staked SKILL tokens
    #[account(
        mut,
        seeds = [b"escrow_vault", game.match_id.as_ref()],
        bump,
        seeds::program = skill_escrow::ID
    )]
    /// CHECK: Validated by skill_escrow in CPI
    pub escrow_vault: AccountInfo<'info>,

    /// Winner's token account - receives payout
    #[account(mut)]
    /// CHECK: Validated by skill_escrow in CPI
    pub winner_ata: AccountInfo<'info>,

    /// Fee vault - receives platform fee
    #[account(
        mut,
        seeds = [b"fee_vault"],
        bump,
        seeds::program = skill_escrow::ID
    )]
    /// CHECK: Validated by skill_escrow in CPI
    pub fee_vault: AccountInfo<'info>,

    /// Player claiming timeout win - must be non-timeout player
    pub claimant: Signer<'info>,

    /// Escrow program for CPI
    pub skill_escrow_program: Program<'info, SkillEscrow>,

    /// Token program - must be classic SPL
    /// CHECK: Validated in instruction
    pub token_program: AccountInfo<'info>,
}

// ============================================================================
// State Accounts
// ============================================================================

/// Game state account
///
/// Stores the complete state of a Tic-Tac-Toe game, including board position,
/// turn tracking, timeout deadline, and winner.
#[account]
#[derive(InitSpace)]
pub struct Game {
    /// Match ID from skill_escrow (used as seed)
    pub match_id: [u8; 32],

    /// Match PDA address from escrow program
    pub match_pda: Pubkey,

    /// Player X (always player1 from match)
    pub player_x: Pubkey,

    /// Player O (always player2 from match)
    pub player_o: Pubkey,

    /// Board state: 0 = empty, 1 = X, 2 = O
    pub board: [u8; 9],

    /// Current turn: 1 = X, 2 = O
    pub current_turn: u8,

    /// Winner address (None if draw or game not finished)
    #[max_len(1)]
    pub winner: Option<Pubkey>,

    /// Game status: 0 = Active, 1 = Finished
    pub status: u8,

    /// Number of moves made (0-9)
    pub move_count: u8,

    /// Slot number when current player must move by
    pub deadline_slot: u64,

    /// Number of slots allowed per move
    pub timeout_slots: u64,

    /// PDA bump seed
    pub bump: u8,
}

// ============================================================================
// Enums
// ============================================================================

/// Game status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum GameStatus {
    /// Game is in progress
    Active = 0,
    /// Game is finished (win, draw, or timeout)
    Finished = 1,
}

// ============================================================================
// Events
// ============================================================================

/// Emitted when a new game is initialized
#[event]
pub struct GameInitialized {
    /// Match ID
    pub match_id: [u8; 32],
    /// Player X address
    pub player_x: Pubkey,
    /// Player O address
    pub player_o: Pubkey,
    /// Timeout slots per move
    pub timeout_slots: u64,
}

/// Emitted when a player makes a move
#[event]
pub struct MoveMade {
    /// Match ID
    pub match_id: [u8; 32],
    /// Player who made the move
    pub player: Pubkey,
    /// Board position (0-8)
    pub position: u8,
    /// Piece placed (1=X, 2=O)
    pub piece: u8,
}

/// Emitted when game finishes
#[event]
pub struct GameFinished {
    /// Match ID
    pub match_id: [u8; 32],
    /// Winner address (None for draw)
    pub winner: Option<Pubkey>,
    /// Reason: "Win", "Draw", or "Timeout"
    pub reason: String,
}

/// Emitted when match is settled via CPI
#[event]
pub struct MatchSettled {
    /// Match ID
    pub match_id: [u8; 32],
    /// Winner address
    pub winner: Pubkey,
}

// ============================================================================
// Errors
// ============================================================================

#[error_code]
pub enum GameError {
    /// 6000 - Match is not in Active status
    #[msg("Match is not active. Ensure match is fully funded before creating game")]
    MatchNotActive,

    /// 6001 - Game is not active
    #[msg("Game is not active. Cannot make moves on finished game")]
    GameNotActive,

    /// 6002 - Invalid board position
    #[msg("Invalid position. Must be 0-8 (see board layout in docs)")]
    InvalidPosition,

    /// 6003 - Position already has a piece
    #[msg("Position already occupied. Choose an empty square")]
    PositionOccupied,

    /// 6004 - Wrong player attempting move
    #[msg("Not your turn. Wait for opponent to move")]
    NotYourTurn,

    /// 6005 - Game not finished yet
    #[msg("Game is not finished. Cannot resolve until game ends")]
    GameNotFinished,

    /// 6006 - Player failed to move in time
    #[msg("Move timeout. Player exceeded deadline")]
    MoveTimeout,

    /// 6007 - Deadline not reached yet
    #[msg("Game has not timed out yet. Wait until deadline passes")]
    NotTimedOut,

    /// 6008 - Wrong player claiming timeout
    #[msg("You cannot claim this timeout. Only non-timeout player can claim")]
    NotYourTimeout,

    /// 6009 - Timeout slots out of bounds
    #[msg("Invalid timeout. Must be between 1 minute and 6 hours (150 - 54,000 slots)")]
    InvalidTimeout,

    /// 6010 - Arithmetic overflow
    #[msg("Mathematical operation caused an overflow")]
    MathOverflow,

    /// 6011 - Wrong token program
    #[msg("Invalid token program. Must use classic SPL Token program")]
    InvalidTokenProgram,
}
