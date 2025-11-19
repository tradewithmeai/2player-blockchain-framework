# Error Code Registry

Complete reference of all error codes in the Skill Gaming platform.

## How to Use This Guide

When you encounter an error:
1. Find the error code number (e.g., `6004`)
2. Read the cause and resolution
3. Check the example if provided

## Format

```
Error Code | Program | Error Name | Message
```

---

## skill_treasury Errors

### Error 6000: `InvalidRate`

**Message:** "Exchange rate must be greater than zero"

**Cause:**
Attempting to initialize or update the treasury with a rate of 0.

**Resolution:**
- Use a positive exchange rate (e.g., 1000 for 1 SOL = 1000 SKILL)
- Typical rates range from 100 to 10,000

**When it occurs:**
- `initialize` instruction with `rate = 0`
- `update_config` instruction with `rate = Some(0)`

---

### Error 6001: `InvalidFeeBps`

**Message:** "Fee basis points must be less than 10000 (100%)"

**Cause:**
Fee basis points >= 10000, which would be >= 100%.

**Resolution:**
- Use a fee less than 10000
- Example: 100 = 1%, 500 = 5%, 1000 = 10%

**When it occurs:**
- `initialize` instruction with invalid fee
- `update_config` instruction with invalid fee

---

### Error 6002: `InvalidAmount`

**Message:** "Amount must be greater than zero"

**Cause:**
Calculation resulted in zero tokens/lamports, usually due to very small amounts.

**Resolution:**
- Increase the amount being deposited or redeemed
- Check that `(lamports × rate) / 1e9 > 0` for deposits
- Check that `(skill_amount × 1e9) / rate > 0` for redemptions

**When it occurs:**
- `deposit_sol_and_mint_skill` with amount too small relative to rate
- `redeem_skill_for_sol` with amount too small relative to rate

---

### Error 6003: `MathOverflow`

**Message:** "Mathematical operation caused an overflow"

**Cause:**
Arithmetic operation exceeded u64 maximum value.

**Resolution:**
- Use smaller amounts
- This is extremely rare with reasonable values

**When it occurs:**
- Any instruction performing checked arithmetic
- Most likely with unrealistic test values

---

### Error 6004: `InsufficientLiquidity`

**Message:** "Insufficient liquidity in treasury vault. Please try a smaller amount or wait for more deposits."

**Cause:**
Treasury vault doesn't have enough SOL to fulfill the redemption.

**Resolution:**
1. Check treasury balance: `solana balance <treasury_vault_address>`
2. Reduce redemption amount
3. Wait for more users to deposit SOL
4. As admin, deposit SOL directly to vault if needed

**Example transaction:**
```
Error: Transaction simulation failed: Error processing Instruction 0:
custom program error: 0x1774 (6004 in decimal)
```

**Common scenarios:**
- Large redemption relative to treasury size
- Many users redeeming simultaneously
- Low initial treasury funding

---

### Error 6005: `DepositTooSmall`

**Message:** "Deposit too small. Minimum deposit is 0.001 SOL"

**Cause:**
Attempting to deposit less than `MIN_DEPOSIT_LAMPORTS` (1,000,000 lamports = 0.001 SOL).

**Resolution:**
- Deposit at least 0.001 SOL (1,000,000 lamports)

**Why this exists:**
Prevents dust attacks and ensures economically meaningful transactions.

---

### Error 6006: `RedemptionTooSmall`

**Message:** "Redemption too small. Minimum redemption is 1 SKILL"

**Cause:**
Attempting to redeem less than `MIN_REDEMPTION_SKILL` (1,000,000 atomic units = 1 SKILL).

**Resolution:**
- Redeem at least 1 SKILL (1,000,000 atomic units)

**Why this exists:**
Prevents dust and ensures cost-effective transactions.

---

### Error 6007: `Paused`

**Message:** "Treasury operations are currently paused"

**Cause:**
Admin has paused treasury operations via `set_pause`.

**Resolution:**
- Wait for admin to unpause operations
- Contact platform administrators
- Check announcements for maintenance schedule

