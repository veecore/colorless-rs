use colorless::{Color, Colored, colorless};

fn gate<C: Color>() -> impl Colored<C, Output = ()> {
    C::ready(())
}

#[colorless]
fn unsupported() {
    let mut state = 0;
    #[colorless::state(mut state, state)]
    loop {
        gate().resolve;
        state += 1;
        break;
    }
}

fn main() {}
