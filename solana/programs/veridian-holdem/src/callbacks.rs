/**
 * @description
 * This file contains all the Arcium callback account contexts and handler functions for the Veridian Hold'em program.
 * These callbacks are invoked by the Arcium network after a confidential computation completes,
 * delivering the results back on-chain to update the public `GameState`.
 *
 * @key_features
 * - `DealNewHandCallback`: Processes the encrypted cards and deck from the shuffle computation.
 * - `RevealCommunityCardsCallback`: Updates the public board with newly revealed cards.
 * - `DetermineWinnerCallback`: Processes the winner index, calculates rake, distributes the pot, and resets the hand.
 *
 * @dependencies
 * - arcium_anchor & arcium_macros: For defining callback instructions and handling `ComputationOutputs`.
 * - crate::state & crate::error: For accessing account structures and custom errors.
 * - anchor_spl::token: For performing secure token transfers (CPIs) during pot distribution.
 */
use crate::{
    error::ErrorCode,
    state::{Config, GamePhase, GameState, HandState, TableConfig, MAX_PLAYERS},
    ID,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::ID_CONST;
use arcium_macros::arcium_callback;
use arcium_client::idl::arcium::types::CallbackInstruction;

// Define output types for Arcium computations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ShuffleAndDealOutput {
    pub field_0: (Vec<u8>, Vec<u8>, Vec<u8>), // (p1_encrypted_cards, p2_encrypted_cards, encrypted_deck)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct RevealCommunityCardsOutput {
    pub field_0: (Vec<u8>, Vec<Vec<u8>>), // (encrypted_deck, revealed_cards)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DetermineWinnerOutput {
    pub field_0: u8, // winner_index (0, 1, or 2 for tie)
}

// This function is required by the arcium_callback macro
fn validate_callback_ixs(_account_info: &AccountInfo, _program_id: &Pubkey) -> Result<()> {
    Ok(())
}

/// Accounts required for the `deal_new_hand` callback.
#[derive(Accounts)]
pub struct DealNewHandCallback<'info> {
    #[account(
        mut,
        seeds = [b"game", &game_state.table_id.to_le_bytes()[..]],
        bump
    )]
    pub game_state: Account<'info, GameState>,

    #[account(
        mut,
        seeds = [b"hand", game_state.key().as_ref()],
        bump
    )]
    pub hand_state: Account<'info, HandState>,

    #[account(
        seeds = [b"table_config", &game_state.table_id.to_le_bytes()[..]],
        bump
    )]
    pub table_config: Account<'info, TableConfig>,
    
    #[account(
        address = derive_comp_def_pda!(comp_def_offset("shuffle_and_deal"))
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
}

impl<'info> DealNewHandCallback<'info> {
    pub fn callback_ix(_args: &[&[u8]]) -> CallbackInstruction {
        CallbackInstruction {
            program_id: crate::ID,
            accounts: vec![],
            discriminator: vec![0u8; 8], // This will be set by the Arcium system
        }
    }
}

impl<'info> RevealCommunityCardsCallback<'info> {
    pub fn callback_ix(_args: &[&[u8]]) -> CallbackInstruction {
        CallbackInstruction {
            program_id: crate::ID,
            accounts: vec![],
            discriminator: vec![0u8; 8], // This will be set by the Arcium system
        }
    }
}

/// Accounts required for the `reveal_community_cards` callback.
#[derive(Accounts)]
pub struct RevealCommunityCardsCallback<'info> {
    #[account(
        mut,
        seeds = [b"game", &game_state.table_id.to_le_bytes()[..]],
        bump
    )]
    pub game_state: Account<'info, GameState>,

    #[account(
        mut,
        seeds = [b"hand", game_state.key().as_ref()],
        bump
    )]
    pub hand_state: Account<'info, HandState>,
    
    #[account(
        address = derive_comp_def_pda!(comp_def_offset("reveal_community_cards"))
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
}

