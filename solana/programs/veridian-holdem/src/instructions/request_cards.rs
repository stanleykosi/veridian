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
    error::ErrorCode,
    state::{GamePhase, GameState, HandState, SignerAccount},
    ID, ID_CONST,
};
use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::accounts::{FeePool, ClockAccount};

/// Accounts for requesting the reveal of community cards (Flop, Turn, River).
#[queue_computation_accounts("reveal_community_cards", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct RequestCommunityCards<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut, seeds = [b"game", &game_state.table_config.key().to_bytes()[..]], bump)]
    pub game_state: Account<'info, GameState>,

    #[account(seeds = [b"hand", game_state.key().as_ref()], bump)]
    pub hand_state: Account<'info, HandState>,

    /// Required signer PDA for Arcium operations
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [b"sign_pda"],
        bump,
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,

    // --- Arcium Required Accounts (simplified) ---
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    
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
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    
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

    #[account(mut, seeds = [b"game", &game_state.table_config.key().to_bytes()[..]], bump)]
    pub game_state: Account<'info, GameState>,

    #[account(seeds = [b"hand", game_state.key().as_ref()], bump)]
    pub hand_state: Account<'info, HandState>,

    /// Required signer PDA for Arcium operations
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [b"sign_pda"],
        bump,
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,

    // --- Arcium Required Accounts (simplified) ---
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    
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
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(mut, address = derive_cluster_pda!(mxe_account))]
    pub cluster_account: Account<'info, Cluster>,
    
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    
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

    // TODO: Pass the encrypted deck from `hand_state` as an argument.
    // This requires client-side logic to fetch and pass account data, which is not yet implemented.
    // For now, passing placeholder arguments.
    let args = vec![
        // Argument::Encrypted(...) for the deck
        Argument::PlaintextU8(phase_u8),
    ];
    
    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
    queue_computation(ctx.accounts, computation_offset, args, None, vec![])?;

    Ok(())
}

/// Handler for the `request_showdown` instruction.
pub fn request_showdown(ctx: Context<RequestShowdown>, computation_offset: u64) -> Result<()> {
    require!(
        ctx.accounts.game_state.game_phase == GamePhase::Showdown,
        ErrorCode::InvalidAction
    );

    // TODO: Pass encrypted hole cards from `hand_state` and community cards from `game_state`.
    let args = vec![
        // Arguments for p1_cards, p2_cards, and board
    ];

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
    queue_computation(ctx.accounts, computation_offset, args, None, vec![])?;
    
    Ok(())
}