use std::any::Any;
use std::ops::Deref;
use std::rc::{Rc, Weak};
use std::sync::Mutex;

/// Represents a function callback that will run whenever a stream emits a new value.
/// 
/// This struct is not merely a token that can be used for unsubscribing, as it is in
/// many other reactive frameworks. Here the Subscription becomes the owner of the
/// callback function. The result is that when the Subscription is destroyed the callback
/// function will stop getting called (and, in fact, cease to exist).
/// 
/// # Examples
/// 
/// ```
/// use std::sync::{Arc, Mutex};
/// 
/// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
/// let stream = stream_host.get_stream();
/// 
/// let emit_count = Arc::new(Mutex::new(0_u32));
/// let emit_count_write = emit_count.clone();
/// {
///     let _sub = stream.subscribe(move |val| {
///         *emit_count_write.lock().unwrap() += 1;
///     });
///     stream_host.emit(1);
///     assert_eq!(*emit_count.lock().unwrap(), 1);
///     stream_host.emit(2);
///     assert_eq!(*emit_count.lock().unwrap(), 2);
/// 
///     // _sub goes out of scope here. The stream callback function will no longer
///     // be run so emit_count will stop increasing.
/// }
/// stream_host.emit(3);
/// assert_eq!(*emit_count.lock().unwrap(), 2);
/// ```
pub struct Subscription<T> {
    call: Box<Fn(&Rc<T>)>,
    id: usize
}

/// Ref-counted pointer to a [Subscription](struct.Subscription.html).
/// 
/// Subscriptions returned from the [subscribe](struct.Stream.html#method.subscribe) function are
/// wrapped in an Rc smartpointer so that the stream can hold a [Weak](std::rc::Weak) reference to
/// them. This allows the stream to emit values to each subscription until the returned
/// SubscriptionRef goes out of scope and causes the underlying subscription to be destroyed.
pub type SubscriptionRef<T> = Rc<Subscription<T>>;


// BASIC STREAMS

/// A stream is data source that can emit new values at any point during its lifetime.
/// 
/// Streams are different from inter-thread channels like std::sync::mpsc::Sender in that
/// they do not need to be polled, and are not intended only for short-lived inter-process
/// communication. It would not be uncommon, or even discouraged, for a stream to exist for the
/// entire lifetime of your program. Streams use a functional interface instead of polling, so they
/// only consume CPU resources when new data is emitted (although they do always consume memory for
/// storing the subscribed callback functions).
/// 
/// Streams are read-only. Streams can either be created with a [StreamHost](struct.StreamHost.html),
/// which is a write-only data source, or by altering an existing stream using iterator-like
/// functional syntax (.map, .filter, etc...).
pub struct Stream<T> {
    listeners: Mutex<Vec<Weak<Subscription<T>>>>,
    derived_streams: Mutex<Vec<Rc<Any>>>,
}

/// If more than this fraction of a stream's subscriptions are dead (weak references that have been
/// dropped), automatically run stream.cleanup() on the next StreamHost emission.
pub const DEAD_SUBSCRIPTION_CLEANUP_PROPORTION: f32 = 0.8;

impl<T> Stream<T> {
    fn new() -> Stream<T> {
        Stream {
            listeners: Mutex::new(vec![]),
            derived_streams: Mutex::new(vec![]),
        }
    }

    /// Registers a callback function to run whenever a new value is emitted to the stream.
    /// 
    /// The `listener` callback function will only be executed while its SubscriptionRef is in
    /// scope. When the returned SubscriptionRef is destroyed the callback function will be
    /// removed from the stream. This is an intentional choice to prevent memory leaks. Note
    /// that, even though this means the callback function has the same _effective_ lifetime
    /// as the returned SubscriptionRef, it must actually have a 'static lifetime due to
    /// limitations in Rust's lifetimes system. This means that most closures (functions that
    /// reference variables defined outside of themselves) will need the 'move' property.
    pub fn subscribe<F>(&self, listener: F) -> SubscriptionRef<T> where
        F: Fn(&Rc<T>),
        F: 'static
    {
        let mut listeners_mut = match self.listeners.lock() {
            Ok(mut_ref) => mut_ref,
            Err(err) => panic!("Stream listener poisoned: {}", err)
        };
        let subscription_ptr = Rc::new(Subscription {
            call: Box::new(listener),
            id: listeners_mut.len()
        });

        listeners_mut.push(Rc::downgrade(&subscription_ptr));
        subscription_ptr
    }

