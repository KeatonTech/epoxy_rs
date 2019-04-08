#[derive(PartialEq)]
enum MouseButton {
    Right,
    Middle,
    Left,
}

struct MyButton {
    click_host: reactive::StreamHost<MouseButton>,
}

impl MyButton {
    pub fn new() -> MyButton {
        MyButton {
            click_host: reactive::StreamHost::new(),
        }
    }

    pub fn get_clicks(&self) -> reactive::BaseStream<MouseButton> {
        self.click_host.get_stream()
    }

    pub fn click(&self, button: MouseButton) {
        self.click_host.emit(button)
    }
}

fn main() {
    let button = MyButton::new();
    let all_clicks_count = button.get_clicks().count_values();
    let left_clicks_count = button
        .get_clicks()
        .filter(|click| *click == MouseButton::Left)
        .count_values();

    button.click(MouseButton::Left);
    button.click(MouseButton::Right);
    button.click(MouseButton::Left);
    button.click(MouseButton::Middle);
    button.click(MouseButton::Left);

    {
        let _sub_all = all_clicks_count.subscribe(|count| {
            assert_eq!(*count, 6);
            println!("Counted 6 clicks");
        });
        let _sub_left = left_clicks_count.subscribe(|count| {
            assert_eq!(*count, 4);
            println!("Counted 4 left clicks");
        });
        button.click(MouseButton::Left);
    }
}