**When it occurs:**
- `deposit_sol_and_mint_skill` while paused
- `redeem_skill_for_sol` while paused

**Note:** Admin can still update config and unpause while paused.

---

### Error 6008: `InvalidMint`

**Message:** "Invalid mint. Must use configured SKILL mint."

**Cause:**
Provided mint address doesn't match the configured `skill_mint` in treasury config.

**Resolution:**
- Use the correct SKILL mint address
- Query config account to get correct mint: `config.skill_mint`
- Don't attempt to use alternative mints

**Security:**
This prevents attacks using fake SKILL tokens.

---

### Error 6009: `InvalidTokenProgram`

**Message:** "Invalid token program. Must use classic SPL Token program."

**Cause:**
Attempting to use Token-2022 or another token program instead of classic SPL.

**Resolution:**
- Use `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` (classic SPL)
- Don't use Token-2022 program

**Why this exists:**
Platform designed for classic SPL tokens for compatibility.

---

### Error 6010: `RateChangeTooLarge`

**Message:** "Rate change too large. Maximum 10x change per update."

**Cause:**
Attempting to change rate by more than 10x in a single update.

**Resolution:**
- Change rate gradually (max 10x per update)
- Example: If rate is 1000, new rate must be between 100 and 10000

**Why this exists:**
Prevents accidental admin errors that could destabilize the treasury.

---

### Error 6011: `InsufficientBalance`

**Message:** "Insufficient SKILL token balance"

**Cause:**
User's token account doesn't have enough SKILL tokens for redemption.

**Resolution:**
1. Check token balance
2. Reduce redemption amount
3. Ensure you're checking the correct token account

---

## skill_escrow Errors

### Error 6000: `InvalidFeeBps`

**Message:** "Invalid fee basis points"

**Cause:**
Fee >= 10000 (100%).

**Resolution:**
- Use fee < 10000
- Typical fees: 100-500 (1-5%)

---

### Error 6001: `StakeTooSmall`

**Message:** "Stake too small. Minimum stake is 1 SKILL (1,000,000 atomic units)"

**Cause:**
Attempting to create a match with stake < MIN_STAKE_AMOUNT (1,000,000 atomic units = 1 SKILL).

**Resolution:**
- Use a stake of at least 1 SKILL (1,000,000 atomic units)
- Typical stakes: 1-1000 SKILL for normal gameplay

**Why this exists:**
Prevents dust attacks and ensures economically meaningful matches.

**When it occurs:**
- `create_match` instruction with stake < 1,000,000

---

### Error 6002: `InvalidMode`

**Message:** "Invalid game mode"

**Cause:**
Mode value > 1 (valid modes: 0 = TurnBased, 1 = Realtime).

**Resolution:**
- Use mode 0 for turn-based games
- Use mode 1 for realtime games

---

### Error 6003: `InvalidMatchStatus`

**Message:** "Invalid match status for this operation"

**Cause:**
Operation not allowed in current match state.

**Resolution:**
- Check match status before operation
- Example: Can't fund a match that's already Active or Settled

**Status flow:**
Created → Active → Settled/Cancelled

---

### Error 6004: `MatchAlreadyFull`

**Message:** "Match is already full"

**Cause:**
Attempting to join a match that already has player2.

**Resolution:**
- Create a new match
- Join a different match
- Wait for new matches to be created

---

### Error 6005: `CannotPlaySelf`

**Message:** "Cannot play against yourself"

**Cause:**
Same wallet trying to be both player1 and player2.

**Resolution:**
- Use a different wallet to join
- Have a friend join your match

---

### Error 6006: `MatchExpired`

**Message:** "Match has expired"

**Cause:**
Current slot > match expiry_slot before both players funded.

**Resolution:**
- Create a new match
- Set longer expiry when creating matches

---

### Error 6007: `NotAPlayer`

**Message:** "Not a player in this match"

**Cause:**
Signer is neither player1 nor player2.

**Resolution:**
- Use the correct wallet
- Join the match first if you're player2

---

### Error 6008: `AlreadyFunded`