    /// Explicitly removes a subscription from the stream.
    /// 
    /// This will usually not be necessary because in most cases it is easier to simply let
    /// the SubscriptionRef be destroyed, which has the same effect.
    pub fn unsubscribe(&self, subscription: SubscriptionRef<T>) {
        let mut listeners_mut = match self.listeners.lock() {
            Ok(mut_ref) => mut_ref,
            Err(err) => panic!("Stream listener poisoned: {}", err)
        };
        listeners_mut[subscription.id] = Weak::new();
    }


    /// Counts the number of active subscribers on this stream.
    /// 
    /// Note that this method needs to aquire a mutex on the stream and attempt to resolve all of
    /// its weak subscription references, so it can be a relatively expensive operation.
    /// 
    /// # Examples
    /// 
    /// ```
    /// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
    /// let stream = stream_host.get_stream();
    /// {
    ///     let _sub = stream.subscribe(|val| {val;});
    ///     assert_eq!(stream.count_subscribers(), 1);
    /// }
    /// assert_eq!(stream.count_subscribers(), 0);
    /// ```
    pub fn count_subscribers(&self) -> usize {
        let listeners = match self.listeners.lock() {
            Ok(mut_ref) => mut_ref,
            Err(err) => panic!("Stream listener poisoned: {}", err)
        };
        let mut count = 0_usize;
        for ref listener_weak in listeners.iter() {
            let maybe_listener = listener_weak.upgrade();
            if !maybe_listener.is_some() {continue;}
            count += 1;
        }
        count
    }


    /// Cleans up the listeners of this stream to remove any expired weak references.
    /// 
    /// Note that although this function is public, there is likely no situation where it needs to
    /// be called manually except in cases that require an extreme level of optimization.
    /// StreamHost will automatically call this function when more than
    /// DEAD_SUBSCRIPTION_CLEANUP_PROPORTION of the subscriptions are dead.
    pub fn cleanup(&self) {
        let mut listeners_mut = match self.listeners.lock() {
            Ok(mut_ref) => mut_ref,
            Err(err) => panic!("Stream listener poisoned: {}", err)
        };
        listeners_mut.retain(|listener_weak| listener_weak.upgrade().is_some());
    }

    fn add_derived_stream(&self, derived: Rc<Any>) {
        let mut derived_streams_mut = match self.derived_streams.lock() {
            Ok(ds) => ds,
            Err(err) => panic!("Derived stream memory poisoned: {}", err)
        };
        derived_streams_mut.push(derived);
    }
}

// STREAM HOST

/// A StreamHost is a write-only data structure that emits data into a [Stream](struct.Stream.html)
/// 
/// # Examples
/// 
/// ```
/// use std::sync::{Arc, Mutex};
/// 
/// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
/// let stream = stream_host.get_stream();
/// 
/// let last_value = Arc::new(Mutex::new(0_i32));
/// let last_value_write = last_value.clone();
///
/// let subscription = stream.subscribe(move |val| {
///     *last_value_write.lock().unwrap() = **val;
/// });
/// 
/// stream_host.emit(1);
/// assert_eq!(*last_value.lock().unwrap(), 1);
/// 
/// stream_host.emit(100);
/// assert_eq!(*last_value.lock().unwrap(), 100);
/// ```
pub struct StreamHost<T> {
    stream: Stream<T>
}

impl<T> StreamHost<T> {
    pub fn new() -> StreamHost<T> {
        StreamHost {
            stream: Stream::new()
        }
    }

    /// Moves a new value into the stream and emits it to every subscriber.
    /// 
    /// This function will wrap the value in an Rc pointer so that it can be distributed to every
    /// subscriber function. If your value is already an Rc you can use emit_rc instead.
    pub fn emit(&self, value: T) {
        self.emit_rc(Rc::from(value))
    }

    /// Moves a reference-counted value into the stream and emits it to every subscriber.
    pub fn emit_rc(&self, value: Rc<T>) {
        let dead_subscription_proportion: f32;
        {
            let listeners = match self.stream.listeners.lock() {
                Ok(mut_ref) => mut_ref,
                Err(err) => panic!("Stream listener poisoned: {}", err)
            };

            let mut emitted_count = 0;
            for ref listener_weak in listeners.iter() {
                let maybe_listener = listener_weak.upgrade();
                if !maybe_listener.is_some() {continue;}
                let listener_subscription = maybe_listener.unwrap();
                (listener_subscription.call)(&value);
                emitted_count += 1;
            }

            dead_subscription_proportion = 1_f32 - (emitted_count as f32 / listeners.len() as f32);
        }
        if dead_subscription_proportion >= DEAD_SUBSCRIPTION_CLEANUP_PROPORTION {
            self.stream.cleanup()
        }
    }

