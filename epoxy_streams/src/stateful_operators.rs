use super::{Stream, Subscription};
use std::sync::Arc;

pub struct StatefulDerivedStreamFields<T, StateType> {
    state: StateType,

    #[allow(dead_code)]
    subscription: Option<Subscription<T>>,
}

impl<T: 'static> Stream<T> {
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
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
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
    pub fn scan<U, F>(&self, scan_fn: F, initial_value: U) -> Stream<U>
    where
        F: Fn(&U, Arc<T>) -> U,
        F: Send,
        F: Sync,
        F: 'static,
        U: 'static,
        U: Send,
        U: Sync,
    {
        let derived_stream = Stream::new_with_fields::<StatefulDerivedStreamFields<T, Arc<U>>>(
            StatefulDerivedStreamFields {
                state: Arc::new(initial_value),
                subscription: None,
            },
        );
        let subscription_stream_ref = derived_stream.clone();

        let subscription = self.subscribe(move |val| {
            let new_state = subscription_stream_ref.read_extra_fields(
                |fields: &StatefulDerivedStreamFields<T, Arc<U>>| {
                    Arc::new(scan_fn(&fields.state, val))
                },
            );

            subscription_stream_ref.mutate_extra_fields(
                |fields: &mut StatefulDerivedStreamFields<T, Arc<U>>| {
                    fields.state = Arc::clone(&new_state)
                },
            );
            subscription_stream_ref.emit_rc(new_state);
        });

        derived_stream.mutate_extra_fields(
            move |fields: &mut StatefulDerivedStreamFields<T, Arc<U>>| {
                fields.subscription = Some(subscription);
            },
        );

        derived_stream
    }

    /// Creates a stream that counts the number of times the original stream emits.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    ///
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
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
    pub fn count_values(&self) -> Stream<u32> {
        self.scan(|acc, _| acc + 1, 0)
    }

    /// Creates a stream that returns a vector of the last `n` emissions of the original stream.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    ///
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
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
    pub fn buffer(&self, max_buffer_size: usize) -> Stream<Vec<T>>
    where
        T: Clone,
        T: Send,
        T: Sync,
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

    /// Creates a stream that filters out repeated values. So a stream that emits
    /// the sequence (1, 1, 2, 3) would be transformed into a stream that emits
    /// (1, 2, 3). Note that this does _not_ dedup the entire stream, it just prevents
    /// the same value from being emitted twice in a row. So (1, 1, 2, 1, 3) would
    /// turn into (1, 2, 1, 3), not (1, 2, 3).
    /// 
    /// # Examples
    /// ```
    /// use std::sync::{Arc, Mutex};
    ///
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
    /// let stream = stream_host.get_stream();
    /// let distinct = stream.distinct_until_changed();
    /// let cache = epoxy_streams::ReactiveCache::from_stream(distinct);
    ///
    /// stream_host.emit(2);
    /// assert_eq!(cache.get_cloned(), vec![2]);
    ///
    /// stream_host.emit(2);
    /// assert_eq!(cache.get_cloned(), vec![2]);
    ///
    /// stream_host.emit(3);
    /// assert_eq!(cache.get_cloned(), vec![2, 3]);
    /// ```
    pub fn distinct_until_changed(&self) -> Stream<T> where 
        T: Send,
        T: Sync,
        T: Eq
    {
        let derived_stream = Stream::new_with_fields::<StatefulDerivedStreamFields<T, Option<Arc<T>>>>(
            StatefulDerivedStreamFields {
                state: None,
                subscription: None,
            },
        );
        let subscription_stream_ref = derived_stream.clone();

        let subscription = self.subscribe(move |val| {
            let is_duplicate = subscription_stream_ref.read_extra_fields(
                |fields: &StatefulDerivedStreamFields<T, Option<Arc<T>>>| {
                   if let Some(last_val) = &fields.state {
                       last_val == &val
                   } else {
                       false
                   }
                }
            );

            if !is_duplicate {
                subscription_stream_ref.mutate_extra_fields(
                    |fields: &mut StatefulDerivedStreamFields<T, Option<Arc<T>>>| {
                        fields.state = Some(val.clone());
                    }
                );
                subscription_stream_ref.emit_rc(val);
            }
        });

        derived_stream.mutate_extra_fields(
            move |fields: &mut StatefulDerivedStreamFields<T, Option<Arc<T>>>| {
                fields.subscription = Some(subscription);
            },
        );

        derived_stream
    }
}