**Message:** "Player already funded"

**Cause:**
Player trying to fund twice.

**Resolution:**
- Each player funds exactly once
- Wait for opponent to fund

---

### Error 6009: `NotFullyFunded`

**Message:** "Match is not fully funded"

**Cause:**
Attempting to start match before both players funded.

**Resolution:**
- Wait for both players to call `fund`
- Check `match.player1_funded && match.player2_funded`

---

### Error 6010: `InvalidWinner`

**Message:** "Invalid winner"

**Cause:**
Winner address is neither player1 nor player2.

**Resolution:**
- Winner must be one of the match players
- Verify winner address

---

### Error 6011: `InvalidModeForOperation`

**Message:** "Invalid mode for this operation"

**Cause:**
Using wrong settlement method for game mode.

**Resolution:**
- Use `settle_onchain` for turn-based (mode 0)
- Use `settle_with_signatures` for realtime (mode 1)

---

### Error 6012: `MissingSignatures`

**Message:** "Missing player signatures"

**Cause:**
Ed25519 signature verification instructions not found or invalid.

**Resolution:**
1. Include TWO ed25519 verify instructions before settlement
2. Both players must sign the digest
3. Use `Ed25519Program.createInstructionWithPublicKey()` for each

**Example (correct):**
```typescript
const tx = new Transaction()
  .add(Ed25519Program.createInstructionWithPublicKey({
    publicKey: player1.publicKey.toBytes(),
    message: digest,
    signature: player1Signature
  }))
  .add(Ed25519Program.createInstructionWithPublicKey({
    publicKey: player2.publicKey.toBytes(),
    message: digest,
    signature: player2Signature
  }))
  .add(settleInstruction);
```

---

### Error 6013: `NotExpired`

**Message:** "Match not expired yet"

**Cause:**
Attempting timeout claim before expiry_slot reached.

**Resolution:**
- Wait for match to expire
- Check `Clock.slot > match.expiry_slot`

---

### Error 6014: `MathOverflow`

**Message:** "Mathematical operation caused an overflow"

**Cause:**
Arithmetic overflow in fee/settlement calculations.

**Resolution:**
- Use reasonable stake amounts
- This is rare with normal values

---

### Error 6015: `Paused`

**Message:** "Match creation is currently paused by admin"

**Cause:**
Admin has paused match creation via `set_pause`.

**Resolution:**
- Wait for admin to unpause operations
- Contact platform administrators
- Check announcements for maintenance schedule

**When it occurs:**
- `create_match` instruction while config.paused = true

**Note:** Admin can still update config and unpause while paused. Existing matches can still be funded and settled.

---

### Error 6016: `InvalidMint`

**Message:** "Invalid mint. Must use configured SKILL mint"

**Cause:**
Provided mint address doesn't match the configured `skill_mint` in escrow config.

**Resolution:**
- Use the correct SKILL mint address
- Query config account to get correct mint: `config.skill_mint`
- Don't attempt to use alternative tokens

**Security:**
This prevents attacks using fake SKILL tokens to fund matches.

**When it occurs:**
- `create_match` with wrong mint
- `fund` with wrong token account mint
- Any operation with mismatched mint

---

### Error 6017: `InvalidExpiry`

**Message:** "Invalid expiry. Must be between 10 minutes and 24 hours (1,500 - 216,000 slots)"

**Cause:**
Expiry slots parameter outside valid range.

**Resolution:**
- Use expiry_slots between MIN_EXPIRY_SLOTS (1,500) and MAX_EXPIRY_SLOTS (216,000)
- 1,500 slots ≈ 10 minutes (at 400ms/slot)
- 216,000 slots ≈ 24 hours

**Why these limits:**
- Minimum: Ensures reasonable time for both players to fund
- Maximum: Prevents excessively long-lived matches

**Example valid values:**
- 3,000 slots = ~20 minutes
- 9,000 slots = ~1 hour
- 54,000 slots = ~6 hours
- 216,000 slots = ~24 hours

---

### Error 6018: `InvalidAmount`

**Message:** "Amount must be greater than zero"