    /// Returns the read-only stream that corresponds to this StreamHost.
    pub fn get_stream(&self) -> &Stream<T> {
        &self.stream
    }
}

// PURE STREAM OPERATORS

pub struct DerivedStream<T, U> {
    stream_host: Box<StreamHost<T>>,

    #[allow(dead_code)]
    subscription: SubscriptionRef<U>
}

pub type DerivedStreamRef<T, U> = Rc<DerivedStream<T, U>>;

impl<T, U> Deref for DerivedStream<T, U> {
    type Target = Stream<T>;

    fn deref(&self) -> &Stream<T> {
        &self.stream_host.stream
    }
}

impl<T> Stream<T> {
    fn create_derived_stream<U, F>(&self, subscription_re_emit: F) -> DerivedStreamRef<U, T> where
        F: Fn(&StreamHost<U>, &Rc<T>),
        F: 'static,
        U: 'static,
        T: 'static,
    {
        let original_host = Box::new(StreamHost::new());
        let push_host = Box::into_raw(original_host);

        // Using an unsafe block here because `stream_host` must have the same lifetime as
        // `subscription`, so it is okay for the subscription function to reference it directly.
        unsafe {
            let saved_host = Box::from_raw(push_host);
            let subscription = self.subscribe(move |val| {
                subscription_re_emit(&*push_host, &val);
            });
            let derived_stream_ref = Rc::new(DerivedStream {
                stream_host: saved_host,
                subscription: subscription
            });
            self.add_derived_stream(derived_stream_ref.clone());
            derived_stream_ref
        }
    }

    /// Returns a stream that emits only those values from the original stream that pass a test.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// 
    /// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
    /// let stream = stream_host.get_stream();
    /// 
    /// let last_value = Arc::new(Mutex::new(0_i32));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream
    ///     .filter(|val| val % 2 == 0 /* Allow only even numbers through */)
    ///     .subscribe(move |val| {
    ///          *last_value_write.lock().unwrap() = **val;
    ///     });
    /// 
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), 2);
    /// 
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), 2); // Note that '3' was not emitted
    /// 
    /// stream_host.emit(4);
    /// assert_eq!(*last_value.lock().unwrap(), 4);
    /// ```
    pub fn filter<F>(&self, filter_function: F) -> DerivedStreamRef<T, T> where
        F: Fn(&T) -> bool,
        F: 'static,
        T: 'static
    {
        self.create_derived_stream(move |host, val| {
            if filter_function(val) {
                host.emit_rc(val.clone());
            }
        })
    }

    /// Returns a stream containing modified values from the original stream.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// 
    /// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
    /// let stream = stream_host.get_stream();
    /// 
    /// let last_value = Arc::new(Mutex::new(0_i32));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream
    ///     .map(|val| val * 100)
    ///     .subscribe(move |val| {
    ///          *last_value_write.lock().unwrap() = **val;
    ///     });
    /// 
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), 200);
    /// 
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), 300);
    /// ```
    pub fn map<U, F>(&self, map_function: F) -> DerivedStreamRef<U, T> where
        U: 'static,
        F: Fn(&T) -> U,
        F: 'static,
        T: 'static
    {
        self.create_derived_stream(move |host, val| {
            host.emit(map_function(val));
        })
    }

    /// Returns a stream that can emit multiple values for each value from the original stream.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// 
    /// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
    /// let stream = stream_host.get_stream();
    /// 
    /// let last_value = Arc::new(Mutex::new(0_i32));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream
    ///     .flat_map(|val| vec![*val, val + 1, val + 2])
    ///     .subscribe(move |val| {
    ///          *last_value_write.lock().unwrap() = **val;
    ///     });
    /// 
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), 4); /* Emitted 2, 3, then 4 */
    /// 
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), 5); /* Emitted 3, 4, then 5 */
    /// ```
    pub fn flat_map<U, F, C>(&self, iter_map_function: F) -> DerivedStreamRef<U, T> where
        U: 'static,
        C: IntoIterator<Item = U>,
        F: Fn(&T) -> C,
        F: 'static,
        T: 'static
    {
        self.create_derived_stream(move |host, val| {
            let mut iter = iter_map_function(val).into_iter();
            loop {
                match iter.next() {
                    Some(x) => {
                        host.emit(x)
                    },
                    None => { break }
                }
            }
        })
    }

    /// Similar to subscribing to a stream in that `inspect_function` runs whenever the
    /// stream emits, but returns a derived stream matching the original stream instead of
    /// a SubscriptionRef.
    pub fn inspect<F>(&self, inspect_function: F) -> DerivedStreamRef<T, T> where
        F: Fn(&T),
        F: 'static,
        T: 'static
    {
        self.create_derived_stream(move |host, val| {
            inspect_function(val);
            host.emit_rc(val.clone());
        })
    }
}

