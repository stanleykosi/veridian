/**
 * @description
 * This file contains the logic for the `player_action` instruction, which is the
 * heart of the on-chain gameplay. It handles all standard poker actions: Fold,
 * Check, Call, Bet, and Raise.
 *
 * @key_features
 * - A single entry point for all player actions, using an enum to differentiate.
 * - Rigorous validation of player turn, action legality, and bet amounts.
 * - Manages updates to player stacks, bets, and the pot.
 * - Handles all-in logic and side pots (though side pots are simpler in heads-up).
 * - Determines when a betting round is complete and transitions the `game_phase`.
 *
 * @dependencies
 * - crate::state: Defines the `GameState` account structure and `GamePhase` enum.
 * - crate::error: Defines custom error codes for validation.
 * - anchor_lang: The core Anchor framework library.
 */

use crate::{
    error::ErrorCode,
    state::{GamePhase, GameState, MAX_PLAYERS},
};
use anchor_lang::prelude::*;

/// Enum representing the possible actions a player can take.
/// Using a rich enum like this allows the client to send a single, structured
/// instruction instead of having separate on-chain instructions for each action.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum Action {
    Fold,
    Check,
    Call,
    Bet(u64),
    Raise(u64),
}

/// Defines the accounts required for a player to take an action.
#[derive(Accounts)]
pub struct PlayerAction<'info> {
    /// The player performing the action, who must sign the transaction.
    pub player: Signer<'info>,

    /// The `GameState` account for the table, which will be modified by the action.
    #[account(
        mut,
        seeds = [b"game", &game_state.table_id.to_le_bytes()[..]],
        bump
    )]
    pub game_state: Account<'info, GameState>,
}

/// The handler function for the `player_action` instruction.
pub fn player_action(ctx: Context<PlayerAction>, action: Action) -> Result<()> {
    let game_state = &mut ctx.accounts.game_state;
    let player = &ctx.accounts.player;
    let player_index = game_state.current_turn_index as usize;
    let opponent_index = (1 - game_state.current_turn_index) as usize;

    // --- 1. Validation ---
    // Ensure it's the correct player's turn.
    require!(
        game_state.players[player_index] == player.key(),
        ErrorCode::NotPlayerTurn
    );
    // Ensure the game is in a phase where actions are allowed.
    require!(
        matches!(
            game_state.game_phase,
            GamePhase::PreFlop | GamePhase::Flop | GamePhase::Turn | GamePhase::River
        ),
        ErrorCode::InvalidAction
    );

    // Get player stack and bet values
    let _player_stack = game_state.stacks[player_index];
    let _player_bet = game_state.bets[player_index];
    let opponent_bet = game_state.bets[opponent_index];

    // --- 2. Process Action ---
    match action {
        Action::Fold => {
            // Award pot to the opponent.
            game_state.stacks[opponent_index] += game_state.pot + game_state.bets[player_index] + opponent_bet;
            // Transition to HandOver to await the next deal.
            transition_to_next_hand(game_state);
        }
        Action::Check => {
            // A check is only valid if the player's bet matches the opponent's bet.
            require!(game_state.bets[player_index] == opponent_bet, ErrorCode::InvalidAction);
            // If the checker is the big blind (second to act pre-flop) or the small blind
            // (first to act post-flop) and bets are equal, the round ends.
            let is_round_over = game_state.current_turn_index != game_state.dealer_index;
            if is_round_over {
                handle_round_transition(game_state);
            } else {
                game_state.current_turn_index = opponent_index as u8;
            }
        }
        Action::Call => {
            let _amount_to_call = opponent_bet - game_state.bets[player_index];
            // Cannot call if no bet is pending.
            require!(_amount_to_call > 0, ErrorCode::InvalidAction);

            if _amount_to_call >= game_state.stacks[player_index] {
                // Player is all-in.
                game_state.bets[player_index] += game_state.stacks[player_index];
                game_state.stacks[player_index] = 0;
                game_state.is_all_in[player_index] = true;
            } else {
                // Regular call.
                game_state.stacks[player_index] -= _amount_to_call;
                game_state.bets[player_index] += _amount_to_call;
            }
            // A call always ends the betting round.
            handle_round_transition(game_state);
        }
        Action::Bet(amount) => {
            // A bet is only valid if there are no outstanding bets.
            require!(game_state.bets[player_index] == opponent_bet, ErrorCode::InvalidAction);
            require!(amount > 0, ErrorCode::InvalidBetAmount);
            require!(amount <= game_state.stacks[player_index], ErrorCode::InsufficientFunds);
            // TODO: Add validation for minimum bet size (e.g., must be at least the big blind).

            game_state.stacks[player_index] -= amount;
            game_state.bets[player_index] += amount;
            if game_state.stacks[player_index] == 0 {
                game_state.is_all_in[player_index] = true;
            }
            game_state.current_turn_index = opponent_index as u8;
        }
        Action::Raise(amount) => {
            let raise_amount = amount - game_state.bets[player_index];
            let _amount_to_call = opponent_bet - game_state.bets[player_index];
            let min_raise = opponent_bet - game_state.bets[player_index]; // The previous bet/raise size.
            // A raise must be at least the size of the previous bet/raise.
            require!(raise_amount >= min_raise, ErrorCode::InvalidBetAmount);
            require!(amount > opponent_bet, ErrorCode::InvalidBetAmount);
            require!(amount <= game_state.stacks[player_index] + game_state.bets[player_index], ErrorCode::InsufficientFunds);

            let total_investment = amount - game_state.bets[player_index];
            game_state.stacks[player_index] -= total_investment;
            game_state.bets[player_index] = amount;

            if game_state.stacks[player_index] == 0 {
                game_state.is_all_in[player_index] = true;
            }
            game_state.current_turn_index = opponent_index as u8;
        }
    }

    // --- 3. Update Timestamp ---
    game_state.last_action_timestamp = Clock::get()?.unix_timestamp;

    Ok(())
}

