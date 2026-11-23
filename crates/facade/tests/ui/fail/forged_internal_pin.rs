use colorless::{Color, Colored, colorless};

fn gate<C: Color>() -> impl Colored<C, Output = ()> {
    C::ready(())
}

// this is no longer valid
#[colorless]
fn forged() {
    let mut local = String::from("local");
    pin!(local);
    gate().resolve;
    __unpin!(local);
}

fn main() {}
