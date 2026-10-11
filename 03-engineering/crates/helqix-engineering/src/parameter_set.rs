use std::{collections::BTreeMap, error::Error, fmt};
use helqix_core::id::Identifier;
use crate::parameter::ParameterDefinition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterSetError { DuplicateParameter { name: String }, NameMismatch { key: String, parameter: String } }
impl fmt::Display for ParameterSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self { Self::DuplicateParameter{name} => write!(f,"duplicate parameter: {name}"), Self::NameMismatch{key,parameter} => write!(f,"parameter key {key} does not match {parameter}") } }
}
impl Error for ParameterSetError {}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParameterSet { parameters: BTreeMap<Identifier, ParameterDefinition> }
impl ParameterSet {
    pub fn new() -> Self { Self::default() }
    pub fn len(&self) -> usize { self.parameters.len() }
    pub fn is_empty(&self) -> bool { self.parameters.is_empty() }
    pub fn contains(&self, name: &Identifier) -> bool { self.parameters.contains_key(name) }
    pub fn register(&mut self, value: ParameterDefinition) -> Result<(), ParameterSetError> {
        let name = value.name().clone();
        if self.parameters.contains_key(&name) { return Err(ParameterSetError::DuplicateParameter { name: name.as_str().to_owned() }); }
        self.parameters.insert(name, value); Ok(())
    }
    pub fn add(&mut self, value: ParameterDefinition) -> Result<(), ParameterSetError> { self.register(value) }
    pub fn get(&self, name: &Identifier) -> Option<&ParameterDefinition> { self.parameters.get(name) }
    pub fn get_mut(&mut self, name: &Identifier) -> Option<&mut ParameterDefinition> { self.parameters.get_mut(name) }
    pub fn iter(&self) -> impl Iterator<Item=(&Identifier, &ParameterDefinition)> { self.parameters.iter() }
    pub fn values(&self) -> impl Iterator<Item=&ParameterDefinition> { self.parameters.values() }
    pub fn validate(&self) -> Result<(), ParameterSetError> {
        for (key, value) in &self.parameters { if key != value.name() { return Err(ParameterSetError::NameMismatch { key:key.as_str().into(), parameter:value.name().as_str().into() }); } }
        Ok(())
    }
}