/// Helper function to transition the game state after a betting round concludes.
fn handle_round_transition(game_state: &mut Account<GameState>) {
    // 1. Collect bets into the main pot.
    game_state.pot += game_state.bets[0] + game_state.bets[1];
    game_state.bets = [0; MAX_PLAYERS];

    // 2. Check for all-in showdown.
    let p0_all_in = game_state.is_all_in[0];
    let p1_all_in = game_state.is_all_in[1];

    if p0_all_in || p1_all_in {
        // If an all-in occurs and is called, the game proceeds directly to showdown.
        // All remaining community cards will be dealt before the winner is determined.
        // This is simplified as the logic to reveal all cards at once is not yet implemented.
        game_state.game_phase = GamePhase::Showdown;
        return;
    }

    // 3. Advance to the next game phase.
    game_state.game_phase = match game_state.game_phase {
        GamePhase::PreFlop => GamePhase::Flop,
        GamePhase::Flop => GamePhase::Turn,
        GamePhase::Turn => GamePhase::River,
        GamePhase::River => GamePhase::Showdown,
        _ => game_state.game_phase, // Should not happen
    };

    // 4. Set the turn to the player out of position (first to act post-flop).
    game_state.current_turn_index = 1 - game_state.dealer_index;
}

/// Helper function to reset the game state for the next hand.
fn transition_to_next_hand(game_state: &mut Account<GameState>) {
    game_state.game_phase = GamePhase::HandOver;
    game_state.pot = 0;
    game_state.bets = [0; MAX_PLAYERS];
    game_state.community_cards = [255; 5];
    game_state.is_all_in = [false; MAX_PLAYERS];
    // Swap the dealer button for the next hand.
    game_state.dealer_index = 1 - game_state.dealer_index;
    game_state.current_turn_index = game_state.dealer_index;
}