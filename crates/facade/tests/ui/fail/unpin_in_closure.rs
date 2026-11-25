use colorless::colorless;

#[colorless(Z)]
pub fn run() {
    let a = &mut 1;
    let mut f = || {
        unpin!(a);
    };
    f()
}
