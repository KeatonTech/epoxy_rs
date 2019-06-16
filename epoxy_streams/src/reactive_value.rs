use super::{Sink, Stream, Subscription};
use std::default::Default;
use std::sync::{Arc, RwLock};

/// Trait that applies to both readonly and writeable reactive values.
pub trait ReactiveValue<T> {
    /// Returns the current value of the ReactiveValue.
    fn get(&self) -> Arc<T>;

    /// Returns a Stream that represents the changing value over time.
    /// Use this function to subscribe to changes in the ReactiveValue.ReadonlyReactiveValue
    ///
    /// # Examples
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use epoxy_streams::ReactiveValue;
    ///
    /// let value = ReactiveValue::new(4);
    ///
    /// let last_value = Arc::new(Mutex::new(0_i32));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = value.as_stream().subscribe(move |val| {
    ///     *last_value_write.lock().unwrap() = *val;
    /// });
    ///
    /// value.set(1);
    /// assert_eq!(*last_value.lock().unwrap(), 1);
    ///
    /// value.set(100);
    /// assert_eq!(*last_value.lock().unwrap(), 100);
    /// ```
    fn as_stream(&self) -> Stream<T>;
}

// IMPLEMENTATIONS

struct ReadonlyReactiveValueImpl<T> {
    value: Box<RwLock<Arc<T>>>,

    #[allow(dead_code)]
    subscription: Subscription<T>,
}

impl<T> ReactiveValue<T> for ReadonlyReactiveValueImpl<T> {
    fn as_stream(&self) -> Stream<T> {
        self.subscription.stream.clone()
    }

    fn get(&self) -> Arc<T> {
        match self.value.read() {
            Ok(val) => Arc::clone(&val),
            Err(err) => panic!("ReactiveValue mutex poisoned: {}", err),
        }
    }
}

struct WriteableReactiveValueImpl<T> {
    value: Box<RwLock<Arc<T>>>,
    host: Sink<T>,
}

impl<T> ReactiveValue<T> for WriteableReactiveValueImpl<T> {
    fn as_stream(&self) -> Stream<T> {
        self.host.get_stream()
    }

    fn get(&self) -> Arc<T> {
        match self.value.read() {
            Ok(val) => Arc::clone(&val),
            Err(err) => panic!("ReactiveValue mutex poisoned: {}", err),
        }
    }
}

/// Holds the latest value emitted by a stream.
///
/// ReactiveValues automatically unsubscribe from the stream when they are destroyed, preventing
/// the kinds of memory leaks common in reactive programming.
///
/// # Examples
///
/// ```
/// use epoxy_streams::ReactiveValue;
///
/// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
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
/// use epoxy_streams::ReactiveValue;
///
/// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
/// let stream = stream_host.get_stream();
/// let reactive_value = stream.map(|val| val * 100).to_reactive_value_with_default(1000);
/// assert_eq!(*reactive_value.get(), 1000);
/// stream_host.emit(100);
/// assert_eq!(*reactive_value.get(), 10000);
/// ```
pub struct ReadonlyReactiveValue<T> {
    pointer: Arc<ReadonlyReactiveValueImpl<T>>,
}

impl<T> ReactiveValue<T> for ReadonlyReactiveValue<T> {
    fn as_stream(&self) -> Stream<T> {
        self.pointer.as_stream()
    }

    fn get(&self) -> Arc<T> {
        self.pointer.get()
    }
}

impl<T> Clone for ReadonlyReactiveValue<T> {
    fn clone(&self) -> Self {
        ReadonlyReactiveValue {
            pointer: Arc::clone(&self.pointer),
        }
    }
}

/// Reactive value that is explicitly set, rather than being derived from a stream.
///
/// # Examples
/// ```
/// use epoxy_streams::ReactiveValue;
///
/// let writeable_value = ReactiveValue::new(5);
/// assert_eq!(*writeable_value.get(), 5);
///
/// writeable_value.set(50);
/// assert_eq!(*writeable_value.get(), 50);
/// ```
pub struct WriteableReactiveValue<T> {
    pointer: Arc<WriteableReactiveValueImpl<T>>,
}

impl<T> ReactiveValue<T> for WriteableReactiveValue<T> {
    fn as_stream(&self) -> Stream<T> {
        self.pointer.as_stream()
    }

    fn get(&self) -> Arc<T> {
        self.pointer.get()
    }
}

impl<T> Clone for WriteableReactiveValue<T> {
    fn clone(&self) -> Self {
        WriteableReactiveValue {
            pointer: Arc::clone(&self.pointer),
        }
    }
}

