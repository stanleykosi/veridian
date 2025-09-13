/**
 * @description
 * This file defines the custom error codes for the Veridian Hold'em on-chain program.
 * Using a custom error enum with `#[error_code]` provides clear, debuggable reasons for
 * transaction failures, which is essential for both development and user experience.
 *
 * @dependencies
 * - anchor_lang: The core Anchor framework library.
 */

use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("It is not the current player's turn to act.")]
    NotPlayerTurn,

    #[msg("The specified bet or raise amount is invalid.")]
    InvalidBetAmount,

    #[msg("The action is not authorized for this signer.")]
    Unauthorized,

    #[msg("The table is already full and cannot accept new players.")]
    TableFull,

    #[msg("The action cannot be performed because the hand is not over.")]
    HandNotOver,

    #[msg("A game is already in progress at this table.")]
    GameAlreadyInProgress,

    #[msg("The specified player is not currently at this table.")]
    PlayerNotInGame,

    #[msg("The attempted action is not valid for the current game state.")]
    InvalidAction,

    #[msg("The player has insufficient funds to perform this action.")]
    InsufficientFunds,

    #[msg("Cluster not set")]
    ClusterNotSet,
}