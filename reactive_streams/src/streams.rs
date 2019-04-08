use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

/// Used to indicate that a stream has no extra fields, which are used to create derived streams.
pub struct EmptyStruct {}

pub(crate) struct StreamImpl<T, ExtraFieldsType> {
    highest_id: u16,
    is_alive: bool,
    on_emit: HashMap<u16, Box<Fn(Rc<T>)>>,
    pub(crate) extra_fields: Box<ExtraFieldsType>,
}

pub struct Stream<T, ExtraFieldsType> {
    pub(crate) pointer: Rc<Mutex<StreamImpl<T, ExtraFieldsType>>>,
}

pub type BaseStream<T> = Stream<T, EmptyStruct>;

pub struct Subscription<T, ExtraFieldsType> {
    id: u16,
    stream: Stream<T, ExtraFieldsType>,
}

pub struct StreamHost<T> {
    stream: Stream<T, EmptyStruct>,
}

impl<T, U> Clone for Stream<T, U> {
    fn clone(&self) -> Self {
        Stream {
            pointer: Rc::clone(&self.pointer),
        }
    }
}

impl<T, ExtraFieldsType> StreamImpl<T, ExtraFieldsType> {
    fn subscribe<F>(&mut self, listener: F) -> u16
    where
        F: Fn(Rc<T>),
        F: 'static,
    {
        let new_subscription_id = self.highest_id;
        self.highest_id += 1;
        self.on_emit.insert(new_subscription_id, Box::new(listener));
        new_subscription_id
    }

    fn emit_rc(&self, value: Rc<T>) {
        for (_id, call) in &self.on_emit {
            call(value.clone())
        }
    }
}

/// # Examples
///
/// ```
/// let stream_host: reactive_streams::StreamHost<i32> = reactive_streams::StreamHost::new();
/// let stream = stream_host.get_stream();
/// {
///     let _sub = stream.subscribe(|val| {val;});
///     assert_eq!(stream.count_subscribers(), 1);
/// }
/// assert_eq!(stream.count_subscribers(), 0);
/// ```
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
/// let subscription = stream.subscribe(move |val| {
///     *last_value_write.lock().unwrap() = *val;
/// });
///
/// stream_host.emit(1);
/// assert_eq!(*last_value.lock().unwrap(), 1);
///
/// stream_host.emit(100);
/// assert_eq!(*last_value.lock().unwrap(), 100);
/// ```
impl<T, ExtraFieldsType> Stream<T, ExtraFieldsType> {
    pub fn subscribe<F>(&self, listener: F) -> Subscription<T, ExtraFieldsType>
    where
        F: Fn(Rc<T>),
        F: 'static,
    {
        let mut stream_mut = match self.pointer.lock() {
            Ok(mut_ref) => mut_ref,
            Err(err) => panic!("Stream mutex poisoned: {}", err),
        };

        Subscription {
            id: stream_mut.subscribe(listener),
            stream: self.clone(),
        }
    }

    pub fn unsubscribe(&self, _subscription: Subscription<T, ExtraFieldsType>) {
        // By moving the subscription into this function it will automatically get dropped,
        // thereby calling the internal unsubscribe_by_id function.
    }

    pub fn count_subscribers(&self) -> usize {
        let stream = match self.pointer.lock() {
            Ok(stream_impl) => stream_impl,
            Err(err) => panic!("Stream mutex poisoned: {}", err),
        };
        stream.on_emit.len()
    }

    fn unsubscribe_by_id(&self, subscription_id: u16) {
        let mut stream_mut = match self.pointer.lock() {
            Ok(mut_ref) => mut_ref,
            Err(err) => panic!("Stream mutex poisoned: {}", err),
        };
        stream_mut.on_emit.remove(&subscription_id);
    }

    // PRIVATE FUNCTIONS

    pub(crate) fn new_with_fields(fields: ExtraFieldsType) -> Stream<T, ExtraFieldsType> {
        Stream {
            pointer: Rc::new(Mutex::new(StreamImpl {
                highest_id: 0_u16,
                is_alive: true,
                on_emit: HashMap::new(),
                extra_fields: Box::new(fields),
            })),
        }
    }

    pub(crate) fn emit_rc(&self, value: Rc<T>) {
        match self.pointer.lock() {
            Ok(stream_impl) => stream_impl.emit_rc(value),
            Err(err) => panic!("Stream mutex poisoned: {}", err),
        }
    }
}

impl<T> StreamHost<T> {
    pub fn new() -> StreamHost<T> {
        StreamHost {
            stream: Stream::new_with_fields(EmptyStruct {}),
        }
    }

    pub fn get_stream(&self) -> BaseStream<T> {
        self.stream.clone()
    }

    pub fn emit(&self, value: T) {
        self.emit_rc(Rc::new(value))
    }

    pub fn emit_rc(&self, value: Rc<T>) {
        self.stream.emit_rc(value)
    }
}

impl<T> Drop for StreamHost<T> {
    fn drop(&mut self) {
        let mut stream_mut = match self.stream.pointer.lock() {
            Ok(mut_ref) => mut_ref,
            Err(err) => panic!("Stream mutex poisoned: {}", err),
        };
        stream_mut.is_alive = false;
    }
}

impl<T, U> Drop for Subscription<T, U> {
    fn drop(&mut self) {
        self.stream.unsubscribe_by_id(self.id)
    }
}
