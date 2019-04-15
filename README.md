# Reactive
### Streaming reactive programming for Rust

This library is similar to the Rx family of libraries, but simpler and designed natively for Rust.

## Example

```rust
let player_1_points = reactive::ReactiveValue::new(4);
let player_1_multiplier = reactive::ReactiveValue::new(1.0_f32);

let player_2_points = reactive::ReactiveValue::new(5);
let player_2_multiplier = reactive::ReactiveValue::new(1.0_f32);

let player_1_score = computed!(*player_1_points as f32 * player_1_multiplier);
let player_2_score = computed!(*player_2_points as f32 * player_2_multiplier);

let winner = computed! {
    if (player_1_score > player_2_score) {
        "Player 1"
    } else {
        "Player 2"
    }
};

assert_eq!(*player_1_score.get(), 4_f32);
assert_eq!(*player_2_score.get(), 5_f32);
assert_eq!(*winner.get(), "Player 2");

player_1_multiplier.set(2.5_f32);

assert_eq!(*player_1_score.get(), 10_f32);
assert_eq!(*player_2_score.get(), 5_f32);
assert_eq!(*winner.get(), "Player 1");
```

See the rustdoc for more.