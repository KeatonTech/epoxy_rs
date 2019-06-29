use super::{Stream, Subscription};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock, RwLockReadGuard};

/// Stores the last N values emitted by a stream. It is similar to ReactiveValue (which
/// stores the most recent value emitted by a stream), but cannot be used to re-host
/// a stream (if you need that, use the `buffer` stream operator instead).
///
/// ReactiveCache has some practical uses in reactive implementations, but is especially
/// helpful inside unit tests to monitor streams.
///
/// # Examples
/// ```
/// use epoxy_streams::ReactiveCache;
///
/// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
/// let stream = stream_host.get_stream();
///
/// let cache = ReactiveCache::from_stream(stream);
/// assert_eq!(cache.get().len(), 0);
///
/// stream_host.emit(100);
/// assert_eq!(cache.get().len(), 1);
/// assert_eq!(*cache.get()[0], 100);
/// ```
pub struct ReactiveCache<T: 'static> {
    cache: Arc<RwLock<VecDeque<Arc<T>>>>,

    #[allow(dead_code)]
    subscription: Subscription<T>,
}

impl<T: Send + Sync + 'static> ReactiveCache<T> {
    /// Constructs a new infinite-size ReactiveCache from a stream. This cache will
    /// store all items emitted by the stream until explicitly cleared.
    pub fn from_stream(stream: Stream<T>) -> ReactiveCache<T> {
        ReactiveCache::from_stream_with_max_size_option(stream, None)
    }

    /// Constructs a new finite-size ReactiveCache from a stream. This cache will hold
    /// at most the most recent `size` values emitted by the stream.
    ///
    /// # Examples
    /// ```
    /// use epoxy_streams::ReactiveCache;
    ///
    /// let stream_host: epoxy_streams::Sink<i32> = epoxy_streams::Sink::new();
    /// let stream = stream_host.get_stream();
    ///
    /// let cache = ReactiveCache::from_stream_with_size(stream, 2);
    /// assert_eq!(cache.get().len(), 0);
    ///
    /// stream_host.emit(100);
    /// assert_eq!(cache.get().len(), 1);
    /// stream_host.emit(200);
    /// assert_eq!(cache.get().len(), 2);
    /// stream_host.emit(300);
    /// assert_eq!(cache.get().len(), 2);
    /// assert_eq!(*cache.get()[0], 200);
    /// assert_eq!(*cache.get()[1], 300);
    /// ```
    pub fn from_stream_with_size(stream: Stream<T>, size: usize) -> ReactiveCache<T> {
        ReactiveCache::from_stream_with_max_size_option(stream, Some(size))
    }

    fn from_stream_with_max_size_option(
        stream: Stream<T>,
        max_size: Option<usize>,
    ) -> ReactiveCache<T> {
        let value_arc = Arc::new(RwLock::new(match max_size {
            Some(size) => VecDeque::with_capacity(size),
            None => VecDeque::new(),
        }));

        let subscription_arc = value_arc.clone();
        let subscription = stream.subscribe(move |val| {
            if let Some(size) = max_size {
                let vec_len = subscription_arc.read().unwrap().len();
                if vec_len >= size {
                    subscription_arc.write().unwrap().pop_front();
                }
            }
            subscription_arc.write().unwrap().push_back(val.clone());
        });

        ReactiveCache {
            cache: value_arc,
            subscription: subscription,
        }
    }

    /// Returns a VecDeque containing recent values emitted by the stream, ordered such that
    /// the newest values are at the back of the queue.
    pub fn get(&self) -> RwLockReadGuard<VecDeque<Arc<T>>> {
        self.cache.read().unwrap()
    }

    /// Similar to `get()`, but pulls all of the values out of their Arc containers, for easier
    /// testing and referencing.
    pub fn get_cloned(&self) -> VecDeque<T> where T: Clone  {
        self.get().iter().map(|val| (**val).clone()).collect()
    }

    /// Removes all values from the queue, freeing memory.
    pub fn clear(&self) {
        self.cache.write().unwrap().clear()
    }
}