// STATEFUL STREAM OPERATORS

pub struct StatefulDerivedStream<T, U, V> {
    stream_host: Box<StreamHost<T>>,
    state: V,

    #[allow(dead_code)]
    subscription: Option<SubscriptionRef<U>>
}

pub type StatefulDerivedStreamRef<T, U, V> = Rc<StatefulDerivedStream<T, U, V>>;

impl<T, U, V> Deref for StatefulDerivedStream<T, U, V> {
    type Target = Stream<T>;

    fn deref(&self) -> &Stream<T> {
        &self.stream_host.stream
    }
}

impl<T> Stream<T> {
    /// Similar to the `reduce` method on iterators, but works iteratively and creates a stream
    /// that emits with the latest 'reduced' value whenever the original stream emits.
    /// 
    /// Note that because streams never officially 'close', there is no actual reduce method.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// 
    /// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
    /// let stream = stream_host.get_stream();
    /// 
    /// let last_value = Arc::new(Mutex::new(vec![]));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream
    ///     .scan(|acc, val| {
    ///         let mut extended = vec![];
    ///         extended.extend(acc);
    ///         extended.push(**val);
    ///         extended
    ///     }, vec![])
    ///     .subscribe(move |val| {
    ///          *last_value_write.lock().unwrap() = (**val).clone();
    ///     });
    /// 
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), vec![2]);
    /// 
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), vec![2, 3]);
    /// ```
    pub fn scan<U, F>(&self, scan_fn: F, initial_state: U) -> StatefulDerivedStreamRef<U, T, Rc<U>> where
        F: Fn(&U, &Rc<T>) -> U,
        F: 'static,
        U: 'static,
        T: 'static
    {
        let original_host = Box::new(StreamHost::new());
        let push_host = Box::into_raw(original_host);

        // Using an unsafe block here because `stream_host` must have the same lifetime as
        // `subscription`, so it is okay for the subscription function to reference it directly.
        unsafe {
            let saved_host = Box::from_raw(push_host);
            let mut sds = Rc::new(StatefulDerivedStream {
                stream_host: saved_host,
                state: Rc::from(initial_state),
                subscription: None
            });
            let sds_ptr = match Rc::get_mut(&mut sds) {
                Some(ptr) => ptr,
                None => panic!("Could not mutate stateful derived stream")
            } as *mut StatefulDerivedStream<U, T, Rc<U>>;
            (*sds_ptr).subscription = Some(self.subscribe(move |val| {
                let accumulated = Rc::from(scan_fn(&(*sds_ptr).state, val));
                (*sds_ptr).state = accumulated.clone();
                (*sds_ptr).stream_host.emit_rc(accumulated);
            }));
            self.add_derived_stream(sds.clone());
            return sds;
        }
    }

    /// Creates a stream that counts the number of times the original stream emits.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// 
    /// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
    /// let stream = stream_host.get_stream();
    /// 
    /// let last_value = Arc::new(Mutex::new(0_u32));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream.count_values().subscribe(move |val| {
    ///     *last_value_write.lock().unwrap() = (**val).clone();
    /// });
    /// 
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), 1);
    /// 
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), 2);
    /// ```
    pub fn count_values(&self) -> StatefulDerivedStreamRef<u32, T, Rc<u32>> where T: 'static {
        self.scan(|acc, _| acc + 1, 0)
    }

    /// Creates a stream that returns a vector of the last `n` emissions of the original stream.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// 
    /// let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
    /// let stream = stream_host.get_stream();
    /// 
    /// let last_value = Arc::new(Mutex::new(vec![]));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream.buffer(2).subscribe(move |val| {
    ///     *last_value_write.lock().unwrap() = (**val).clone();
    /// });
    /// 
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), vec![2]);
    /// 
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), vec![2, 3]);
    /// 
    /// stream_host.emit(4);
    /// assert_eq!(*last_value.lock().unwrap(), vec![3, 4]);
    /// ```
    pub fn buffer(&self, max_buffer_size: usize) -> StatefulDerivedStreamRef<Vec<T>, T, Rc<Vec<T>>> where
        T: Clone,
        T: 'static
    {
        self.scan(move |acc, val| {
            let mut extended = vec![];
            if acc.len() > 0 && acc.len() < max_buffer_size {
                extended.push(acc[0].clone());
            }
            for i in 1..acc.len() {
                extended.push(acc[i].clone());
            }
            extended.push((**val).clone());
            extended
        }, vec![])
    }
}