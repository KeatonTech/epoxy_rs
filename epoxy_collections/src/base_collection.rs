use super::mutations::Mutation;
use super::reactive_container_item::ReactiveContainerItem;
use std::any::Any;
use std::sync::{Arc, RwLock};

pub struct BaseReactiveCollection<CollectionType> {
    pub(crate) collection: Box<RwLock<CollectionType>>,
    pub(crate) mutation_sink: epoxy_streams::Sink<Mutation>,
    pub(crate) extra_fields: RwLock<Option<Box<dyn Any + Send + Sync + 'static>>>,
}

pub trait ReadonlyReactiveCollectionInternal {
    type CollectionType;

    fn get_collection(&self) -> &Box<RwLock<Self::CollectionType>>;
    fn get_mutation_sink(&self) -> &epoxy_streams::Sink<Mutation>;
}

pub trait ReactiveCollectionInternal: ReadonlyReactiveCollectionInternal {
    fn apply_mutation(&self, collection: &mut Self::CollectionType, mutation: &Mutation);
}

pub trait ReadonlyReactiveCollection {
    type CollectionType;

    fn get_snapshot(&self) -> Self::CollectionType;
    fn get_mutation_stream(&self) -> epoxy_streams::Stream<Mutation>;
}

pub trait ReactiveCollection: ReadonlyReactiveCollection {
    fn write_mutations(&self, mutations: Vec<Arc<Mutation>>);
}

impl<CollectionType> BaseReactiveCollection<CollectionType> {
    pub fn new(initial_collection: CollectionType) -> BaseReactiveCollection<CollectionType> {
        BaseReactiveCollection {
            collection: Box::new(RwLock::new(initial_collection)),
            mutation_sink: epoxy_streams::Sink::new(),
            extra_fields: RwLock::new(None),
        }
    }
}

impl<CollectionType, T: ReadonlyReactiveCollectionInternal<CollectionType = CollectionType>>
    ReadonlyReactiveCollection for T
where
    CollectionType: Clone,
{
    type CollectionType = CollectionType;

    fn get_snapshot(&self) -> Self::CollectionType {
        (*self.get_collection().read().unwrap()).clone()
    }

    fn get_mutation_stream(&self) -> epoxy_streams::Stream<Mutation> {
        self.get_mutation_sink().get_stream()
    }
}

impl<CollectionType, T: ReactiveCollectionInternal<CollectionType = CollectionType>>
    ReactiveCollection for T
where
    CollectionType: Clone,
{
    fn write_mutations(&self, mutations: Vec<Arc<Mutation>>) {
        let collection_lock = self.get_collection();
        {
            let mut collection_value = collection_lock.write().unwrap();
            for mutation in &mutations {
                self.apply_mutation(&mut *collection_value, &mutation);
            }
        }
        for mutation in mutations {
            self.get_mutation_sink().emit_rc(mutation);
        }
    }
}

impl<C, T> ReactiveContainerItem for T
where
    T: ReactiveCollection<CollectionType = C>,
{
    fn get_item_mutation_stream(&self) -> Option<epoxy_streams::Stream<Mutation>> {
        Some(T::get_mutation_stream(self))
    }

    fn write_mutations(&self, mutations: Vec<Arc<Mutation>>) {
        T::write_mutations(self, mutations)
    }
}
