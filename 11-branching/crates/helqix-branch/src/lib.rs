//! HELQIX Layer 11 — deterministic branching.
//!
//! A branch is an isolated continuation from an exact captured state and
//! logical time. Forks clone state values, retain explicit parentage and
//! provenance, and never modify their source branch. Comparisons describe
//! structural runtime-state differences without selecting a preferred outcome.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod branch;
pub mod branch_point;
pub mod branch_tree;
pub mod comparison;
pub mod state_fork;

pub use branch::{Branch, BranchError, BranchId};
pub use branch_point::{BranchPoint, BranchPointError};
pub use branch_tree::{BranchTree, BranchTreeError};
pub use comparison::{compare_branches, BranchComparison, ComponentChange};
pub use state_fork::{StateFork, StateForkError};