**Cause:**
Calculation resulted in zero tokens, usually in fee calculations.

**Resolution:**
- This is rare and usually indicates an implementation error
- Check that fee calculations don't round to zero
- Ensure stake amounts are reasonable

**When it occurs:**
- Internal fee calculations in settlement
- Usually with extremely small stakes

---

### Error 6019: `InsufficientBalance`

**Message:** "Insufficient SKILL token balance"

**Cause:**
Player's token account doesn't have enough SKILL tokens to fund the match.

**Resolution:**
1. Check token balance: Query player's SKILL token account
2. Ensure balance >= match stake amount
3. Deposit more SOL and mint SKILL via treasury if needed

**When it occurs:**
- `fund` instruction when player_skill_ata.amount < match.stake

**Example:**
```
Match stake: 10 SKILL (10,000,000 atomic units)
Player balance: 5 SKILL (5,000,000 atomic units)
Result: InsufficientBalance error
```

---

## ttt_onchain Errors

### Error 6000: `MatchNotActive`

**Message:** "Match is not active"

**Cause:**
Referenced match isn't in Active status.

**Resolution:**
- Ensure match is fully funded and started
- Check match status in escrow program

---

### Error 6001: `GameNotActive`

**Message:** "Game is not active"

**Cause:**
Game already finished.

**Resolution:**
- Can't make moves after game ends
- Check `game.status == Active`

---

### Error 6002: `InvalidPosition`

**Message:** "Invalid position (must be 0-8)"

**Cause:**
Position >= 9 on Tic-Tac-Toe board.

**Resolution:**
- Use position 0-8
- Board layout:
  ```
  0 | 1 | 2
  ---------
  3 | 4 | 5
  ---------
  6 | 7 | 8
  ```

---

### Error 6003: `PositionOccupied`

**Message:** "Position already occupied"

**Cause:**
Trying to place piece on non-empty square.

**Resolution:**
- Choose an empty position
- Check `game.board[position] == 0`

---

### Error 6004: `NotYourTurn`

**Message:** "Not your turn"

**Cause:**
Wrong player making move.

**Resolution:**
- Wait for your turn
- Check `game.current_turn`: 1 = X, 2 = O
- X = player_x, O = player_o

---

### Error 6005: `GameNotFinished`

**Message:** "Game is not finished"

**Cause:**
Attempting to resolve game that's still in progress.

**Resolution:**
- Wait for game to end (win or draw)
- Check `game.status == Finished`

---

### Error 6006: `MoveTimeout`

**Message:** "Move timeout"

**Cause:**
Player took too long to make move.

**Resolution:**
- Make moves within deadline
- Opponent can claim timeout win
- Check `Clock.slot <= game.deadline_slot`

---

### Error 6007: `NotTimedOut`

**Message:** "Game has not timed out yet"

**Cause:**
Attempting to claim timeout before deadline passed.

**Resolution:**
- Wait for deadline_slot to pass
- Check `Clock.slot > game.deadline_slot`

---

### Error 6008: `NotYourTimeout`

**Message:** "You cannot claim this timeout"

**Cause:**
Player whose turn it is trying to claim timeout.

**Resolution:**
- Only non-timeout player can claim
- If it's your turn and you timed out, opponent claims

---

## Debugging Tips

### Finding Error Codes

1. **From transaction simulation:**
   ```
   Error processing Instruction 0: custom program error: 0x1774
   ```
   Convert hex to decimal: `0x1774` = `6004`

2. **Using Solana CLI:**
   ```bash
   solana confirm <signature> -v
   ```

3. **Using Explorer:**
   - Go to Solana Explorer
   - Search transaction signature
   - View "Program Instruction Logs"

### Common Patterns

**Constraint violations:**
Usually show as "6000-6999" range errors.

**Account validation failures:**
Often appear as Anchor framework errors (different range).

**CPI failures:**
Look for nested error logs from called program.

---

## Auto-Generated

This documentation can be regenerated using:
```bash
./scripts/extract-errors.sh
```

Last updated: Generated from program source
