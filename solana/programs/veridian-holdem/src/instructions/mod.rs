/**
 * @description
 * This file serves as the module manager for the `instructions` directory.
 * It publicly exports all the instruction modules, allowing the main `lib.rs` file
 * to access them through a single, clean `use` statement.
 *
 * This pattern helps in organizing the codebase as the number of instructions grows.
 */

// Declare the instruction modules, making their contents available within this scope.
pub mod admin;
pub mod create_table;
pub mod join_table;
pub mod deal_new_hand;
pub mod player_action;
pub mod request_cards;
pub mod leave_table;
pub mod crank_fold;

// Publicly re-export all items from the declared modules.
pub use admin::*;
pub use create_table::*;
pub use join_table::*;
pub use deal_new_hand::*;
pub use player_action::*;
pub use request_cards::*;
pub use leave_table::*;
pub use crank_fold::*;