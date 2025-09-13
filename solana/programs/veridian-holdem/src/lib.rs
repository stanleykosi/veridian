/**
 * @description
 * This is the main library file for the Veridian Hold'em on-chain program.
 * It serves as the entry point for all instructions and defines the program's overall structure.
 *
 * @dependencies
 * - anchor_lang: The core Anchor framework library.
 * - arcium_anchor: Arcium's extensions for Anchor, enabling confidential compute.
 * - Internal modules: `state` and `error`.
 *
 * @notes
 * - The `#[arcium_program]` macro is used instead of Anchor's `#[program]` to enable
 *   integration with the Arcium confidential compute network.
 * - The program `declare_id!` must match the program ID in `Anchor.toml` and the deployed program address.
 */

use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

pub mod error;
pub mod state;

// Re-export modules to make their contents easily accessible to other parts of the program.
pub use state::*;

// The unique on-chain address of the Veridian Hold'em program.
declare_id!("Cd23WfyTo2XjmswN1n8WvcWARUJiTjXtK4wnLmwxh7in");

#[arcium_program]
pub mod veridian_holdem {
    use super::*;

    // Note: All game logic instructions (e.g., create_table, player_action) will be added here
    // in subsequent implementation steps.

    /// A temporary placeholder instruction to ensure the program is valid and can be compiled.
    /// This will be removed once the actual game instructions are implemented.
    pub fn placeholder(_ctx: Context<Placeholder>) -> Result<()> {
        Ok(())
    }
}

/// The account context for the placeholder instruction.
#[derive(Accounts)]
pub struct Placeholder {}