use crate::streams;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

/// Holds the latest value emitted by a stream.
/// 
/// ReactiveValues automatically unsubscribe from the stream when they are destroyed, preventing
/// the kinds of memory leaks common in reactive programming.
/// 
/// # Examples
/// 
/// ```
/// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
/// let stream = stream_host.get_stream();
/// let reactive_value = reactive::ReactiveValue::from_stream(stream, 0);
/// assert_eq!(*reactive_value.get_value(), 0);
/// stream_host.emit(1);
/// assert_eq!(*reactive_value.get_value(), 1);
/// stream_host.emit(3);
/// assert_eq!(*reactive_value.get_value(), 3);
/// ```
/// 
/// ```
/// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
/// let stream = stream_host.get_stream();
/// {
///     let reactive_value = reactive::ReactiveValue::from_stream(stream, 0);
///     assert_eq!(stream.count_subscribers(), 1);
/// }
/// assert_eq!(stream.count_subscribers(), 0);
/// ```
pub struct ReactiveValue<'a, T> {
    value: Box<RwLock<Rc<T>>>,
    stream: &'a streams::Stream<T>,

    #[allow(dead_code)]
    subscription: streams::SubscriptionRef<T>,
}

/// Reference to a ReactiveValue. Returned instead of the raw struct so that the computed!
/// function does not have to make copies of the actual ReactiveValue object.
pub type ReactiveValueRef<'a, T> = Arc<ReactiveValue<'a, T>>;

impl<'a, T> ReactiveValue<'a, T> {

    /// Creates a new ReactiveValue, tied to the value of the given stream.
    /// 
    /// The get_value() function of the output ReactiveValue will return `default` until
    /// the stream next emits.
    /// 
    /// If your default value is already an Rc pointer, use `from_stream_rc` as it will
    /// prevent the default value from getting unnecessarily copied.
    pub fn from_stream(stream: &'a streams::Stream<T>, default: T) -> ReactiveValueRef<'a, T> where
        T: 'static
    {
        ReactiveValue::from_stream_rc(stream, Rc::new(default))
    }

    /// Creates a new ReactiveValue, tied to the value of the given stream.
    /// 
    /// The get_value() function of the output ReactiveValue will return `default` until
    /// the stream next emits.
    pub fn from_stream_rc(stream: &'a streams::Stream<T>, default: Rc<T>) -> ReactiveValueRef<'a, T> where
        T: 'static
    {
        let original_value = Box::new(RwLock::new(default));
        let val_ptr = Box::into_raw(original_value);
        unsafe {
            let value = Box::from_raw(val_ptr);
            let subscription = stream.subscribe(move |val| {
                let mut val_mut = (*val_ptr).write().unwrap();
                *val_mut = val.clone();
            });
            Arc::new(ReactiveValue {
                value: value, 
                subscription: subscription,
                stream: stream
            })
        }
    }

    /// Subscribes to the stream backing this ReactiveValue.
    pub fn subscribe<F>(&self, listener: F) -> streams::SubscriptionRef<T> where
        F: Fn(&Rc<T>),
        F: 'static
    {
        self.stream.subscribe(listener)
    }

    /// Unsubscribes from the stream backing this ReactiveValue.
    pub fn unsubscribe(&self, subscription: streams::SubscriptionRef<T>) {
        self.stream.unsubscribe(subscription)
    }

    /// Returns the last value emitted from the stream, as an Rc pointer.
    pub fn get_value(&self) -> Rc<T> {
        match self.value.read() {
            Ok(val) => val.clone(),
            Err(err) => panic!("Reactive Value lock poisoned: {}", err)
        }
    }
}