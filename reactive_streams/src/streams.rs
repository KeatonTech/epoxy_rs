use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

/// Used to indicate that a stream has no extra fields, which are used to create derived streams.
pub struct EmptyStruct {}

pub(crate) struct StreamImpl<T> {
    highest_id: u16,
    is_alive: bool,
    on_emit: HashMap<u16, Box<Fn(Rc<T>)>>,
    pub(crate) extra_fields: *mut (dyn Any + 'static),
}

pub struct Stream<T> {
    pub(crate) pointer: Rc<Mutex<StreamImpl<T>>>,
}

pub struct Subscription<T> {
    id: u16,
    pub(crate) stream: Stream<T>,
}

pub struct StreamHost<T> {
    stream: Stream<T>,
}

impl<T> Clone for Stream<T> {
    fn clone(&self) -> Self {
        Stream {
            pointer: Rc::clone(&self.pointer),
        }
    }
}

impl<T> StreamImpl<T> {
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

    pub(crate) fn emit_rc(&self, value: Rc<T>) {
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
impl<T> Stream<T> {
    pub fn subscribe<F>(&self, listener: F) -> Subscription<T>
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

    pub fn unsubscribe(&self, _subscription: Subscription<T>) {
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

    pub(crate) fn new_with_fields<FieldsType>(fields: FieldsType) -> Stream<T>
    where
        FieldsType: 'static,
    {
        Stream {
            pointer: Rc::new(Mutex::new(StreamImpl {
                highest_id: 0_u16,
                is_alive: true,
                on_emit: HashMap::new(),
                extra_fields: Box::into_raw(Box::new(fields)),
            })),
        }
    }

    pub(crate) fn emit_rc(&self, value: Rc<T>) {
        match self.pointer.lock() {
            Ok(stream_impl) => stream_impl.emit_rc(value),
            Err(err) => panic!("Stream mutex poisoned: {}", err),
        }
    }

    pub(crate) fn read_extra_fields<ExtraFieldsType, RetType, FnType>(&self, cb: FnType) -> RetType
    where
        ExtraFieldsType: 'static,
        RetType: 'static,
        FnType: FnOnce(&ExtraFieldsType) -> RetType,
    {
        match self.pointer.lock() {
            Ok(stream_impl) => unsafe {
                let any_box = Box::from_raw(stream_impl.extra_fields);
                match any_box.downcast::<ExtraFieldsType>() {
                    Ok(fields) => {
                        let ret = cb(&*fields);
                        let _nofree = Box::into_raw(fields);
                        ret
                    }
                    Err(_) => panic!("Invalid type for derived stream field."),
                }
            },
            Err(err) => panic!("Stream mutex poisoned: {}", err),
        }
    }

    pub(crate) fn mutate_extra_fields<ExtraFieldsType, FnType>(&self, cb: FnType)
    where
        ExtraFieldsType: 'static,
        FnType: FnOnce(&mut ExtraFieldsType),
    {
        match self.pointer.lock() {
            Ok(stream_impl) => unsafe {
                let any_box = Box::from_raw(stream_impl.extra_fields);
                match any_box.downcast::<ExtraFieldsType>() {
                    Ok(mut fields) => {
                        cb(&mut *fields);
                        let _nofree = Box::into_raw(fields);
                    }
                    Err(_) => panic!("Invalid type for derived stream field."),
                }
            },
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

    pub fn get_stream(&self) -> Stream<T> {
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

impl<T> Drop for StreamImpl<T> {
    fn drop(&mut self) {
        unsafe {
            let _extra_fields_box = Box::from_raw(self.extra_fields);
        }
    }
}

impl<T> Drop for Subscription<T> {
    fn drop(&mut self) {
        self.stream.unsubscribe_by_id(self.id)
    }
}
