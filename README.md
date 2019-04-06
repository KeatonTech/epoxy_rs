# Reactive
### Streaming reactive programming for Rust

This library is similar to the Rx family of libraries, but simpler and designed natively for Rust.

## Example

```rust
let stream_host: reactive::StreamHost<i32> = reactive::StreamHost::new();
let stream = stream_host.get_stream();

let reactive_value = reactive::ReactiveValue::from_stream(stream, 0);
assert_eq!(*reactive_value.get_value(), 0);

stream_host.emit(1);
assert_eq!(*reactive_value.get_value(), 1);

stream_host.emit(3);
assert_eq!(*reactive_value.get_value(), 3);
```

See the rustdoc for more.