impl<'info> DetermineWinnerCallback<'info> {
    pub fn callback_ix(_args: &[&[u8]]) -> CallbackInstruction {
        CallbackInstruction {
            program_id: crate::ID,
            accounts: vec![],
            discriminator: vec![0u8; 8], // This will be set by the Arcium system
        }
    }
}

/// Accounts required for the `determine_winner` callback.
#[derive(Accounts)]
pub struct DetermineWinnerCallback<'info> {
    #[account(
        mut,
        seeds = [b"game", &game_state.table_id.to_le_bytes()[..]],
        bump
    )]
    pub game_state: Account<'info, GameState>,

    #[account(
        mut,
        seeds = [b"hand", game_state.key().as_ref()],
        bump,
        close = dealer_account // Close the HandState account and refund rent to the dealer.
    )]
    pub hand_state: Account<'info, HandState>,

    #[account(
        seeds = [b"config"],
        bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"escrow", game_state.key().as_ref()],
        bump
    )]
    pub escrow_account: Account<'info, TokenAccount>,

    /// CHECK: This is the dealer of the hand who paid for the HandState account's rent.
    #[account(mut)]
    pub dealer_account: UncheckedAccount<'info>,
    
    /// CHECK: This is the treasury wallet that receives rake.
    #[account(mut)]
    pub treasury_token_account: UncheckedAccount<'info>,

    #[account(
        address = derive_comp_def_pda!(comp_def_offset("determine_winner"))
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
    pub arcium_program: Program<'info, Arcium>,
}

// --- Callback Implementations ---

/// Callback for the `shuffle_and_deal` confidential instruction.
/// It receives the encrypted card data and updates the on-chain state to start the hand.
#[arcium_callback(encrypted_ix = "shuffle_and_deal")]
pub fn shuffle_and_deal_callback(
    ctx: Context<DealNewHandCallback>,
    output: ComputationOutputs<ShuffleAndDealOutput>,
) -> Result<()> {
    let (p1_data, p2_data, deck_data) = match output {
        ComputationOutputs::Success(ShuffleAndDealOutput { field_0: data }) => {
            (data.0, data.1, data.2)
        }
        _ => return err!(ErrorCode::InvalidAction), // Or a more specific error
    };

    let hand_state = &mut ctx.accounts.hand_state;

    // Serialize and store the encrypted data blobs into the HandState account.
    let p1_vec = p1_data.try_to_vec()?;
    hand_state.encrypted_hole_cards[0][..p1_vec.len()].copy_from_slice(&p1_vec);

    let p2_vec = p2_data.try_to_vec()?;
    hand_state.encrypted_hole_cards[1][..p2_vec.len()].copy_from_slice(&p2_vec);
    
    let deck_vec = deck_data.try_to_vec()?;
    hand_state.encrypted_deck[..deck_vec.len()].copy_from_slice(&deck_vec);

    // Post blinds.
    let game_state = &mut ctx.accounts.game_state;
    let table_config = &ctx.accounts.table_config;
    let small_blind_idx = game_state.dealer_index as usize;
    let big_blind_idx = (1 - game_state.dealer_index) as usize;

    game_state.stacks[small_blind_idx] -= table_config.small_blind;
    game_state.bets[small_blind_idx] = table_config.small_blind;

    game_state.stacks[big_blind_idx] -= table_config.big_blind;
    game_state.bets[big_blind_idx] = table_config.big_blind;

    // Set the game phase and first player to act (dealer/small blind acts first pre-flop).
    game_state.game_phase = GamePhase::PreFlop;
    game_state.current_turn_index = game_state.dealer_index;
    
    Ok(())
}

