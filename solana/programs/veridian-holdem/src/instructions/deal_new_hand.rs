/**
 * @description
 * This file contains the logic for the `deal_new_hand` instruction. This instruction
 * is called at the beginning of each hand to trigger the confidential shuffling and dealing
 * process on the Arcium network.
 *
 * @key_features
 * - Initializes a new `HandState` account to store encrypted card data for the hand.
 * - Triggers the `shuffle_and_deal` confidential instruction via a CPI to Arcium.
 * - Validates that the game is in a state ready for a new hand and that the caller is the dealer.
 *
 * @dependencies
 * - crate::state: Defines the `GameState` and `HandState` account structures.
 * - crate::error: Defines custom error codes for validation.
 * - anchor_lang & arcium_anchor: For Solana program development and Arcium integration.
 */

use crate::{
    error::ErrorCode,
    state::{GamePhase, GameState, HandState, SignerAccount},
    ID,
};
use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::accounts::{ClockAccount, FeePool};
use arcium_client::idl::arcium::ID_CONST;

/// Defines the minimal accounts required to prepare a new hand (setup only).
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct DealNewHandSetup<'info> {
    /// The signer of the transaction, who must be the current dealer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The `GameState` account, which will be updated to reflect the start of a new hand.
    #[account(
        mut,
        seeds = [b"game", &game_state.table_id.to_le_bytes()[..]],
        bump
    )]
    pub game_state: Account<'info, GameState>,

    /// The `HandState` account, initialized to store this hand's encrypted data.
    #[account(
        init,
        payer = payer,
        space = 8 + HandState::INIT_SPACE,
        seeds = [b"hand", game_state.key().as_ref()],
        bump,
    )]
    pub hand_state: Box<Account<'info, HandState>>,

    /// Required signer PDA for Arcium operations
    #[account(
        init_if_needed,
        space = 8 + SignerAccount::INIT_SPACE,
        payer = payer,
        seeds = [b"sign_pda"],
        bump,
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,

    /// System program required for init constraints
    pub system_program: Program<'info, System>,
}

/// The handler function for the setup step of `deal_new_hand`.
pub fn deal_new_hand_setup(ctx: Context<DealNewHandSetup>, computation_offset: u64) -> Result<()> {
    let game_state = &mut ctx.accounts.game_state;
    let payer = &ctx.accounts.payer;

    // 1. Validation Checks
    require!(
        game_state.game_phase == GamePhase::HandOver || game_state.game_phase == GamePhase::Idle,
        ErrorCode::InvalidAction
    );
    require!(
        game_state.players[game_state.dealer_index as usize] == payer.key(),
        ErrorCode::Unauthorized
    );
    require!(
        game_state.players[0] != Pubkey::default() && game_state.players[1] != Pubkey::default(),
        ErrorCode::InvalidAction // Not enough players
    );

    // 2. Reset hand-specific state in GameState and initialize HandState.
    game_state.pot = 0;
    game_state.bets = [0, 0];
    game_state.community_cards = [255; 5];
    game_state.is_all_in = [false, false];
    game_state.game_phase = GamePhase::Dealing;
    game_state.last_action_timestamp = Clock::get()?.unix_timestamp;
    
    ctx.accounts.hand_state.computation_offset = computation_offset;
    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    Ok(())
}

/// Minimal Arcium queue context to avoid BPF stack overflow
#[queue_computation_accounts("shuffle_and_deal", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct DealNewHandQueue<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

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
    )]
    pub hand_state: Box<Account<'info, HandState>>,

    /// Required signer PDA for Arcium operations (already initialized in setup)
    #[account(
        seeds = [b"sign_pda"],
        bump
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,

    // Arcium
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut, address = derive_mempool_pda!())]
    /// CHECK: Arcium validates
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())]
    /// CHECK: Arcium validates
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))]
    /// CHECK: Arcium validates
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(comp_def_offset("shuffle_and_deal")))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: sysvar
    pub instructions_sysvar: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

pub fn deal_new_hand_queue(ctx: Context<DealNewHandQueue>, computation_offset: u64) -> Result<()> {
    // queue computation only
    let args = vec![];
    queue_computation(
        ctx.accounts,
        computation_offset,
        args,
        Some(String::new()),
        vec![],
    )?;
    Ok(())
}