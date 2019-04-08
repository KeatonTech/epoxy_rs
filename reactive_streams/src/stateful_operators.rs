use super::{Stream, Subscription};
use std::rc::Rc;

pub struct StatefulDerivedStreamFields<T, StateType, ExtraFieldsType> {
    state: StateType,

    #[allow(dead_code)]
    subscription: Option<Subscription<T, ExtraFieldsType>>,
}

impl<T: 'static, ExtraFieldsType: 'static> Stream<T, ExtraFieldsType> {
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
    /// let stream_host: reactive_streams::StreamHost<i32> = reactive_streams::StreamHost::new();
    /// let stream = stream_host.get_stream();
    ///
    /// let last_value = Arc::new(Mutex::new(vec![]));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream
    ///     .scan(|acc, val| {
    ///         let mut extended = vec![];
    ///         extended.extend(acc);
    ///         extended.push(*val);
    ///         extended
    ///     }, vec![])
    ///     .subscribe(move |val| {
    ///          *last_value_write.lock().unwrap() = (*val).clone();
    ///     });
    ///
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), vec![2]);
    ///
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), vec![2, 3]);
    /// ```
    pub fn scan<U, F>(
        &self,
        scan_fn: F,
        initial_value: U,
    ) -> Stream<U, StatefulDerivedStreamFields<T, Rc<U>, ExtraFieldsType>>
    where
        F: Fn(&U, Rc<T>) -> U,
        F: 'static,
        U: 'static,
    {
        let mut derived_stream = Stream::new_with_fields(StatefulDerivedStreamFields {
            state: Rc::new(initial_value),
            subscription: None,
        });
        let subscription_stream_ref = derived_stream.clone();

        let subscription = self.subscribe(move |val| {
            let new_state = match subscription_stream_ref.pointer.lock() {
                Ok(stream_impl) => Rc::new(scan_fn(&*stream_impl.extra_fields.state, val)),
                Err(err) => panic!("Stream mutex poisoned: {}", err),
            };
            match subscription_stream_ref.pointer.lock() {
                Ok(mut mut_ref) => {
                    mut_ref.extra_fields.state = Rc::clone(&new_state);
                }
                Err(err) => panic!("Stream mutex poisoned: {}", err),
            };
            subscription_stream_ref.emit_rc(new_state);
        });

        match (&mut derived_stream).pointer.lock() {
            Ok(mut mut_ref) => {
                mut_ref.extra_fields.subscription = Some(subscription);
            }
            Err(err) => panic!("Stream mutex poisoned: {}", err),
        };

        derived_stream
    }

    /// Creates a stream that counts the number of times the original stream emits.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    ///
    /// let stream_host: reactive_streams::StreamHost<i32> = reactive_streams::StreamHost::new();
    /// let stream = stream_host.get_stream();
    ///
    /// let last_value = Arc::new(Mutex::new(0_u32));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream.count_values().subscribe(move |val| {
    ///     *last_value_write.lock().unwrap() = (*val).clone();
    /// });
    ///
    /// stream_host.emit(2);
    /// assert_eq!(*last_value.lock().unwrap(), 1);
    ///
    /// stream_host.emit(3);
    /// assert_eq!(*last_value.lock().unwrap(), 2);
    /// ```
    pub fn count_values(
        &self,
    ) -> Stream<u32, StatefulDerivedStreamFields<T, Rc<u32>, ExtraFieldsType>> {
        self.scan(|acc, _| acc + 1, 0)
    }

    /// Creates a stream that returns a vector of the last `n` emissions of the original stream.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    ///
    /// let stream_host: reactive_streams::StreamHost<i32> = reactive_streams::StreamHost::new();
    /// let stream = stream_host.get_stream();
    ///
    /// let last_value = Arc::new(Mutex::new(vec![]));
    /// let last_value_write = last_value.clone();
    ///
    /// let subscription = stream.buffer(2).subscribe(move |val| {
    ///     *last_value_write.lock().unwrap() = (*val).clone();
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
    pub fn buffer(
        &self,
        max_buffer_size: usize,
    ) -> Stream<Vec<T>, StatefulDerivedStreamFields<T, Rc<Vec<T>>, ExtraFieldsType>>
    where
        T: Clone,
    {
        self.scan(
            move |acc, val| {
                let mut extended = vec![];
                if acc.len() > 0 && acc.len() < max_buffer_size {
                    extended.push(acc[0].clone());
                }
                for i in 1..acc.len() {
                    extended.push(acc[i].clone());
                }
                extended.push((*val).clone());
                extended
            },
            vec![],
        )
    }
}
