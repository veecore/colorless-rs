use colorless::colorless;

#[colorless]
fn unsupported() -> usize {
    let value = if true { 1 };
    value
}

fn main() {}
