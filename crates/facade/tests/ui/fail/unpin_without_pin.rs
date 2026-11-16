use colorless::{Color, Colored, colorless};

fn gate<C: Color>() -> impl Colored<C, Output = ()> {
    C::ready(())
}

#[colorless]
fn unsupported() {
    let mut local = String::from("local");
    unpin_local!(local);
    gate().resolve;
}

fn main() {}
