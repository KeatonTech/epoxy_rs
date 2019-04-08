#[macro_use]
extern crate reactive;

fn main() {
    let mut x = 1;
    let result = computed! {
        let y = 3;
        x + y
    };
    println!("{}", result);
}