impl<T: 'static> WriteableReactiveValue<T> {
    /// Sets the value of the ReactiveValue, using a mutex to ensure thread safety.
    ///
    /// Note: use `set_rc` if your new value is already an Arc, as this will prevent the
    /// value from being unnecessarily copied.
    pub fn set(&self, value: T) {
        self.set_rc(Arc::new(value))
    }

    /// Sets the value of the ReactiveValue, using a mutex to ensure thread safety.
    pub fn set_rc(&self, value: Arc<T>) {
        {
            let mut val_mut = self.pointer.value.write().unwrap();
            *val_mut = value.clone();
        }
        self.pointer.host.emit_rc(value)
    }

    /// Returns a ReadonlyReactiveValue whose value matches this one.
    /// This is helpful when exposing ReactiveValues to public APIs, so that
    /// the consumer cannot alter the state of your component.
    pub fn as_readonly(&self) -> ReadonlyReactiveValue<T> {
        self.as_stream()
            .to_reactive_value_with_default_rc(self.get())
    }
}

// CONSTRUCTORS

impl<T: 'static> ReactiveValue<T> {
    /// Creates a new writeable reactive value.
    ///
    /// Note: Use `new_rc` if your default value is already an Arc, as this will
    /// prevent another pointer from being created unnecessarily.
    ///
    /// # Examples
    /// ```
    /// use epoxy_streams::ReactiveValue;
    ///
    /// let writeable_value = ReactiveValue::new(5);
    /// assert_eq!(*writeable_value.get(), 5);
    ///
    /// writeable_value.set(50);
    /// assert_eq!(*writeable_value.get(), 50);
    /// ```
    pub fn new(initial_value: T) -> WriteableReactiveValue<T> {
        ReactiveValue::new_rc(Arc::new(initial_value))
    }

    /// See docs for `new`
    pub fn new_rc(initial_value: Arc<T>) -> WriteableReactiveValue<T> {
        WriteableReactiveValue {
            pointer: Arc::new(WriteableReactiveValueImpl {
                value: Box::new(RwLock::new(initial_value)),
                host: Sink::new(),
            }),
        }
    }

    pub fn from_stream(stream: Stream<T>) -> ReadonlyReactiveValue<T>
    where
        T: Default,
    {
        ReactiveValue::from_stream_with_default(stream, Default::default())
    }

    pub fn from_stream_with_default(stream: Stream<T>, default: T) -> ReadonlyReactiveValue<T> {
        ReactiveValue::from_stream_with_default_rc(stream, Arc::new(default))
    }

    pub fn from_stream_with_default_rc(
        stream: Stream<T>,
        default: Arc<T>,
    ) -> ReadonlyReactiveValue<T> {
        let original_value = Box::new(RwLock::new(default));
        let val_ptr = Box::into_raw(original_value);
        unsafe {
            let value = Box::from_raw(val_ptr);
            let subscription = stream.subscribe(move |val| {
                let mut val_mut = (*val_ptr).write().unwrap();
                *val_mut = val.clone();
            });
            ReadonlyReactiveValue {
                pointer: Arc::new(ReadonlyReactiveValueImpl {
                    value: value,
                    subscription: subscription,
                }),
            }
        }
    }
}

impl<T: 'static> Stream<T> {
    /// Creates a ReactiveValue from the stream, using the empty state value for type T as the
    /// default.
    ///
    /// Use `to_reactive_value_with_default` or `to_reactive_value_with_default_rc` if you want
    /// a more reasonable default value, or if your type T does not implement `Default`.
    ///
    /// # Examples
    /// ```
    /// use epoxy_streams::ReactiveValue;
    ///
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
    /// let stream = stream_host.get_stream();
    ///
    /// let reactive_value = stream.map(|val| val * 100).to_reactive_value();
    /// assert_eq!(*reactive_value.get(), 0);
    ///
    /// stream_host.emit(100);
    /// assert_eq!(*reactive_value.get(), 10000);
    /// ```
    pub fn to_reactive_value(self) -> ReadonlyReactiveValue<T>
    where
        T: Default,
    {
        ReactiveValue::from_stream(self)
    }

    /// See `to_reactive_value`.
    pub fn to_reactive_value_with_default(self, default: T) -> ReadonlyReactiveValue<T> {
        ReactiveValue::from_stream_with_default(self, default)
    }

    /// See `to_reactive_value`.
    pub fn to_reactive_value_with_default_rc(self, default: Arc<T>) -> ReadonlyReactiveValue<T> {
        ReactiveValue::from_stream_with_default_rc(self, default)
    }
}
