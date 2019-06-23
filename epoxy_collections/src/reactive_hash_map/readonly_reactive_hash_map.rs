use crate::base_collection::{
    BaseReactiveCollection, ReadonlyReactiveCollection, ReadonlyReactiveCollectionInternal,
};
use crate::mutations::Mutation;
use crate::mutations::Mutation::{Property, Subproperty};
use crate::reactive_container_item::ReactiveContainerItem;
use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

pub(super) struct ReadonlyReactiveHashMapInternal<KeyType, ValueType>
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
    pub(super) base: BaseReactiveCollection<HashMap<KeyType, Arc<ValueType>>>,
}

pub struct ReadonlyReactiveHashMap<KeyType, ValueType>
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
    pub(super) internal: Arc<ReadonlyReactiveHashMapInternal<KeyType, ValueType>>,
}

impl<KeyType, ValueType> ReadonlyReactiveHashMapInternal<KeyType, ValueType>
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
    pub fn get(&self, key: &KeyType) -> Option<Arc<ValueType>> {
        let read_guard = self.base.collection.read().unwrap();
        if let Some(value) = read_guard.get(key) {
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn contains_key(&self, key: &KeyType) -> bool {
        self.base.collection.read().unwrap().contains_key(key)
    }
}

impl<KeyType, ValueType> ReadonlyReactiveCollectionInternal
    for ReadonlyReactiveHashMap<KeyType, ValueType>
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
    type CollectionType = HashMap<KeyType, Arc<ValueType>>;

    fn get_collection(&self) -> &Box<RwLock<HashMap<KeyType, Arc<ValueType>>>> {
        &self.internal.base.collection
    }

    fn get_mutation_sink(&self) -> &epoxy_streams::Sink<Mutation> {
        &self.internal.base.mutation_sink
    }
}

impl<KeyType, ValueType> ReadonlyReactiveHashMap<KeyType, ValueType>
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
    pub(super) fn new() -> ReadonlyReactiveHashMap<KeyType, ValueType> {
        ReadonlyReactiveHashMap {
            internal: Arc::new(ReadonlyReactiveHashMapInternal {
                base: BaseReactiveCollection::new(HashMap::new()),
            }),
        }
    }

    pub(super) fn copy_of(
        data: &HashMap<KeyType, ValueType>,
    ) -> ReadonlyReactiveHashMap<KeyType, ValueType>
    where
        ValueType: Clone,
    {
        let mut copied_data: HashMap<KeyType, Arc<ValueType>> = HashMap::new();
        for (key, value) in data {
            copied_data.insert(key.clone(), Arc::new(value.clone()));
        }

        ReadonlyReactiveHashMap {
            internal: Arc::new(ReadonlyReactiveHashMapInternal {
                base: BaseReactiveCollection::new(copied_data),
            }),
        }
    }

    /// Retrieves the value for a given key, if it exists. The value is returned as an Arc,
    /// so holding a reference to it does not hold a reference to the HashMap. The value can
    /// be deleted from the HashMap and the Arc will still exist.
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    ///
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// hash_map.insert(1, 10);
    /// assert_eq!(*hash_map.get(&1).unwrap(), 10);
    /// ```
    pub fn get(&self, key: &KeyType) -> Option<Arc<ValueType>> {
        self.internal.get(key)
    }

    /// Returns true if a value exists for the given key.
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    ///
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// hash_map.insert(1, 10);
    /// assert_eq!(hash_map.contains_key(&1), true);
    /// assert_eq!(hash_map.contains_key(&2), false);
    /// ```
    pub fn contains_key(&self, key: &KeyType) -> bool {
        self.internal.contains_key(key)
    }

    /// Observes the value at a given key, returning a ReadonlyReactiveValue that will update
    /// whenever the value in the HashMap is updated. This ReadonlyReactiveValue can be converted
    /// into a stream as necessary.
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    /// use epoxy_streams::ReactiveValue;
    /// use std::sync::Arc;
    ///
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// let observer = hash_map.observe(&1);
    /// assert_eq!(*observer.get(), None);
    ///
    /// let update_count = observer.as_stream().count_values().to_reactive_value();
    /// assert_eq!(*update_count.get(), 0);
    ///
    /// hash_map.insert(1, 10);
    /// assert_eq!(*observer.get(), Some(Arc::new(10)));
    /// assert_eq!(*update_count.get(), 1);
    ///
    /// hash_map.insert(1, 100);
    /// assert_eq!(*observer.get(), Some(Arc::new(100)));
    /// assert_eq!(*update_count.get(), 2);
    ///
    /// hash_map.remove(1);
    /// assert_eq!(*observer.get(), None);
    /// assert_eq!(*update_count.get(), 3);
    /// ```
    pub fn observe(
        &self,
        key: &KeyType,
    ) -> epoxy_streams::ReadonlyReactiveValue<Option<Arc<ValueType>>> {
        let filter_key = key.clone();
        let map_key = key.clone();
        let map_internal = self.internal.clone();

        self.get_mutation_stream()
            .filter(move |mutation| match mutation {
                Property(property_mutation) => {
                    property_mutation.key.downcast_ref::<KeyType>().unwrap() == &filter_key
                }
                Subproperty(subproperty_mutation) => {
                    subproperty_mutation.key.downcast_ref::<KeyType>().unwrap() == &filter_key
                }
                _ => panic!("Incompatible mutation type for ReactiveHashMap"),
            })
            .map(move |_mutation| map_internal.get(&map_key))
            .to_reactive_value_with_default(self.get(&key))
    }
}

impl<KeyType, ValueType> Clone for ReadonlyReactiveHashMap<KeyType, ValueType>
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
    /// Returns a clone of the ReadonlyReactiveHashMap. Note that the clone will behave
    /// *identically* to the original hash map as they will both be tied to the same source.
    /// Clone should therefore be used in much the same way you would use it for an Rc/Arc
    /// pointer (in fact, it's using Arc behind the scenes).
    fn clone(&self) -> ReadonlyReactiveHashMap<KeyType, ValueType> {
        ReadonlyReactiveHashMap {
            internal: self.internal.clone(),
        }
    }
}
