/**
 * @description
 * This file contains the logic for the `join_table` instruction, which allows a second player
 * to join an existing, open game table. It handles validation, updates the game state, and
 * transfers the player's buy-in to the escrow account.
 *
 * @key_features
 * - Validates that the table is open and not already active.
 * - Prevents a player from joining their own game.
 * - Updates the `GameState` with the new player's information.
 * - Transfers the joiner's buy-in using a secure CPI to the SPL Token Program.
 * - Transitions the game to the `HandOver` phase, making it ready for the first deal.
 *
 * @dependencies
 * - crate::state: Defines the `GameState` and `TableConfig` account structures.
 * - crate::error: Defines custom error codes for validation.
 * - anchor_lang: The core Anchor framework library.
 * - anchor_spl: Anchor's helpers for interacting with SPL Token Program.
 */
use crate::{
    error::ErrorCode,
    state::{GamePhase, GameState, TableConfig},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

/// Defines the accounts required for a player to join a table.
#[derive(Accounts)]
pub struct JoinTable<'info> {
    /// The `GameState` account for the table being joined.
    /// Several constraints are applied to ensure the join is valid:
    /// - `!game_state.is_active`: The game cannot already be in progress.
    /// - `game_state.players[1] == Pubkey::default()`: There must be an empty seat.
    #[account(
        mut,
        seeds = [b"game", &table_config.table_id.to_le_bytes()[..]],
        bump,
        constraint = !game_state.is_active @ ErrorCode::GameAlreadyInProgress,
        constraint = game_state.players[1] == Pubkey::default() @ ErrorCode::TableFull,
    )]
    pub game_state: Account<'info, GameState>,

    /// The `TableConfig` account, needed to verify the `buy_in` amount and `token_mint`.
    #[account(
        seeds = [b"table_config", &table_config.table_id.to_le_bytes()[..]],
        bump
    )]
    pub table_config: Account<'info, TableConfig>,

    /// The game's escrow token account where the joiner's buy-in will be deposited.
    #[account(
        mut,
        seeds = [b"escrow", game_state.key().as_ref()],
        bump
    )]
    pub escrow_account: Account<'info, TokenAccount>,

    /// The player joining the table, who must sign the transaction.
    #[account(mut)]
    pub joiner: Signer<'info>,

    /// The joiner's personal token account.
    /// Constraints ensure it's the correct token and that the joiner isn't the same
    /// as the player already at the table.
    #[account(
        mut,
        constraint = joiner_token_account.mint == table_config.token_mint,
        constraint = joiner.key() != game_state.players[0] @ ErrorCode::InvalidAction
    )]
    pub joiner_token_account: Account<'info, TokenAccount>,

    /// The SPL Token Program.
    pub token_program: Program<'info, Token>,
}

/// The handler function for the `join_table` instruction.
pub fn join_table(ctx: Context<JoinTable>) -> Result<()> {
    let game_state = &mut ctx.accounts.game_state;
    let table_config = &ctx.accounts.table_config;

    // 1. Update GameState: Add the new player to the empty seat, set their stack,
    //    and mark the game as active and ready for a new hand.
    game_state.players[1] = ctx.accounts.joiner.key();
    game_state.stacks[1] = table_config.buy_in;
    game_state.is_active = true;
    game_state.game_phase = GamePhase::HandOver; // Ready for the first hand to be dealt.

    // 2. Perform a CPI to the SPL Token Program to transfer the joiner's buy-in.
    let cpi_accounts = Transfer {
        from: ctx.accounts.joiner_token_account.to_account_info(),
        to: ctx.accounts.escrow_account.to_account_info(),
        authority: ctx.accounts.joiner.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, table_config.buy_in)?;

    Ok(())
}