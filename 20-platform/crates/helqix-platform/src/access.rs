//! Explicit project/workspace scope checks. Authentication is external.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Principal { pub id: String, pub workspace_id: String }
impl Principal {
    pub fn new(id: impl Into<String>, workspace_id: impl Into<String>) -> Result<Self, AccessError> {
        let value = Self { id: id.into(), workspace_id: workspace_id.into() };
        if value.id.trim().is_empty() || value.workspace_id.trim().is_empty() { return Err(AccessError::EmptyIdentity); }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceScope { pub workspace_id: String, pub project_id: Option<String> }
impl ResourceScope {
    pub fn workspace(id: impl Into<String>) -> Self { Self { workspace_id: id.into(), project_id: None } }
    pub fn project(workspace_id: impl Into<String>, project_id: impl Into<String>) -> Self { Self { workspace_id: workspace_id.into(), project_id: Some(project_id.into()) } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action { Read, Create, Update, Delete, Execute, Export }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDecision { Allow, DenyWorkspaceMismatch, DenyProjectNotGranted, DenyActionNotGranted }

/// Deterministic role binding policy. No implicit cross-workspace access is possible.
#[derive(Debug, Clone, Default)]
pub struct AccessPolicy { bindings: BTreeMap<(String, String), RoleBinding> }
#[derive(Debug, Clone)]
struct RoleBinding { actions: BTreeSet<Action>, projects: Option<BTreeSet<String>> }
impl Default for RoleBinding { fn default() -> Self { Self { actions: BTreeSet::new(), projects: Some(BTreeSet::new()) } } }
impl AccessPolicy {
    pub fn grant(&mut self, principal: &Principal, actions: impl IntoIterator<Item=Action>, projects: Option<impl IntoIterator<Item=String>>) {
        let binding = self.bindings.entry((principal.workspace_id.clone(), principal.id.clone())).or_default();
        binding.actions.extend(actions);
        match (binding.projects.as_mut(), projects) {
            // An unrestricted grant is the union of all grants. A later
            // project-specific grant must not accidentally narrow it.
            (None, Some(_)) => {}
            (Some(granted), Some(projects)) => granted.extend(projects),
            (_, None) => binding.projects = None,
        }
    }
    pub fn decide(&self, principal: &Principal, scope: &ResourceScope, action: Action) -> AccessDecision {
        if principal.workspace_id != scope.workspace_id { return AccessDecision::DenyWorkspaceMismatch; }
        let Some(binding) = self.bindings.get(&(principal.workspace_id.clone(), principal.id.clone())) else { return AccessDecision::DenyActionNotGranted; };
        if !binding.actions.contains(&action) { return AccessDecision::DenyActionNotGranted; }
        // A project allow-list does not imply access to workspace-wide data.
        // Only an explicit workspace grant (`projects: None`) may authorize
        // workspace-scope operations; otherwise a project grant could be used
        // to read or mutate resources spanning every project in the tenant.
        if let Some(allowed) = &binding.projects {
            let Some(project) = &scope.project_id else {
                return AccessDecision::DenyProjectNotGranted;
            };
            if !allowed.contains(project) { return AccessDecision::DenyProjectNotGranted; }
        }
        AccessDecision::Allow
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessError { EmptyIdentity }
impl std::fmt::Display for AccessError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("principal and workspace identifiers must not be empty") } }
impl std::error::Error for AccessError {}

#[cfg(test)]
mod tests {
    use super::*;
    fn principal() -> Principal { Principal::new("user-a", "workspace-a").unwrap() }
    #[test] fn cross_workspace_is_denied_even_with_project_grant() { let p = principal(); let mut policy = AccessPolicy::default(); policy.grant(&p, [Action::Read], Some(["project-a".to_owned()])); assert_eq!(policy.decide(&p, &ResourceScope::project("workspace-b", "project-a"), Action::Read), AccessDecision::DenyWorkspaceMismatch); }
    #[test] fn project_scope_and_action_are_both_checked() { let p = principal(); let mut policy = AccessPolicy::default(); policy.grant(&p, [Action::Read], Some(["p1".into()])); assert_eq!(policy.decide(&p, &ResourceScope::project("workspace-a", "p2"), Action::Read), AccessDecision::DenyProjectNotGranted); assert_eq!(policy.decide(&p, &ResourceScope::workspace("workspace-a"), Action::Execute), AccessDecision::DenyActionNotGranted); }
    #[test] fn project_grant_does_not_authorize_workspace_wide_resources() { let p = principal(); let mut policy = AccessPolicy::default(); policy.grant(&p, [Action::Read], Some(["p1".into()])); assert_eq!(policy.decide(&p, &ResourceScope::workspace("workspace-a"), Action::Read), AccessDecision::DenyProjectNotGranted); }
    #[test] fn explicit_workspace_grant_allows_workspace_scope() { let p = principal(); let mut policy = AccessPolicy::default(); policy.grant(&p, [Action::Read], None::<Vec<String>>); assert_eq!(policy.decide(&p, &ResourceScope::workspace("workspace-a"), Action::Read), AccessDecision::Allow); assert_eq!(policy.decide(&p, &ResourceScope::project("workspace-a", "new-project"), Action::Read), AccessDecision::Allow); }
    #[test] fn adding_project_grant_does_not_narrow_existing_workspace_grant() { let p = principal(); let mut policy = AccessPolicy::default(); policy.grant(&p, [Action::Read], None::<Vec<String>>); policy.grant(&p, [Action::Read], Some(["p1".to_owned()])); assert_eq!(policy.decide(&p, &ResourceScope::project("workspace-a", "p2"), Action::Read), AccessDecision::Allow); }
}
