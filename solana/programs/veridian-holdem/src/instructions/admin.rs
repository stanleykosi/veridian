/**
 * @description
 * This file defines the account contexts for administrative instructions related to the
 * Veridian Hold'em platform. These instructions manage the global `Config` account.
 *
 * @dependencies
 * - anchor_lang: The core Anchor framework library.
 * - crate::state: Defines the `Config` account structure.
 * - crate::error: Defines custom error codes for validation.
 *
 * @notes
 * - The use of Anchor constraints (`constraint = ...`) is critical for enforcing on-chain
 *   authorization, ensuring that only the designated admin can perform sensitive actions.
 */

use crate::error::ErrorCode;
use crate::state::Config;
use anchor_lang::prelude::*;

/// Defines the accounts required to initialize the global configuration PDA.
/// This instruction should only be executed once during the initial deployment and setup
/// of the platform.
#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    /// The `Config` account to be created.
    /// It's a PDA seeded with "config", ensuring there's only one such account for the program.
    /// The space is calculated automatically based on the `Config` struct definition.
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    /// The signer of the transaction, who will be set as the initial administrator.
    /// This account pays for the creation of the `Config` account.
    #[account(mut)]
    pub admin: Signer<'info>,

    /// The Solana System Program, required by Anchor for creating new accounts.
    pub system_program: Program<'info, System>,
}

/// Defines the accounts required to update the rake settings in the global configuration PDA.
/// This instruction can be called by the current admin to adjust the platform's rake structure.
#[derive(Accounts)]
pub struct SetRakeConfig<'info> {
    /// The global `Config` account to be modified.
    /// A crucial security constraint ensures that the transaction signer's public key
    /// matches the `admin` key stored within the `Config` account itself. If this check fails,
    /// the transaction will be rejected with the `Unauthorized` error.
    #[account(
        mut,
        seeds = [b"config"],
        bump,
        constraint = config.admin == admin.key() @ ErrorCode::Unauthorized
    )]
    pub config: Account<'info, Config>,

    /// The signer of the transaction, who must be the current administrator.
    pub admin: Signer<'info>,
}

/// The handler function for the `initialize_config` instruction.
pub fn initialize_config(
    ctx: Context<InitializeConfig>,
    treasury_wallet: Pubkey,
    rake_percentage: u8,
    rake_cap: u64,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.treasury_wallet = treasury_wallet;
    config.rake_percentage = rake_percentage;
    config.rake_cap = rake_cap;
    Ok(())
}

/// The handler function for the `set_rake_config` instruction.
pub fn set_rake_config(
    ctx: Context<SetRakeConfig>,
    rake_percentage: u8,
    rake_cap: u64,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.rake_percentage = rake_percentage;
    config.rake_cap = rake_cap;
    Ok(())
}