use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::{CircuitSource, OffChainCircuitSource};

pub mod callbacks;
pub mod error;
pub mod instructions;
pub mod state;

// Re-export modules to make their contents easily accessible to other parts of the program.
use instructions::*;
pub use state::*;

// The unique on-chain address of the Veridian Hold'em program.
declare_id!("Grax8NuUaPo4bA43PiYkAhdLvU7Vts2o8Gk16TdV6ZEQ");

#[arcium_program]
pub mod veridian_holdem {
    use super::*;

    /// Initializes the global configuration for the platform.
    /// This instruction can only be called once by the designated program deployer/admin.
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        treasury_wallet: Pubkey,
        rake_percentage: u8,
        rake_cap: u64,
    ) -> Result<()> {
        instructions::admin::initialize_config(ctx, treasury_wallet, rake_percentage, rake_cap)
    }

    /// Updates the rake configuration.
    /// Only the current admin, as stored in the `Config` account, can call this instruction.
    pub fn set_rake_config(
        ctx: Context<SetRakeConfig>,
        rake_percentage: u8,
        rake_cap: u64,
    ) -> Result<()> {
        instructions::admin::set_rake_config(ctx, rake_percentage, rake_cap)
    }

    /// Creates a new poker table with a specific configuration.
    pub fn create_table(
        ctx: Context<CreateTable>,
        table_id: u64,
        small_blind: u64,
        big_blind: u64,
        buy_in: u64,
    ) -> Result<()> {
        instructions::create_table::create_table(ctx, table_id, small_blind, big_blind, buy_in)
    }

    /// Allows a second player to join an existing, open poker table.
    pub fn join_table(ctx: Context<JoinTable>) -> Result<()> {
        instructions::join_table::join_table(ctx)
    }

    /// Step A: prepare accounts for a new hand (no Arcium queue here).
    pub fn deal_new_hand_setup(ctx: Context<DealNewHandSetup>, computation_offset: u64) -> Result<()> {
        instructions::deal_new_hand::deal_new_hand_setup(ctx, computation_offset)
    }

    /// Step B: queue the confidential shuffle and deal computation with a minimal Arcium context.
    pub fn deal_new_hand_queue(ctx: Context<DealNewHandQueue>, computation_offset: u64) -> Result<()> {
        instructions::deal_new_hand::deal_new_hand_queue(ctx, computation_offset)
    }

    /// Processes a player's action (Fold, Check, Call, Bet, Raise).
    pub fn player_action(ctx: Context<PlayerAction>, action: Action) -> Result<()> {
        instructions::player_action::player_action(ctx, action)
    }

    /// Requests the reveal of the next community cards (Flop, Turn, River).
    pub fn request_community_cards(
        ctx: Context<RequestCommunityCards>,
        computation_offset: u64,
    ) -> Result<()> {
        instructions::request_cards::request_community_cards(ctx, computation_offset)
    }

    /// Requests the confidential showdown computation to determine the winner.
    pub fn request_showdown(ctx: Context<RequestShowdown>, computation_offset: u64) -> Result<()> {
        instructions::request_cards::request_showdown(ctx, computation_offset)
    }

    /// Allows a player to leave the table and withdraw their funds.
    pub fn leave_table(ctx: Context<LeaveTable>) -> Result<()> {
        instructions::leave_table::leave_table(ctx)
    }

    /// A permissionless instruction to fold on behalf of a player whose turn timer has expired.
    pub fn crank_fold(ctx: Context<CrankFold>) -> Result<()> {
        instructions::crank_fold::crank_fold(ctx)
    }

    // --- Arcium Callbacks ---
    // Callbacks are defined in the callbacks module

    // --- Arcium Comp Def Initializers ---
    // These instructions are required to register the Arcis circuits on-chain.
    
    pub fn init_shuffle_and_deal_comp_def(ctx: Context<InitShuffleAndDealCompDef>) -> Result<()> {
        init_comp_def(
            ctx.accounts,
            true,
            0,
            Some(CircuitSource::OffChain(OffChainCircuitSource {
                source: "https://arcium.s3.us-east-1.amazonaws.com/shuffle_and_deal_testnet.arcis".to_string(),
                hash: [0; 32],
            })),
            None,
        )?;
        Ok(())
    }
    
    pub fn init_reveal_community_cards_comp_def(ctx: Context<InitRevealCommunityCardsCompDef>) -> Result<()> {
        init_comp_def(
            ctx.accounts,
            true,
            0,
            Some(CircuitSource::OffChain(OffChainCircuitSource {
                source: "https://arcium.s3.us-east-1.amazonaws.com/reveal_community_cards_testnet.arcis".to_string(),
                hash: [0; 32],
            })),
            None,
        )?;
        Ok(())
    }

    pub fn init_determine_winner_comp_def(ctx: Context<InitDetermineWinnerCompDef>) -> Result<()> {
        init_comp_def(
            ctx.accounts,
            true,
            0,
            Some(CircuitSource::OffChain(OffChainCircuitSource {
                source: "https://arcium.s3.us-east-1.amazonaws.com/determine_winner_testnet.arcis".to_string(),
                hash: [0; 32],
            })),
            None,
        )?;
        Ok(())
    }
}

// --- Arcium Comp Def Contexts ---
#[init_computation_definition_accounts("shuffle_and_deal", payer)]
#[derive(Accounts)]
pub struct InitShuffleAndDealCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK: This account is validated by the Arcium program
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("reveal_community_cards", payer)]
#[derive(Accounts)]
pub struct InitRevealCommunityCardsCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK: This account is validated by the Arcium program
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("determine_winner", payer)]
#[derive(Accounts)]
pub struct InitDetermineWinnerCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK: This account is validated by the Arcium program
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}