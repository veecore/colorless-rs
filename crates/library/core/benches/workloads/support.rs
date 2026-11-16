use tokio::runtime::{Builder, Runtime};

/// Construct one current-thread runtime per Criterion group. Criterion enters
/// it once per sample and awaits all measured iterations inside that entry, so
/// runtime startup and repeated `block_on` calls do not dominate short futures.
pub(crate) fn runtime() -> Runtime {
    Builder::new_current_thread()
        .build()
        .expect("benchmark runtime must build")
}
