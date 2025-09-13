/**
 * @description
 * This is the main library file for the Veridian Hold'em on-chain program.
 * It serves as the entry point for all instructions and defines the program's overall structure.
 *
 * @dependencies
 * - anchor_lang: The core Anchor framework library.
 * - arcium_anchor: Arcium's extensions for Anchor, enabling confidential compute.
 * - Internal modules: `state`, `error`, and `instructions`.
 *
 * @notes
 * - The `#[arcium_program]` macro is used instead of Anchor's `#[program]` to enable
 *   integration with the Arcium confidential compute network.
 * - The program `declare_id!` must match the program ID in `Anchor.toml` and the deployed program address.
 */

use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

// Re-export modules to make their contents easily accessible to other parts of the program.
use instructions::*;
pub use state::*;

// The unique on-chain address of the Veridian Hold'em program.
declare_id!("Cd23WfyTo2XjmswN1n8WvcWARUJiTjXtK4wnLmwxh7in");

#[arcium_program]
pub mod veridian_holdem {
    use super::*;

    /// Initializes the global configuration for the platform.
    /// This instruction can only be called once by the designated program deployer/admin.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context containing accounts for initializing the config.
    /// * `treasury_wallet` - The public key of the wallet that will receive rake fees.
    /// * `rake_percentage` - The percentage of the pot to be taken as rake (e.g., 5 for 5%).
    /// * `rake_cap` - The maximum rake amount that can be taken from a single pot.
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

    /// Updates the rake configuration.
    /// Only the current admin, as stored in the `Config` account, can call this instruction.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context containing accounts for setting the rake config.
    /// * `rake_percentage` - The new rake percentage.
    /// * `rake_cap` - The new maximum rake amount.
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
}