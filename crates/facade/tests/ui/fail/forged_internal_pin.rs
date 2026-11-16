use colorless::{Color, Colored, colorless};

fn gate<C: Color>() -> impl Colored<C, Output = ()> {
    C::ready(())
}

#[colorless]
fn forged() {
    let mut local = String::from("local");
    pin_local!(local);
    gate().resolve;
    __unpin_local!(local);
}

fn main() {}
