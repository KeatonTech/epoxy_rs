use super::readonly_reactive_hash_map::ReadonlyReactiveHashMap;
use crate::base_collection::{ReactiveCollectionInternal, ReadonlyReactiveCollectionInternal};
use crate::mutations::Mutation::{Property, Subproperty};
use crate::mutations::{Mutation, PropertyMutation, SubpropertyMutation};
use crate::reactive_container_item::ReactiveContainerItem;
use std::any::Any;
use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

pub(super) struct ReactiveHashMapInternal<KeyType, ValueType>
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
    pub(super) map: ReadonlyReactiveHashMap<KeyType, ValueType>,
    subproperty_mutation_subscriptions:
        RwLock<HashMap<KeyType, epoxy_streams::Subscription<Mutation>>>,
    property_input_subscriptions: RwLock<HashMap<KeyType, epoxy_streams::Subscription<ValueType>>>,
}

pub struct ReactiveHashMap<KeyType, ValueType>
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
    pub(super) internal: Arc<ReactiveHashMapInternal<KeyType, ValueType>>,
}

impl<KeyType, ValueType> ReadonlyReactiveCollectionInternal for ReactiveHashMap<KeyType, ValueType>
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
        self.internal.map.get_collection()
    }

    fn get_mutation_sink(&self) -> &epoxy_streams::Sink<Mutation> {
        self.internal.map.get_mutation_sink()
    }
}

