//! HELQIX Layer 12 — deterministic engineering search.
//!
//! This crate generates candidate parameter assignments, applies declared
//! constraints, delegates each valid candidate to a caller-supplied evaluator,
//! and ranks resulting objective metrics. It does not own simulation physics,
//! mutate a reality, or assume a particular execution engine.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod candidate;
pub mod constraints;
pub mod execution_scheduler;
pub mod objective;
pub mod optimizer;
pub mod ranking;
pub mod sampler;
pub mod search_space;
pub mod variable;

pub use candidate::{Candidate, CandidateId, CandidateStatus};
pub use constraints::{ConstraintOperator, ConstraintTarget, ConstraintViolation, SearchConstraint, SearchConstraintError};
pub use execution_scheduler::{CandidateEvaluator, EvaluationError, EvaluationFailurePolicy, ExecutionReport, ExecutionScheduler, SchedulerError};
pub use objective::{Evaluation, Objective, ObjectiveDirection, ObjectiveError};
pub use optimizer::{OptimizationResult, Optimizer, OptimizerError};
pub use ranking::{rank_candidates, RankedCandidate, RankingError};
pub use sampler::{Sampler, SamplerError};
pub use search_space::{SearchSpace, SearchSpaceError};
pub use variable::{Variable, VariableDomain, VariableError};
