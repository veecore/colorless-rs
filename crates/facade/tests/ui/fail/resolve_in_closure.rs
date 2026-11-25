use colorless::{colorless, ready};

// This isn't meant to be explicitly rejected.... if the output of this ready had a resolve field,
// then rust shouldn't error, but currently it'd because of how inert works aggressively
#[colorless]
fn ex16() {
    let f = || { ready(1).resolve };
    f();
}

fn main() {}