use super::base_collection::{
    BaseReactiveCollection, ReactiveCollectionInternal, ReadonlyReactiveCollectionInternal,
};
use super::mutations::Mutation::{Property, Subproperty};
use super::mutations::{Mutation, PropertyMutation, SubpropertyMutation};
use super::reactive_container_item::ReactiveContainerItem;
use std::any::Any;
use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::iter::once;
use std::sync::{Arc, RwLock};

// READONLY REACTIVE HASH MAP

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
    base: BaseReactiveCollection<HashMap<KeyType, Arc<ValueType>>>,
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
        &self.base.collection
    }

    fn get_mutation_sink(&self) -> &epoxy_streams::Sink<Mutation> {
        &self.base.mutation_sink
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
    pub fn get(&self, key: &KeyType) -> Option<Arc<ValueType>> {
        let read_guard = self.base.collection.read().unwrap();
        if let Some(value) = read_guard.get(key) {
            Some(value.clone())
        } else {
            None
        }
    }
}

// WRITABLE REACTIVE HASH MAP

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
    map: Arc<ReadonlyReactiveHashMap<KeyType, ValueType>>,
    subproperty_subscriptions: RwLock<HashMap<KeyType, epoxy_streams::Subscription<Mutation>>>,
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
        self.map.get_collection()
    }

    fn get_mutation_sink(&self) -> &epoxy_streams::Sink<Mutation> {
        self.map.get_mutation_sink()
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

                    if let Some(stream) = value.get_mutation_stream() {
                        let stream_key = key.clone();
                        let subscription = stream
                            .map_rc(move |mutation| {
                                Arc::new(Mutation::Subproperty(SubpropertyMutation {
                                    key: Box::new(stream_key.clone()),
                                    mutation: mutation.clone(),
                                }))
                            })
                            .pipe_into(self.get_mutation_sink());

                        self.subproperty_subscriptions
                            .write()
                            .unwrap()
                            .insert(key.clone(), subscription);
                    }

                    collection.insert(key, value);
                } else {
                    self.subproperty_subscriptions.write().unwrap().remove(&key);
                    collection.remove(&key);
                }
            }
            Subproperty(mutation) => {
                let key = mutation.key.downcast_ref::<KeyType>().unwrap().clone();
                if let Some(value) = collection.get(&key) {
                    value.write_mutations(Box::new(once(mutation.mutation.clone())));
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
    pub fn new() -> ReactiveHashMap<KeyType, ValueType> {
        ReactiveHashMap {
            map: Arc::new(ReadonlyReactiveHashMap {
                base: BaseReactiveCollection::new(HashMap::new()),
            }),
            subproperty_subscriptions: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &KeyType) -> Option<Arc<ValueType>> {
        self.map.get(key)
    }

    fn get_dynamic(&self, key: &KeyType) -> Option<Arc<dyn Any + Send + Sync>> {
        match self.get(key) {
            Some(arc) => Some(arc),
            None => None,
        }
    }

    pub fn insert(&self, key: KeyType, value: ValueType) {
        self.insert_rc(key, Arc::new(value))
    }

    pub fn insert_rc(&self, key: KeyType, value: Arc<ValueType>) {
        let old_value = self.get_dynamic(&key);
        self.write_mutations(Box::new(once(Arc::new(Mutation::Property(
            PropertyMutation {
                key: Box::new(key),
                old_value: old_value,
                new_value: Some(value),
            },
        )))));
    }
}
