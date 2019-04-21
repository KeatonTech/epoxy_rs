use super::{Stream, Subscription};
use std::rc::Rc;

pub struct CombinedStreamFields<T> {
    #[allow(dead_code)]
    subscriptions: Vec<Subscription<T>>,
}

/// Combines all values emitted from a list of same-typed streams into one single stream.
///
/// # Examples
/// ```
/// use epoxy_streams::ReactiveValue;
///
/// let stream_host_1: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
/// let stream_1 = stream_host_1.get_stream();
///
/// let stream_host_2: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
/// let stream_2 = stream_host_2.get_stream();
///
/// let merged = epoxy_streams::merge(vec![stream_1, stream_2]);
/// let merged_value = merged.clone().to_reactive_value();
/// let emit_count = merged.count_values().to_reactive_value();
///
/// stream_host_1.emit(10);
/// assert_eq!(*merged_value.get(), 10);
///
/// stream_host_2.emit(15);
/// assert_eq!(*merged_value.get(), 15);
///
/// stream_host_2.emit(16);
/// stream_host_1.emit(17);
/// assert_eq!(*merged_value.get(), 17);
/// assert_eq!(*emit_count.get(), 4);
/// ```
pub fn merge<T: 'static>(streams: Vec<Stream<T>>) -> Stream<T> {
    let merged_stream = Stream::new_with_fields::<CombinedStreamFields<T>>(CombinedStreamFields {
        subscriptions: vec![],
    });

    let subscriptions: Vec<Subscription<T>> = streams
        .into_iter()
        .map(|stream| {
            let weak_stream_ref = Rc::downgrade(&merged_stream.pointer);
            stream.subscribe(move |value| match weak_stream_ref.upgrade() {
                Some(stream_ref) => match stream_ref.lock() {
                    Ok(stream_impl) => stream_impl.emit_rc(value),
                    Err(err) => panic!("Stream mutex poisoned: {}", err),
                },
                None => (),
            })
        })
        .collect();

    merged_stream.mutate_extra_fields(move |extra_fields: &mut CombinedStreamFields<T>| {
        extra_fields.subscriptions = subscriptions;
    });

    return merged_stream;
}
