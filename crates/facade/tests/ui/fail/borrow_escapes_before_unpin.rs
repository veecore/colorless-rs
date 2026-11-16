use colorless::{Color, Colored, colorless};

fn retains_borrow<'a, C: Color>(value: &'a mut usize) -> impl Colored<C, Output = ()> + use<'a, C> {
    C::wrap(move || *value += 1)
}

#[colorless]
fn unsupported() {
    let mut local = 0;
    pin_local!(local);
    let retained = retains_borrow(local);
    unpin_local!(local);
    retained.resolve;
}

fn main() {}
