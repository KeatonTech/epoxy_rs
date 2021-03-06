use super::{Stream, Subscription};
use std::sync::Arc;

pub struct DerivedStreamFields<T> {
    #[allow(dead_code)]
    subscription: Option<Subscription<T>>,
}

impl<T: 'static + Send + Sync> Stream<T> {
    fn create_derived_stream<U, F>(&self, subscription_re_emit: F) -> Stream<U>
    where
        F: Fn(&Stream<U>, Arc<T>),
        F: Send,
        F: Sync,
        F: 'static,
        U: 'static,
    {
        let derived_stream =
            Stream::new_with_fields::<DerivedStreamFields<T>>(DerivedStreamFields {
                subscription: None,
            });
        let subscription_stream_ref = derived_stream.clone();

        let subscription =
            self.subscribe(move |val| subscription_re_emit(&subscription_stream_ref, val));

        derived_stream.mutate_extra_fields(move |fields: &mut DerivedStreamFields<T>| {
            fields.subscription = Some(subscription);
        });

        derived_stream
    }

    /// Returns a stream that emits only those values from the original stream that pass a test.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    ///
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
    /// let stream = stream_host.get_stream();
    ///
    /// let last_value = Arc::new(Mutex::new(0_i32));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream
    ///     .filter(|val| val % 2 == 0 /* Allow only even numbers through */)
    ///     .subscribe(move |val| {
    ///          *last_value_write.lock().unwrap() = *val;
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
    pub fn filter<F>(&self, filter_function: F) -> Stream<T>
    where
        F: Fn(&T) -> bool,
        F: Send,
        F: Sync,
        F: 'static,
    {
        self.create_derived_stream(move |host, val| {
            if filter_function(&*val) {
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
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
    /// let stream = stream_host.get_stream();
    ///
    /// let last_value = Arc::new(Mutex::new(0_i32));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream
    ///     .map(|val| val * 100)
    ///     .subscribe(move |val| {
    ///          *last_value_write.lock().unwrap() = *val;
    ///     });
    ///
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), 200);
    ///
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), 300);
    /// ```
    pub fn map<U, F>(&self, map_function: F) -> Stream<U>
    where
        U: 'static,
        F: Fn(&T) -> U,
        F: Send,
        F: Sync,
        F: 'static,
    {
        self.create_derived_stream(move |host, val| {
            host.emit_rc(Arc::new(map_function(&*val)));
        })
    }

    /// Returns a stream containing modified values from the original stream.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use std::rc::Rc;
    ///
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
    /// let stream = stream_host.get_stream();
    ///
    /// let last_value = Arc::new(Mutex::new(0_i32));
    /// let last_value_write = last_value.clone();
    ///
    /// let fallback_value = Arc::new(10);
    ///
    /// let subscription = stream
    ///     .map_rc(move |val| if (*val < 0) { fallback_value.clone() } else { val })
    ///     .subscribe(move |val| {
    ///          *last_value_write.lock().unwrap() = *val;
    ///     });
    ///
    /// stream_host.emit(-2);
    /// assert_eq!(*last_value.lock().unwrap(), 10);
    ///
    /// stream_host.emit(12);
    /// assert_eq!(*last_value.lock().unwrap(), 12);
    ///
    /// stream_host.emit(-10);
    /// assert_eq!(*last_value.lock().unwrap(), 10);
    /// ```
    pub fn map_rc<U, F>(&self, map_function: F) -> Stream<U>
    where
        U: 'static,
        F: Fn(Arc<T>) -> Arc<U>,
        F: Send,
        F: Sync,
        F: 'static,
    {
        self.create_derived_stream(move |host, val| {
            host.emit_rc(map_function(val));
        })
    }

    /// Returns a stream that can emit multiple values for each value from the original stream.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    ///
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
    /// let stream = stream_host.get_stream();
    ///
    /// let last_value = Arc::new(Mutex::new(0_i32));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream
    ///     .flat_map(|val| vec![*val, val + 1, val + 2])
    ///     .subscribe(move |val| {
    ///          *last_value_write.lock().unwrap() = *val;
    ///     });
    ///
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), 4); /* Emitted 2, 3, then 4 */
    ///
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), 5); /* Emitted 3, 4, then 5 */
    /// ```
    pub fn flat_map<U, F, C>(&self, iter_map_function: F) -> Stream<U>
    where
        U: 'static,
        C: IntoIterator<Item = U>,
        F: Fn(&T) -> C,
        F: Send,
        F: Sync,
        F: 'static,
    {
        self.create_derived_stream(move |host, val| {
            let mut iter = iter_map_function(&*val).into_iter();
            loop {
                match iter.next() {
                    Some(x) => host.emit_rc(Arc::new(x)),
                    None => break,
                }
            }
        })
    }

    /// Similar to subscribing to a stream in that `inspect_function` runs whenever the
    /// stream emits, but returns a derived stream matching the original stream instead of
    /// a SubscriptionRef.
    pub fn inspect<F>(&self, inspect_function: F) -> Stream<T>
    where
        F: Fn(&T),
        F: Send,
        F: Sync,
        F: 'static,
    {
        self.create_derived_stream(move |host, val| {
            inspect_function(&*val);
            host.emit_rc(val.clone());
        })
    }
}
