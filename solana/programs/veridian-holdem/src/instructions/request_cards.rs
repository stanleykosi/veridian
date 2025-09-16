/**
 * @description
 * This file contains instructions for requesting confidential computations related to cards.
 * This includes revealing community cards and initiating a showdown to determine the winner.
 *
 * @key_features
 * - `request_community_cards`: Triggers the Arcium computation to reveal the flop, turn, or river.
 * - `request_showdown`: Triggers the Arcium computation to confidentially compare hands and find a winner.
 *
 * @dependencies
 * - crate::state: Defines `GameState` and `HandState`.
 * - crate::error: Defines custom error codes.
 * - anchor_lang & arcium_anchor: For Solana and Arcium integration.
 */
use crate::{
    callbacks::{RevealCommunityCardsCallback, DetermineWinnerCallback},
    error::ErrorCode,
    state::{Config, GamePhase, GameState, HandState, SignerAccount},
    ID,
};
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::accounts::{ClockAccount, FeePool};
use arcium_client::idl::arcium::ID_CONST;

/// Accounts for requesting the reveal of community cards (Flop, Turn, River).
#[queue_computation_accounts("reveal_community_cards", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct RequestCommunityCards<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut, seeds = [b"game", &game_state.table_id.to_le_bytes()[..]], bump)]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(seeds = [b"hand", game_state.key().as_ref()], bump)]
    pub hand_state: Box<Account<'info, HandState>>,

    #[account(
        init_if_needed,
        space = 8 + SignerAccount::INIT_SPACE,
        payer = payer,
        seeds = [b"sign_pda"],
        bump,
    )]
    pub sign_pda_account: Box<Account<'info, SignerAccount>>,

    // --- Arcium Required Accounts ---
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut, address = derive_mempool_pda!())]
    /// CHECK: Checked by Arcium program
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())]
    /// CHECK: Checked by Arcium program
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))]
    /// CHECK: Checked by Arcium program
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(comp_def_offset("reveal_community_cards")))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions sysvar
    pub instructions_sysvar: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

/// Accounts for requesting a showdown to determine the winner.
#[queue_computation_accounts("determine_winner", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct RequestShowdown<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut, seeds = [b"game", &game_state.table_id.to_le_bytes()[..]], bump)]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(seeds = [b"hand", game_state.key().as_ref()], bump)]
    pub hand_state: Box<Account<'info, HandState>>,
    
    #[account(seeds = [b"config"], bump)]
    pub config: Box<Account<'info, Config>>,
    
    #[account(mut, seeds = [b"escrow", game_state.key().as_ref()], bump)]
    pub escrow_account: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: The treasury wallet from the config, to be used in the callback.
    #[account(mut, address = config.treasury_wallet)]
    pub treasury_token_account: UncheckedAccount<'info>,
    
    /// CHECK: The dealer of the hand, who will receive the rent refund from HandState.
    #[account(mut)]
    pub dealer_account: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        space = 8 + SignerAccount::INIT_SPACE,
        payer = payer,
        seeds = [b"sign_pda"],
        bump,
    )]
    pub sign_pda_account: Box<Account<'info, SignerAccount>>,

    // --- Arcium Required Accounts ---
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut, address = derive_mempool_pda!())]
    /// CHECK: Checked by Arcium program
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!())]
    /// CHECK: Checked by Arcium program
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset))]
    /// CHECK: Checked by Arcium program
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(comp_def_offset("determine_winner")))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions sysvar
    pub instructions_sysvar: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

/// Handler for the `request_community_cards` instruction.
pub fn request_community_cards(
    ctx: Context<RequestCommunityCards>,
    computation_offset: u64,
) -> Result<()> {
    let phase_u8 = match ctx.accounts.game_state.game_phase {
        GamePhase::Flop => 0,
        GamePhase::Turn => 1,
        GamePhase::River => 2,
        _ => return err!(ErrorCode::InvalidAction),
    };
    
    let args = vec![Argument::PlaintextU8(phase_u8)]; // Client must also pass encrypted deck.
    
    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    let callback_accounts = String::new();

    queue_computation(ctx.accounts, computation_offset, args, Some(callback_accounts), vec![RevealCommunityCardsCallback::callback_ix(&[])])?;

    Ok(())
}

/// Handler for the `request_showdown` instruction.
pub fn request_showdown(ctx: Context<RequestShowdown>, computation_offset: u64) -> Result<()> {
    require!(
        ctx.accounts.game_state.game_phase == GamePhase::Showdown,
        ErrorCode::InvalidAction
    );
    // Ensure the provided dealer account matches the one in game state for rent refund.
    require!(
        ctx.accounts.game_state.players[ctx.accounts.game_state.dealer_index as usize] == ctx.accounts.dealer_account.key(),
        ErrorCode::Unauthorized
    );

    let args = vec![]; // Client will pass encrypted cards and board state.

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    let callback_accounts = String::new();

    queue_computation(ctx.accounts, computation_offset, args, Some(callback_accounts), vec![DetermineWinnerCallback::callback_ix(&[])])?;
    
    Ok(())
}