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

// Publicly re-export all items from the declared modules.
pub use admin::*;
pub use create_table::*;
pub use join_table::*;