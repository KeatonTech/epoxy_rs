use super::mutations::Mutation;
use std::sync::Arc;

pub trait ReactiveContainerItem {
    fn get_mutation_stream(&self) -> Option<epoxy_streams::Stream<Mutation>>;
    fn write_mutations(&self, mutations: Box<Iterator<Item = Arc<Mutation>>>);
}

macro_rules! no_op_container_item_for_immutable_type {
    ($type:ty) => {
        impl ReactiveContainerItem for $type {
            fn get_mutation_stream(&self) -> Option<epoxy_streams::Stream<Mutation>> {
                None
            }

            fn write_mutations(&self, _mutations: Box<Iterator<Item = Arc<Mutation>>>) {
                panic!("Attempted to mutate a primitive child of a reactive data structure")
            }
        }
    };
}

no_op_container_item_for_immutable_type!(bool);
no_op_container_item_for_immutable_type!(char);
no_op_container_item_for_immutable_type!(i8);
no_op_container_item_for_immutable_type!(i16);
no_op_container_item_for_immutable_type!(i32);
no_op_container_item_for_immutable_type!(i64);
no_op_container_item_for_immutable_type!(i128);
no_op_container_item_for_immutable_type!(isize);
no_op_container_item_for_immutable_type!(u8);
no_op_container_item_for_immutable_type!(u16);
no_op_container_item_for_immutable_type!(u32);
no_op_container_item_for_immutable_type!(u64);
no_op_container_item_for_immutable_type!(u128);
no_op_container_item_for_immutable_type!(usize);
no_op_container_item_for_immutable_type!(f32);
no_op_container_item_for_immutable_type!(f64);
no_op_container_item_for_immutable_type!(&'static str);