/// Callback for the `reveal_community_cards` confidential instruction.
#[arcium_callback(encrypted_ix = "reveal_community_cards")]
pub fn reveal_community_cards_callback(
    ctx: Context<RevealCommunityCardsCallback>,
    output: ComputationOutputs<RevealCommunityCardsOutput>,
) -> Result<()> {
    let (deck_data, revealed_cards_data) = match output {
        ComputationOutputs::Success(RevealCommunityCardsOutput { field_0: data }) => {
            (data.0, data.1)
        }
        _ => return err!(ErrorCode::InvalidAction),
    };

    // Update the encrypted deck in HandState.
    let hand_state = &mut ctx.accounts.hand_state;
    let deck_vec = deck_data.try_to_vec()?;
    hand_state.encrypted_deck[..deck_vec.len()].copy_from_slice(&deck_vec);

    // Update the public community cards in GameState.
    let game_state = &mut ctx.accounts.game_state;
    let revealed_cards = revealed_cards_data; // This assumes they are revealed as plaintext in a real scenario.
                                                            // For now, let's assume the callback gives us plaintext cards.
                                                            // NOTE: Arcis instruction needs adjustment to return plaintext.
                                                            // For now, we'll work with this assumption.

    if game_state.game_phase == GamePhase::Flop {
        if revealed_cards.len() >= 3 {
            game_state.community_cards[0] = revealed_cards[0][0]; // Simplified extraction
            game_state.community_cards[1] = revealed_cards[1][0];
            game_state.community_cards[2] = revealed_cards[2][0];
        }
    } else if game_state.game_phase == GamePhase::Turn {
        if revealed_cards.len() >= 1 {
            game_state.community_cards[3] = revealed_cards[0][0];
        }
    } else if game_state.game_phase == GamePhase::River {
        if revealed_cards.len() >= 1 {
            game_state.community_cards[4] = revealed_cards[0][0];
        }
    }

    // Set turn for the next betting round (player out of position acts first).
    game_state.current_turn_index = 1 - game_state.dealer_index;

    Ok(())
}

/// Callback for the `determine_winner` confidential instruction.
#[arcium_callback(encrypted_ix = "determine_winner")]
pub fn determine_winner_callback(
    ctx: Context<DetermineWinnerCallback>,
    output: ComputationOutputs<DetermineWinnerOutput>,
) -> Result<()> {
    let winner_index = match output {
        ComputationOutputs::Success(DetermineWinnerOutput { field_0: index }) => index,
        _ => return err!(ErrorCode::InvalidAction),
    };

    let game_state = &mut ctx.accounts.game_state;
    let config = &ctx.accounts.config;

    let total_pot = game_state.pot + game_state.bets[0] + game_state.bets[1];
    let mut rake = 0;

    // Rake Calculation ("No Flop, No Drop").
    if game_state.community_cards[0] != 255 {
        rake = (total_pot * config.rake_percentage as u64) / 100;
        if rake > config.rake_cap {
            rake = config.rake_cap;
        }
    }

    let pot_after_rake = total_pot - rake;

    let seeds = &[
        b"game",
        &game_state.table_id.to_le_bytes()[..],
        &[ctx.bumps.game_state],
    ];
    let signer = &[&seeds[..]];

    // Transfer rake to treasury.
    if rake > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.escrow_account.to_account_info(),
            to: ctx.accounts.treasury_token_account.to_account_info(),
            authority: game_state.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, rake)?;
    }

    // Distribute pot.
    if winner_index == 2 { // Tie
        let split_amount = pot_after_rake / 2;
        game_state.stacks[0] += split_amount;
        game_state.stacks[1] += split_amount;
        // Handle odd chip if pot is not even.
        if pot_after_rake % 2 == 1 {
            let odd_chip_recipient = 1 - game_state.dealer_index; // Out of position
            game_state.stacks[odd_chip_recipient as usize] += 1;
        }
    } else { // Single winner
        game_state.stacks[winner_index as usize] += pot_after_rake;
    }

    // Reset game state for the next hand.
    game_state.game_phase = GamePhase::HandOver;
    game_state.pot = 0;
    game_state.bets = [0; MAX_PLAYERS];
    game_state.community_cards = [255; 5];
    game_state.is_all_in = [false; MAX_PLAYERS];
    game_state.dealer_index = 1 - game_state.dealer_index;
    game_state.current_turn_index = game_state.dealer_index;
    
    Ok(())
}