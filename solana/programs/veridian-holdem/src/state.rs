/**
 * @description
 * This file defines all the on-chain account structures and enums required for Veridian Hold'em.
 * These structs act as the "database schema" for the public state of the game on the Solana blockchain.
 *
 * @dependencies
 * - anchor_lang: The core Anchor framework library for building Solana programs.
 *
 * @notes
 * - The `InitSpace` macro is used to automatically calculate the required space for each account,
 *   which simplifies account initialization and rent calculation.
 * - All accounts are designed to be Program Derived Addresses (PDAs) to ensure they are owned and
 *   managed by the on-chain program.
 * - Constants like `MAX_PLAYERS` are used to ensure consistency and make the code more maintainable.
 */

use anchor_lang::prelude::*;

/// The maximum number of players at a table. For Heads-Up, this is always 2.
pub const MAX_PLAYERS: usize = 2;

/// Defines the current phase of a poker hand, dictating which actions are valid.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace, Copy)]
pub enum GamePhase {
    /// No game is active. The table is waiting for players to join or for a new hand to start.
    Idle,
    /// The hand is being dealt. An Arcium computation is in progress to shuffle and deal cards.
    Dealing,
    /// The first betting round, before any community cards are revealed.
    PreFlop,
    /// The second betting round, after the first three community cards (the flop) are revealed.
    Flop,
    /// The third betting round, after the fourth community card (the turn) is revealed.
    Turn,
    /// The final betting round, after the fifth community card (the river) is revealed.
    River,
    /// Hands are being compared confidentially by Arcium to determine the winner.
    Showdown,
    /// The hand is complete, and the pot has been distributed. Waiting to start the next hand.
    HandOver,
}

/// Singleton PDA account for global administrative configuration.
/// This account stores settings that apply to the entire platform, like rake rules.
/// PDA Seeds: `[b"config"]`
#[account]
#[derive(InitSpace)]
pub struct Config {
    /// The public key of the administrator who has permission to change settings.
    pub admin: Pubkey,
    /// The wallet address that receives the collected rake from all games.
    pub treasury_wallet: Pubkey,
    /// The percentage of the pot taken as rake, represented as a u8 (e.g., 5 means 5%).
    pub rake_percentage: u8,
    /// The maximum rake amount that can be taken from a single pot, specified in the smallest
    /// unit of the game's SPL token (e.g., lamports for SOL).
    pub rake_cap: u64,
}

/// Stores the immutable configuration for a specific poker table, such as stakes and buy-in.
/// This account is created once when a new table is set up.
/// PDA Seeds: `[b"table_config", table_id.to_le_bytes().as_ref()]`
#[account]
#[derive(InitSpace)]
pub struct TableConfig {
    /// A unique identifier for the table.
    pub table_id: u64,
    /// The small blind amount for this table.
    pub small_blind: u64,
    /// The big blind amount for this table.
    pub big_blind: u64,
    /// The required buy-in amount to join the table.
    pub buy_in: u64,
    /// The mint address of the SPL Token used as the currency for this table (e.g., USDC).
    pub token_mint: Pubkey,
}

/// Holds the public, mutable state of a single poker table.
/// This account is updated frequently as the game progresses.
/// PDA Seeds: `[b"game", table_id.to_le_bytes().as_ref()]`
#[account]
#[derive(InitSpace)]
pub struct GameState {
    /// A public key linking to the table's static `TableConfig` account.
    pub table_config: Pubkey,
    /// The public keys of the two players at the table. A `Pubkey::default()`
    /// value indicates an empty seat.
    pub players: [Pubkey; MAX_PLAYERS],
    /// The current chip stacks for each player.
    pub stacks: [u64; MAX_PLAYERS],
    /// The current phase of the game (e.g., PreFlop, Flop).
    pub game_phase: GamePhase,
    /// The total amount of chips in the pot for the current hand.
    pub pot: u64,
    /// The amount each player has contributed to the pot in the current betting round.
    pub bets: [u64; MAX_PLAYERS],
    /// The five community cards. A value of 255 represents an un-dealt card.
    pub community_cards: [u8; 5],
    /// Flags to track if a player is all-in.
    pub is_all_in: [bool; MAX_PLAYERS],
    /// The index (0 or 1) of the player whose turn it is to act.
    pub current_turn_index: u8,
    /// The index (0 or 1) of the player who is the dealer (on the button).
    pub dealer_index: u8,
    /// The Unix timestamp of the last action taken, used for the turn timer.
    pub last_action_timestamp: i64,
    /// A flag indicating if a game is currently active at this table.
    pub is_active: bool,
}

/// A temporary account holding encrypted, confidential data for the current hand.
/// This account is created at the start of a hand and closed at the end to reclaim rent.
/// PDA Seeds: `[b"hand", game_state.key().as_ref()]`
#[account]
#[derive(InitSpace)]
pub struct HandState {
    /// Encrypted hole cards for each player. Each blob contains the ciphertext, nonce,
    /// and public key required for client-side decryption. The size is 128 bytes per player.
    pub encrypted_hole_cards: [[u8; 128]; MAX_PLAYERS],
    /// The remaining 48 cards of the deck, encrypted for use only by the Arcium MXE.
    /// The size is calculated to hold the nonce and 48 encrypted card values.
    pub encrypted_deck: [u8; 1568],
    /// The transaction signature of the `deal_new_hand` instruction. This provides a
    /// verifiable on-chain link for auditing the integrity of the shuffle.
    pub shuffle_tx_sig: [u8; 64],
}