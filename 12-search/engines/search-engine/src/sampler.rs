//! Deterministic bounded Cartesian sampling of a finite search space.
use std::collections::BTreeMap;
use crate::{candidate::{Candidate, CandidateId}, search_space::{SearchSpace, SearchSpaceError}};

/// Stateless sampler that emits a deterministic prefix of the Cartesian product.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sampler { limit: usize }
impl Sampler {
    /// Creates a sampler with a hard maximum candidate count.
    pub fn new(limit: usize) -> Result<Self, SamplerError> { if limit == 0 { Err(SamplerError::ZeroLimit) } else { Ok(Self { limit }) } }
    /// Maximum number of candidates this sampler may allocate.
    pub fn limit(&self) -> usize { self.limit }

    /// Generates at most `limit` assignments in canonical variable order.
    /// The final variable changes fastest; the same space and limit always
    /// produce the same candidate IDs and values.
    pub fn sample(&self, space: &SearchSpace) -> Result<Vec<Candidate>, SamplerError> {
        let cardinality = space.candidate_count().map_err(SamplerError::SearchSpace)?;
        let count = cardinality.min(self.limit);
        let variables = space.variables().collect::<Vec<_>>();
        let mut indices = vec![0usize; variables.len()];
        let mut candidates = Vec::with_capacity(count);
        for ordinal in 0..count {
            let id = CandidateId::try_from(ordinal).map_err(|_| SamplerError::CandidateIdOverflow)?;
            let mut assignments = BTreeMap::new();
            for (variable, index) in variables.iter().zip(indices.iter()) {
                let value = variable.value_at(*index).ok_or(SamplerError::InvalidDomainIndex)?;
                assignments.insert(variable.id().clone(), value);
            }
            candidates.push(Candidate::new(id, assignments));
            for dimension in (0..indices.len()).rev() {
                indices[dimension] += 1;
                if indices[dimension] < variables[dimension].value_count() { break; }
                indices[dimension] = 0;
            }
        }
        Ok(candidates)
    }
}

/// Sampling configuration or generation error.
#[derive(Debug, Clone, PartialEq)]
pub enum SamplerError {
    /// Candidate limit cannot be zero.
    ZeroLimit,
    /// Search space could not provide a finite cardinality.
    SearchSpace(SearchSpaceError),
    /// A variable returned no value for a valid sample index.
    InvalidDomainIndex,
    /// Candidate ID cannot represent the generated ordinal.
    CandidateIdOverflow,
}
impl std::fmt::Display for SamplerError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self {
    Self::ZeroLimit => f.write_str("sampler limit must be greater than zero"), Self::SearchSpace(error) => write!(f, "cannot sample search space: {error}"),
    Self::InvalidDomainIndex => f.write_str("search variable produced an invalid domain index"), Self::CandidateIdOverflow => f.write_str("candidate ordinal exceeds its ID type"),
} } }
impl std::error::Error for SamplerError { fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { match self { Self::SearchSpace(error) => Some(error), _ => None } } }

#[cfg(test)] mod tests {
    use super::*;
    use helqix_core::Identifier;
    use crate::Variable;
    #[test] fn emits_stable_cartesian_prefix() { let mut space = SearchSpace::new(); space.add_variable(Variable::discrete(Identifier::parse("a").unwrap(), [1.0, 2.0]).unwrap()).unwrap(); space.add_variable(Variable::discrete(Identifier::parse("b").unwrap(), [10.0, 20.0]).unwrap()).unwrap(); let candidates = Sampler::new(3).unwrap().sample(&space).unwrap(); assert_eq!(candidates.iter().map(|c| (c.value(&Identifier::parse("a").unwrap()).unwrap(), c.value(&Identifier::parse("b").unwrap()).unwrap())).collect::<Vec<_>>(), vec![(1.0, 10.0), (1.0, 20.0), (2.0, 10.0)]); }
    #[test] fn rejects_unbounded_allocation_request_zero() { assert_eq!(Sampler::new(0), Err(SamplerError::ZeroLimit)); }
}
