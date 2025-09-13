/**
 * @description
 * This file is the main library for the Arcis confidential instructions crate.
 * It defines the `circuits` module, which contains all the functions that will be
 * compiled into secure MPC circuits for execution on the Arcium network.
 *
 * It also includes other necessary modules, such as `hand_eval`, which provides
 * core logic used by the confidential instructions.
 *
 * @modules
 * - hand_eval: Contains the data-independent logic for evaluating poker hands.
 *
 * @dependencies
 * - arcis_imports: Imports all necessary types and functions for writing Arcis code.
 */
use arcis_imports::*;

// Make the hand evaluation logic available to other confidential instructions in this crate.
pub mod hand_eval;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    // This is a placeholder encrypted instruction.
    // It will be replaced with actual game logic in later steps, such as
    // shuffle_and_deal, reveal_community_cards, and determine_winner.
    pub struct PlaceholderInput {
        value: u8,
    }

    #[instruction]
    pub fn placeholder(input_ctxt: Enc<Shared, PlaceholderInput>) -> Enc<Shared, u8> {
        let input = input_ctxt.to_arcis();
        let result = input.value;
        input_ctxt.owner.from_arcis(result)
    }
}