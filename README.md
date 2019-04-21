# Reactive Streams for Rust.
### Reactive Programming the Rust way

This library provides 2 basic reactive programming primitives. `Stream` represents a stateless
pipe of data that can be subscribed to and handled asynchronously. `ReactiveValue` represents
a piece of data whose value can change over time (similar to an Atomic, but backed by streams
so that dependents be alerted when its value changes). These two primitives loosely correspond
to 'Stream' and 'BehaviorSubject' respectively in the Rx family of libraries.

One unique feature of this library is that stream subscriptions only last as long as the
subscription object stays in scope, preventing a many of the memory leaks and zombie callback
problems common in reactive code.

```
let stream_host: epoxy::Sink<i32> = epoxy::Sink::new();
let stream = stream_host.get_stream();
{
    let _sub = stream.subscribe(|val| println!("Emitted {}", val));
    stream_host.emit(1); // 'Emitted 1' is printed
    assert_eq!(stream.count_subscribers(), 1);
}
stream_host.emit(2); // Nothing is printed
assert_eq!(stream.count_subscribers(), 0);
```

Streams can be manipulated using a library of built-in functions based on Rust's set of
iterator operations. Currently these operations include:

| Operation          | Property of returned stream                                            |
|--------------------|------------------------------------------------------------------------|
| map(fn)            | Runs all values from the input stream through a mapper function        |
| map_rc(fn)         | Same as map() but the mapper function takes and returns an Rc          |
| flat_map(fn)       | Similar to map() but iterates through the result of the mapper function|
| filter(fn)         | Returns only input values that pass the given filter function          |
| inspect(method)    | Passes through the original stream, calls a method for each item       |
| scan(fn, default)  | Similar to reduce(), but returns the value after each iteration        |
| count_values()     | Returns the number of times the stream has emitted                     |
| buffer(size)       | Collects emitted values into vectors of length `size`                  |

ReactiveValues have their own set of operators, although it is also possible to get a reference
to the underlying stream of a ReactiveValue with `.as_stream()` and use any of the above
operations as well.

| Operation             | Property of returned reactive value                                 |
|-----------------------|---------------------------------------------------------------------|
| map(fn)               | Runs all values from the input stream through a mapper function     |
| sanitize(fn, default) | Does not change the value if the input does not pass a test fn      |
| fallback(fn, fallback)| Changes the value to `fallback` if the input does not pass a test fn|

However, this library also ships with a `computed!` macro that makes dealing with ReactiveValue
just as easy as dealing with any other Rust variable.

```
# #[macro_use] extern crate reactive;
use epoxy::ReactiveValue;

let points = epoxy::ReactiveValue::new(4);
let multiplier = epoxy::ReactiveValue::new(1);
let score = computed!(points * multiplier);

assert_eq!(*score.get(), 4);

multiplier.set(2);
assert_eq!(*score.get(), 8);
```

## Comparison to Rx

These streams are intended to be substantially simpler than those in the Rx family of
libraries. The most significant difference is that this library has no concept of a 'cold'
stream, meaning no streams will ever emit a value immediately upon subscription. Streams
in this library also never close, as they are intended to model long-term asynchronous data
flows (of course it is possible to make a stream 'closeable' by making a stream of Option
enums and unsubscribing on `None`, that just isn't built in to the library). Finally, where
Rx subscriptions live until explicitly unsubscribed, Rust Reactive subscriptions only live
as long as they are in scope.

## Status

This crate is under active development and is probably not ready for production use yet.