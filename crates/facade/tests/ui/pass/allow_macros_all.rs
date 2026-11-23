use colorless::colorless;

macro_rules! my_log {
    ($msg:expr) => { let _ = $msg; };
}

macro_rules! another {
    () => { () };
}

#[colorless(allow_macros(all))]
fn greet(name: &str) -> &str {
    my_log!(name);
    let _ = another!();
    name
}

fn main() {}
