use std::any::Any;
use std::sync::Arc;

pub enum Mutation {
    Property(PropertyMutation),
    Subproperty(SubpropertyMutation),
    Insertion(InsertionMutation),
    Deleton(DeletionMutation),
}

/// Represents a single-property mutation in a key-value store such as a Map, Vec (where the key is
/// the index of the changed item), or struct.
pub struct PropertyMutation {
    pub key: Box<dyn Any + Send + Sync + 'static>,
    pub old_value: Option<Arc<dyn Any + Send + Sync>>,
    pub new_value: Option<Arc<dyn Any + Send + Sync>>,
}

/// Represents one or more items being inserted into an array. When index is unset, the new values
/// should be inserted at the end of the array.
pub struct InsertionMutation {
    pub new_values: Box<Iterator<Item = Arc<dyn Any + Send + Sync>> + Send + Sync>,
    pub index: Option<usize>,
}

/// Represents one or more items being removed from an array.
pub struct DeletionMutation {
    pub removed_values: Box<Iterator<Item = Arc<dyn Any + Send + Sync>> + Send + Sync>,
    pub index: usize,
}

/// Represents a mutation to a sub-property of a nested structure, for example an item getting
/// inserted into the inner array of an array of arrays.
pub struct SubpropertyMutation {
    pub key: Box<dyn Any + Send + Sync + 'static>,
    pub mutation: Arc<Mutation>,
}
