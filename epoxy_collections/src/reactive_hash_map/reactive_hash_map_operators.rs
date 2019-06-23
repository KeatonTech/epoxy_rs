use crate::base_collection::ReadonlyReactiveCollection;
use crate::mutations::Mutation::{Property, Subproperty};
use crate::reactive_container_item::ReactiveContainerItem;
use super::reactive_hash_map::ReactiveHashMap;
use super::readonly_reactive_hash_map::ReadonlyReactiveHashMap;
use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;

impl<KeyType, ValueType> ReactiveHashMap<KeyType, ValueType>
where
    KeyType: Hash,
    KeyType: Eq,
    KeyType: Clone,
    KeyType: Send,
    KeyType: Sync,
    KeyType: 'static,
    ValueType: ReactiveContainerItem,
    ValueType: Sync,
    ValueType: Send,
    ValueType: 'static,
{
    /// Creates a new ReadonlyReactiveHashMap where each value is run through a mapper function.
    /// 
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    /// 
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// hash_map.insert(1, 10);
    /// 
    /// let mapped = hash_map.map(|value| value * 2);
    /// assert_eq!(*mapped.get(&1).unwrap(), 20);
    /// 
    /// hash_map.insert(2, 20);
    /// assert_eq!(*mapped.get(&2).unwrap(), 40);
    /// 
    /// hash_map.insert(1, 50);
    /// assert_eq!(*mapped.get(&1).unwrap(), 100);
    /// 
    /// hash_map.remove(1);
    /// assert_eq!(mapped.get(&1), None);
    /// ```
    pub fn map<U, F>(&self, mapper_fn: F) -> ReadonlyReactiveHashMap<KeyType, U>
    where
        U: ReactiveContainerItem,
        U: Send,
        U: Sync,
        U: Clone,
        U: 'static,
        F: Fn(&ValueType) -> U,
        F: Send,
        F: Sync,
        F: 'static,
    {
        let mut mapped_data;
        {
            let original_data = self.internal.map.internal.base.collection.read().unwrap();
            let mut original_mapped_data: HashMap<KeyType, U> = HashMap::new();
            for (key, value) in original_data.iter() {
                original_mapped_data.insert(key.clone(), mapper_fn(&*value));
            }

            mapped_data = ReactiveHashMap::copy_of(&original_mapped_data);
        }

        let readonly_mapped_data = mapped_data.as_readonly();

        let subscription_self_ref = self.clone_internal();
        let subscription = self.get_mutation_stream()
            .map(|mutation| match mutation {
                Property(mutation) => (*mutation.key.downcast_ref::<KeyType>().unwrap()).clone(),
                Subproperty(mutation) => (*mutation.key.downcast_ref::<KeyType>().unwrap()).clone(),
                _ => panic!("Invalid mutation type for ReactiveHashMap")
            })
            .subscribe(move |key| {
                if subscription_self_ref.contains_key(&key) {
                    mapped_data.insert((*key).clone(), mapper_fn(&*subscription_self_ref.get(&key).unwrap()));
                } else {
                    mapped_data.remove((*key).clone());
                }
            });

        {
            let mut extra_fields = readonly_mapped_data.internal.base.extra_fields.write().unwrap();
            *extra_fields = Some(Box::new(subscription));
        }

        readonly_mapped_data
    }
}