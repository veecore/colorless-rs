use colorless::{IdentityExt, colorless};
use colorless_std::iter::{IntoColoredIterator, ColoredIterator, from_iter};

#[colorless(C)]
fn borrowed_items<'a>(items: impl IntoColoredIterator<C, Item = &'a str>) {
    #[colorless]
    for item in items {
        let _ = item;
    }
}

fn main() {
    borrowed_items(from_iter(["a", "b"])).sync_();
}
