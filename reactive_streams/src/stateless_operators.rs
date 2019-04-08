use super::{Stream, Subscription};
use std::rc::Rc;

pub struct DerivedStreamFields<T, ExtraFieldsType> {
    #[allow(dead_code)]
    subscription: Option<Subscription<T, ExtraFieldsType>>,
}

impl<T: 'static, ExtraFieldsType: 'static> Stream<T, ExtraFieldsType> {
    fn create_derived_stream<U, F>(
        &self,
        subscription_re_emit: F,
    ) -> Stream<U, DerivedStreamFields<T, ExtraFieldsType>>
    where
        F: Fn(&Stream<U, DerivedStreamFields<T, ExtraFieldsType>>, Rc<T>),
        F: 'static,
        U: 'static,
    {
        let mut derived_stream =
            Stream::new_with_fields(DerivedStreamFields { subscription: None });
        let subscription_stream_ref = derived_stream.clone();

        let subscription =
            self.subscribe(move |val| subscription_re_emit(&subscription_stream_ref, val));

        match (&mut derived_stream).pointer.lock() {
            Ok(mut mut_ref) => {
                mut_ref.extra_fields.subscription = Some(subscription);
            }
            Err(err) => panic!("Stream mutex poisoned: {}", err),
        };

        derived_stream
    }

    /// Returns a stream that emits only those values from the original stream that pass a test.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    ///
    /// let stream_host: reactive_streams::StreamHost<i32> = reactive_streams::StreamHost::new();
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
    pub fn filter<F>(
        &self,
        filter_function: F,
    ) -> Stream<T, DerivedStreamFields<T, ExtraFieldsType>>
    where
        F: Fn(&T) -> bool,
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
    /// let stream_host: reactive_streams::StreamHost<i32> = reactive_streams::StreamHost::new();
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
    pub fn map<U, F>(&self, map_function: F) -> Stream<U, DerivedStreamFields<T, ExtraFieldsType>>
    where
        U: 'static,
        F: Fn(&T) -> U,
        F: 'static,
    {
        self.create_derived_stream(move |host, val| {
            host.emit_rc(Rc::new(map_function(&*val)));
        })
    }

    /// Returns a stream that can emit multiple values for each value from the original stream.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    ///
    /// let stream_host: reactive_streams::StreamHost<i32> = reactive_streams::StreamHost::new();
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
    pub fn flat_map<U, F, C>(
        &self,
        iter_map_function: F,
    ) -> Stream<U, DerivedStreamFields<T, ExtraFieldsType>>
    where
        U: 'static,
        C: IntoIterator<Item = U>,
        F: Fn(&T) -> C,
        F: 'static,
    {
        self.create_derived_stream(move |host, val| {
            let mut iter = iter_map_function(&*val).into_iter();
            loop {
                match iter.next() {
                    Some(x) => host.emit_rc(Rc::new(x)),
                    None => break,
                }
            }
        })
    }

    /// Similar to subscribing to a stream in that `inspect_function` runs whenever the
    /// stream emits, but returns a derived stream matching the original stream instead of
    /// a SubscriptionRef.
    pub fn inspect<F>(
        &self,
        inspect_function: F,
    ) -> Stream<T, DerivedStreamFields<T, ExtraFieldsType>>
    where
        F: Fn(&T),
        F: 'static,
    {
        self.create_derived_stream(move |host, val| {
            inspect_function(&*val);
            host.emit_rc(val.clone());
        })
    }
}
