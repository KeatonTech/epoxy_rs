extern crate epoxy_collections;
extern crate epoxy_streams;

use epoxy_collections::{ReactiveCollection, ReactiveHashMap, ReadonlyReactiveCollection};
use epoxy_streams::ReactiveCache;

fn main() {
    let hash_map: ReactiveHashMap<i8, i8> = ReactiveHashMap::new();
    let mutations = ReactiveCache::from_stream(hash_map.get_mutation_stream());

    hash_map.insert(1, 2);
    hash_map.insert(2, 4);
    assert_eq!(mutations.get().len(), 2);
    assert_eq!(*hash_map.get(&1).unwrap(), 2);

    let nested_hash_map: ReactiveHashMap<i8, ReactiveHashMap<i8, i8>> = ReactiveHashMap::new();
    nested_hash_map.insert(1, hash_map);
    let nested_mutations = ReactiveCache::from_stream(nested_hash_map.get_mutation_stream());
    assert_eq!(nested_mutations.get().len(), 0);

    nested_hash_map.get(&1).unwrap().insert(3, 6);
    assert_eq!(nested_mutations.get().len(), 1);
}
