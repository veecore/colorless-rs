use colorless::colorless;

macro_rules! my_log {
    ($msg:expr) => { let _ = $msg; };
}

// No allow_macros — custom macro must be rejected
#[colorless]
fn greet(name: &str) -> &str {
    my_log!(name);
    name
}

fn main() {}
