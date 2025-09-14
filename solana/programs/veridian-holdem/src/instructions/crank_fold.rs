/**
 * @description
 * This file contains the logic for the `crank_fold` permissionless instruction.
 * Anyone can call this instruction to force a fold on behalf of a player whose
 * turn timer has expired, preventing the game from stalling due to an inactive player.
 *
 * @key_features
 * - Permissionless: Can be called by any account, ensuring the game can always proceed.
 * - Time-based Validation: Uses Solana's on-chain `Clock` to check if the turn duration has exceeded a predefined limit.
 * - State Transition: Folds the current player's hand, awards the pot to the opponent, and resets the game state for the next hand.
 *
 * @dependencies
 * - crate::state: Defines `GameState`, `GamePhase`, and the `TURN_TIME_SECONDS` constant.
 * - crate::error: Defines custom error codes for validation.
 * - anchor_lang: The core Anchor framework library.
 */

use crate::{
    error::ErrorCode,
    state::{GamePhase, GameState, MAX_PLAYERS, TURN_TIME_SECONDS},
};
use anchor_lang::prelude::*;

/// Defines the accounts required for the `crank_fold` instruction.
/// Since this is a permissionless crank, it only needs mutable access to the `GameState`.
/// The caller of this instruction will be the transaction fee payer.
#[derive(Accounts)]
pub struct CrankFold<'info> {
    /// The `GameState` account for the table being cranked.
    /// It must be mutable as the instruction will update its state.
    #[account(
        mut,
        seeds = [b"game", &game_state.table_id.to_le_bytes()[..]],
        bump
    )]
    pub game_state: Account<'info, GameState>,
}

/// The handler function for the `crank_fold` instruction.
pub fn crank_fold(ctx: Context<CrankFold>) -> Result<()> {
    let game_state = &mut ctx.accounts.game_state;

    // 1. Validate that the game is in an active betting phase where a player can time out.
    require!(
        matches!(
            game_state.game_phase,
            GamePhase::PreFlop | GamePhase::Flop | GamePhase::Turn | GamePhase::River
        ),
        ErrorCode::InvalidAction
    );

    // 2. Check if the turn timer has actually expired using the on-chain clock.
    let current_timestamp = Clock::get()?.unix_timestamp;
    require!(
        current_timestamp > game_state.last_action_timestamp + TURN_TIME_SECONDS,
        ErrorCode::TimerNotExpired
    );

    // 3. Identify the player who timed out and their opponent.
    let timed_out_player_index = game_state.current_turn_index as usize;
    let opponent_index = (1 - game_state.current_turn_index) as usize;

    // 4. Perform the fold logic:
    //    a. Calculate the total pot size, including all bets from the current street.
    let total_pot = game_state.pot + game_state.bets[0] + game_state.bets[1];
    
    //    b. Award the entire pot to the opponent.
    game_state.stacks[opponent_index] += total_pot;

    // 5. Transition the game to the "HandOver" state to prepare for the next deal.
    game_state.game_phase = GamePhase::HandOver;
    game_state.pot = 0;
    game_state.bets = [0; MAX_PLAYERS];
    game_state.community_cards = [255; 5];
    game_state.is_all_in = [false; MAX_PLAYERS];
    
    //    c. Swap the dealer button for the next hand.
    game_state.dealer_index = 1 - game_state.dealer_index;
    
    //    d. The turn for the next hand starts with the player who is now the small blind/button.
    game_state.current_turn_index = game_state.dealer_index;

    //    e. Update the action timestamp to reset the timer for the next hand's pre-deal phase.
    game_state.last_action_timestamp = Clock::get()?.unix_timestamp;

    msg!("Player {} timed out. Awarded pot of {} to player {}.", timed_out_player_index, total_pot, opponent_index);

    Ok(())
}