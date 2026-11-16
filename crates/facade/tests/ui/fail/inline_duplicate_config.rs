use colorless::{Identity, color};

fn main() {
    let _ = color! {
        #![colorless(color = Identity)]
        #![colorless(color = Identity)]
        1
    };
}