impl<KeyType, ValueType> ReactiveCollectionInternal for ReactiveHashMap<KeyType, ValueType>
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
    fn apply_mutation(
        &self,
        collection: &mut HashMap<KeyType, Arc<ValueType>>,
        mutation: &Mutation,
    ) {
        match mutation {
            Property(mutation) => {
                let key = mutation.key.downcast_ref::<KeyType>().unwrap().clone();
                if let Some(value_box) = &mutation.new_value {
                    let local_value = value_box.clone();
                    let value = local_value.downcast::<ValueType>().unwrap();
                    self.subscribe_to_mutation_stream(&key, &value);
                    collection.insert(key, value);
                } else {
                    self.internal
                        .subproperty_mutation_subscriptions
                        .write()
                        .unwrap()
                        .remove(&key);
                    collection.remove(&key);
                }
            }
            Subproperty(mutation) => {
                let key = mutation.key.downcast_ref::<KeyType>().unwrap().clone();
                if let Some(value) = collection.get(&key) {
                    value.write_mutations(vec![mutation.mutation.clone()]);
                }
            }
            _ => panic!("Invalid mutation received on ReactiveHashMap."),
        }
    }
}

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
    /// Creates a new empty ReactiveHashMap.
    pub fn new() -> ReactiveHashMap<KeyType, ValueType> {
        ReactiveHashMap {
            internal: Arc::new(ReactiveHashMapInternal {
                map: ReadonlyReactiveHashMap::new(),
                subproperty_mutation_subscriptions: RwLock::new(HashMap::new()),
                property_input_subscriptions: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Creates a new ReactiveHashMap by copying a non-reactive HashMap.
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    /// use std::collections::HashMap;
    ///
    /// let mut static_data = HashMap::new();
    /// static_data.insert(1, 2);
    ///
    /// let reactive_data = ReactiveHashMap::copy_of(&static_data);
    /// assert_eq!(*reactive_data.get(&1).unwrap(), 2);
    ///
    /// static_data.insert(1, 10);
    /// assert_eq!(*reactive_data.get(&1).unwrap(), 2);
    ///
    /// reactive_data.insert(1, 9);
    /// assert_eq!(*reactive_data.get(&1).unwrap(), 9);
    /// assert_eq!(static_data.get(&1).unwrap(), &10);
    /// ```
    pub fn copy_of(data: &HashMap<KeyType, ValueType>) -> ReactiveHashMap<KeyType, ValueType>
    where
        ValueType: Clone,
    {
        let ret = ReactiveHashMap::copy_of_internal(data);
        for (key, value) in data {
            ReactiveHashMap::subscribe_to_mutation_stream(&ret, key, value);
        }
        ret
    }

    /// Similar to copy_of but does not add subproperty mutation subscriptions. This function
    /// is used internally for operators, where subproperty mutation subscriptions would be
    /// redundant.
    fn copy_of_internal(data: &HashMap<KeyType, ValueType>) -> ReactiveHashMap<KeyType, ValueType>
    where
        ValueType: Clone,
    {
        ReactiveHashMap {
            internal: Arc::new(ReactiveHashMapInternal {
                map: ReadonlyReactiveHashMap::copy_of(data),
                subproperty_mutation_subscriptions: RwLock::new(HashMap::new()),
                property_input_subscriptions: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Subscribes to mutations on a value, if necessary.
    fn subscribe_to_mutation_stream(&self, key: &KeyType, value: &ValueType) {
        if let Some(stream) = value.get_item_mutation_stream() {
            let stream_key = key.clone();
            let subscription = stream
                .map_rc(move |mutation| {
                    Arc::new(Mutation::Subproperty(SubpropertyMutation {
                        key: Box::new(stream_key.clone()),
                        mutation: mutation.clone(),
                    }))
                })
                .pipe_into(self.get_mutation_sink());

            self.internal
                .subproperty_mutation_subscriptions
                .write()
                .unwrap()
                .insert(key.clone(), subscription);
        }
    }

    /// Returns a version of the data structure that will update whenever this structure changes,
    /// but can not initiate changes itself. This can be used to expose reactive APIs without
    /// worrying about users tampering with the data.
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    ///
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// let readonly_hash_map = hash_map.as_readonly();
    ///
    /// hash_map.insert(1, 10);
    /// assert_eq!(*readonly_hash_map.get(&1).unwrap(), 10);
    /// ```
    pub fn as_readonly(&self) -> ReadonlyReactiveHashMap<KeyType, ValueType> {
        self.internal.map.clone()
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
        self.internal.map.get(key)
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
        self.internal.map.contains_key(key)
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
        self.internal.map.observe(key)
    }

    fn get_dynamic(&self, key: &KeyType) -> Option<Arc<dyn Any + Send + Sync>> {
        match self.get(key) {
            Some(arc) => Some(arc),
            None => None,
        }
    }

    /// Removes and returns the value for a given key, if it exists.
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    /// use std::sync::Arc;
    ///
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// hash_map.insert(1, 10);
    /// assert_eq!(hash_map.remove(1), Some(Arc::new(10)));
    /// assert_eq!(hash_map.remove(1), None);
    /// ```
    pub fn remove(&self, key: KeyType) -> Option<Arc<ValueType>> {
        self.remove_input_subscription(&key);
        let old_value = self.get(&key);
        match old_value {
            Some(old_value_arc) => {
                self.write_mutations(vec![Arc::new(Mutation::Property(PropertyMutation {
                    key: Box::new(key),
                    old_value: Some(old_value_arc.clone()),
                    new_value: None,
                }))]);
                Some(old_value_arc)
            }
            None => None,
        }
    }

    /// Ensures that any insert_stream or insert_reactive_value subscriptions are dropped for this key.
    fn remove_input_subscription(&self, key: &KeyType) {
        let mut has_input_subscription = false;
        if let Ok(read_input_subscriptions) = self.internal.property_input_subscriptions.read() {
            if read_input_subscriptions.contains_key(&key) {
                has_input_subscription = true;
            }
        }
        if has_input_subscription {
            self.internal
                .property_input_subscriptions
                .write()
                .unwrap()
                .remove(&key);
        }
    }

    /// Inserts a key-value pair into the map.
    /// If the map did not have this key present, None is returned.
    /// If the map did have this key present, the value is updated, and the old value is returned
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    /// use std::sync::Arc;
    ///
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// assert_eq!(hash_map.insert(1, 10), None);
    /// assert_eq!(hash_map.insert(1, 100), Some(Arc::new(10)));
    /// ```
    pub fn insert(&self, key: KeyType, value: ValueType) -> Option<Arc<ValueType>> {
        self.insert_rc(key, Arc::new(value))
    }

    /// Inserts a key-value pair into the map, where the value is an Arc type. This is a small
    /// optimization that saves the ReactiveHashMap from having to create a new Arc and potentially
    /// clone the underlying object.
    /// If the map did not have this key present, None is returned.
    /// If the map did have this key present, the value is updated, and the old value is returned
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    /// use std::sync::Arc;
    ///
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// assert_eq!(hash_map.insert_rc(1, Arc::new(10)), None);
    /// assert_eq!(hash_map.insert_rc(1, Arc::new(100)), Some(Arc::new(10)));
    /// ```
    pub fn insert_rc(&self, key: KeyType, value: Arc<ValueType>) -> Option<Arc<ValueType>> {
        // If there is an existing input subscription on this key, close it. This will essentially
        // overwrite the stream inserted by insert_stream and replace it with a constant value.
        self.remove_input_subscription(&key);

        let existing_value = self.get(&key);
        self.insert_rc_internal(key, value);
        existing_value
    }

    /// Ties a value in the Map to a ReactiveValue. Whenever the ReactiveValue changes
    /// the entry in the Map will be updated, until the map key is overwritten or removed.
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    /// use epoxy_streams::{ReactiveValue, WriteableReactiveValue};
    /// use std::sync::Arc;
    ///
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// let reactive_value: WriteableReactiveValue<i8> = ReactiveValue::new(1);
    /// hash_map.insert_reactive_value(1, &reactive_value);
    /// assert_eq!(hash_map.get(&1), Some(Arc::new(1)));
    ///
    /// reactive_value.set(12);
    /// assert_eq!(hash_map.get(&1), Some(Arc::new(12)));
    ///
    /// hash_map.insert(1, 0);
    /// assert_eq!(hash_map.get(&1), Some(Arc::new(0)));
    ///
    /// reactive_value.set(90);
    /// assert_eq!(hash_map.get(&1), Some(Arc::new(0)));
    /// ```
    pub fn insert_reactive_value<ReactiveValueType>(
        &self,
        key: KeyType,
        reactive_value: &ReactiveValueType,
    ) where
        ReactiveValueType: epoxy_streams::ReactiveValue<ValueType>,
    {
        self.insert_rc_internal(key.clone(), reactive_value.get());
        self.insert_stream(key, reactive_value.as_stream());
    }

    /// Ties a value in the Map to the latest emission from a stream. Whenever the stream emits
    /// the entry in the Map will be updated, until the map key is overwritten or removed.
    ///
    /// # Examples
    /// ```
    /// use epoxy_collections::ReactiveHashMap;
    /// use epoxy_streams::Sink;
    /// use std::sync::Arc;
    ///
    /// let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    /// let sink: Sink<i8> = Sink::new();
    /// hash_map.insert_stream(1, sink.get_stream());
    ///
    /// assert_eq!(hash_map.get(&1), None);
    ///
    /// sink.emit(9);
    /// assert_eq!(hash_map.get(&1), Some(Arc::new(9)));
    ///
    /// hash_map.remove(1);
    /// assert_eq!(hash_map.get(&1), None);
    ///
    /// sink.emit(90);
    /// assert_eq!(hash_map.get(&1), None);
    /// ```
    pub fn insert_stream(&self, key: KeyType, value_stream: epoxy_streams::Stream<ValueType>) {
        let mut write_input_subscriptions =
            self.internal.property_input_subscriptions.write().unwrap();
        let cloned_self = self.clone_internal();
        write_input_subscriptions.insert(
            key.clone(),
            value_stream.subscribe(move |val| {
                cloned_self.insert_rc_internal(key.clone(), val);
            }),
        );
    }

    fn insert_rc_internal(&self, key: KeyType, value: Arc<ValueType>) {
        let old_value = self.get_dynamic(&key);
        self.write_mutations(vec![Arc::new(Mutation::Property(PropertyMutation {
            key: Box::new(key),
            old_value: old_value,
            new_value: Some(value),
        }))]);
    }

    pub(super) fn clone_internal(&self) -> ReactiveHashMap<KeyType, ValueType> {
        ReactiveHashMap {
            internal: self.internal.clone(),
        }
    }
}
