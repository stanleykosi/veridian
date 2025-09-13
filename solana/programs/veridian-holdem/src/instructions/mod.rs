/**
 * @description
 * This file serves as the module manager for the `instructions` directory.
 * It publicly exports all the instruction modules, allowing the main `lib.rs` file
 * to access them through a single, clean `use` statement.
 *
 * This pattern helps in organizing the codebase as the number of instructions grows.
 */

// Declare the `admin` module, making its contents available within this scope.
pub mod admin;

// Publicly re-export all items from the `admin` module.
pub use admin::*;