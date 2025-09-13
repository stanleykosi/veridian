/**
 * @description
 * This file contains the logic for the `create_table` instruction, which allows a player
 * to initialize a new poker game table. It sets up all required Program Derived Accounts (PDAs)
 * and transfers the creator's buy-in into a secure escrow.
 *
 * @key_features
 * - Initializes `TableConfig`, `GameState`, and an SPL Token `escrow` account.
 * - Seeds PDAs with a unique `table_id` to ensure each table has a distinct set of accounts.
 * - Transfers the creator's funds using a secure CPI to the SPL Token Program.
 *
 * @dependencies
 * - crate::state: Defines the `GameState` and `TableConfig` account structures.
 * - crate::error: Defines custom error codes for validation.
 * - anchor_lang: The core Anchor framework library.
 * - anchor_spl: Anchor's helpers for interacting with SPL Token Program.
 */
use crate::{
    state::{GamePhase, GameState, TableConfig, MAX_PLAYERS},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

/// Defines the accounts required to create a new poker table.
/// The `#[instruction(table_id: u64)]` macro makes the `table_id` from the instruction
/// arguments available for use in account seed constraints.
#[derive(Accounts)]
#[instruction(table_id: u64)]
pub struct CreateTable<'info> {
    /// The `TableConfig` account, which stores the immutable rules of the table (blinds, buy-in).
    /// Initialized as a PDA seeded with "table_config" and the unique table ID.
    #[account(
        init,
        payer = creator,
        space = 8 + TableConfig::INIT_SPACE,
        seeds = [b"table_config", &table_id.to_le_bytes()[..]],
        bump
    )]
    pub table_config: Account<'info, TableConfig>,

    /// The `GameState` account, holding the dynamic public state of the game.
    /// Initialized as a PDA seeded with "game" and the unique table ID.
    #[account(
        init,
        payer = creator,
        space = 8 + GameState::INIT_SPACE,
        seeds = [b"game", &table_id.to_le_bytes()[..]],
        bump
    )]
    pub game_state: Account<'info, GameState>,

    /// The SPL Token account that will act as the secure escrow for this game.
    /// Initialized as a PDA seeded with "escrow" and the `game_state` PDA's key.
    /// The `game_state` PDA is set as the authority, meaning only the program can move funds.
    #[account(
        init,
        payer = creator,
        seeds = [b"escrow", game_state.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = game_state,
    )]
    pub escrow_account: Account<'info, TokenAccount>,

    /// The player creating the table. They must sign the transaction and will pay for account creation.
    #[account(mut)]
    pub creator: Signer<'info>,

    /// The mint of the SPL Token to be used for this table's currency.
    pub token_mint: Account<'info, Mint>,

    /// The creator's personal token account from which the buy-in will be transferred.
    /// A constraint ensures this account matches the specified `token_mint`.
    #[account(
        mut,
        constraint = creator_token_account.mint == token_mint.key()
    )]
    pub creator_token_account: Account<'info, TokenAccount>,

    /// Standard Solana programs required for account creation and token operations.
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

/// The handler function for the `create_table` instruction.
pub fn create_table(
    ctx: Context<CreateTable>,
    table_id: u64,
    small_blind: u64,
    big_blind: u64,
    buy_in: u64,
) -> Result<()> {
    // 1. Initialize the TableConfig account with the specified game rules.
    let table_config = &mut ctx.accounts.table_config;
    table_config.table_id = table_id;
    table_config.small_blind = small_blind;
    table_config.big_blind = big_blind;
    table_config.buy_in = buy_in;
    table_config.token_mint = ctx.accounts.token_mint.key();

    // 2. Initialize the GameState account with default values for a new, empty table.
    let game_state = &mut ctx.accounts.game_state;
    game_state.table_config = table_config.key();
    game_state.players[0] = ctx.accounts.creator.key();
    game_state.players[1] = Pubkey::default(); // Represents an empty seat.
    game_state.stacks[0] = buy_in;
    game_state.stacks[1] = 0;
    game_state.game_phase = GamePhase::Idle; // Waiting for another player.
    game_state.pot = 0;
    game_state.bets = [0; MAX_PLAYERS];
    game_state.community_cards = [255; 5]; // 255 indicates an un-dealt card.
    game_state.is_all_in = [false; MAX_PLAYERS];
    game_state.current_turn_index = 0;
    game_state.dealer_index = 0; // The creator is the first dealer.
    game_state.last_action_timestamp = 0;
    game_state.is_active = false; // Game becomes active when the second player joins.

    // 3. Perform a CPI to the SPL Token Program to transfer the creator's buy-in to the escrow account.
    let cpi_accounts = Transfer {
        from: ctx.accounts.creator_token_account.to_account_info(),
        to: ctx.accounts.escrow_account.to_account_info(),
        authority: ctx.accounts.creator.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, buy_in)?;

    Ok(())
}