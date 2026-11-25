use colorless::{colorless, ready};

// making this work means destroying potential O(1) for sync and
// also introducing bugs since we couldn't know which other arms
// target base simply with syn... we could make exemption for two
// arms though
#[colorless]
fn ex18(x: i32) {
    match x {
        n if ready(n).resolve > 0 => println!("Pos"),
        _ => println!("Neg"),
    }
}

fn main() {}