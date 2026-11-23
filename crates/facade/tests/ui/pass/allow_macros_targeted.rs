use colorless::colorless;

macro_rules! my_log {
    ($msg:expr) => { let _ = $msg; };
}

#[colorless(allow_macros(my_log))]
fn greet(name: &str) -> &str {
    my_log!(name);
    name
}

fn main() {}
