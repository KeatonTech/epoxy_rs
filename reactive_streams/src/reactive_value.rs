use super::{Stream, Subscription};
use std::default::Default;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

struct ReactiveValueImpl<T> {
    value: Box<RwLock<Rc<T>>>,

    #[allow(dead_code)]
    subscription: Subscription<T>,
}

/// Holds the latest value emitted by a stream.
///
/// ReactiveValues automatically unsubscribe from the stream when they are destroyed, preventing
/// the kinds of memory leaks common in reactive programming.
///
/// # Examples
///
/// ```
/// let stream_host: reactive_streams::StreamHost<i32> = reactive_streams::StreamHost::new();
/// let stream = stream_host.get_stream();
/// let reactive_value = stream.map(|val| val * 100).to_reactive_value();
/// assert_eq!(*reactive_value.get(), 0);
/// stream_host.emit(1);
/// assert_eq!(*reactive_value.get(), 100);
/// stream_host.emit(3);
/// assert_eq!(*reactive_value.get(), 300);
/// ```
///
/// ```
/// let stream_host: reactive_streams::StreamHost<i32> = reactive_streams::StreamHost::new();
/// let stream = stream_host.get_stream();
/// let reactive_value = stream.map(|val| val * 100).to_reactive_value_with_default(1000);
/// assert_eq!(*reactive_value.get(), 1000);
/// stream_host.emit(100);
/// assert_eq!(*reactive_value.get(), 10000);
/// ```
pub struct ReactiveValue<T> {
    pointer: Arc<ReactiveValueImpl<T>>,
}

impl<T> Clone for ReactiveValue<T> {
    fn clone(&self) -> Self {
        ReactiveValue {
            pointer: Arc::clone(&self.pointer),
        }
    }
}

impl<T: 'static> ReactiveValue<T> {
    pub fn from_stream(stream: Stream<T>) -> ReactiveValue<T>
    where
        T: Default,
    {
        ReactiveValue::from_stream_with_default(stream, Default::default())
    }

    pub fn from_stream_with_default(stream: Stream<T>, default: T) -> ReactiveValue<T> {
        ReactiveValue::from_stream_with_default_rc(stream, Rc::new(default))
    }

    pub fn from_stream_with_default_rc(stream: Stream<T>, default: Rc<T>) -> ReactiveValue<T> {
        let original_value = Box::new(RwLock::new(default));
        let val_ptr = Box::into_raw(original_value);
        unsafe {
            let value = Box::from_raw(val_ptr);
            let subscription = stream.subscribe(move |val| {
                let mut val_mut = (*val_ptr).write().unwrap();
                *val_mut = val.clone();
            });
            ReactiveValue {
                pointer: Arc::new(ReactiveValueImpl {
                    value: value,
                    subscription: subscription,
                }),
            }
        }
    }

    pub fn get(&self) -> Rc<T> {
        match (*self.pointer).value.read() {
            Ok(val) => Rc::clone(&val),
            Err(err) => panic!("ReactiveValue mutex poisoned: {}", err),
        }
    }
}

impl<T: 'static> Stream<T> {
    pub fn to_reactive_value(self) -> ReactiveValue<T>
    where
        T: Default,
    {
        ReactiveValue::from_stream(self)
    }

    pub fn to_reactive_value_with_default(self, default: T) -> ReactiveValue<T> {
        ReactiveValue::from_stream_with_default(self, default)
    }

    pub fn to_reactive_value_with_default_rc(self, default: Rc<T>) -> ReactiveValue<T> {
        ReactiveValue::from_stream_with_default_rc(self, default)
    }
}
