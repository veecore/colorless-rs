/// The async version of [Optional](crate::Optional)
#[doc(alias = "AsyncOption")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptionalAsync;

pub use impls::OptionalAsyncExt;

mod impls {
    use crate::OptionalAsync;

    macro_rules! native_exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                None => None,
                Some($v) => $ok,
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                None => None,
                Some($v) => Some($ok),
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                Some(ControlFlow::Break(b)) => return Some(b),
                Some(ControlFlow::Continue($v)) => $ok,
                None => return None,
            }
        };
        (@init $expr:expr) => {
            Some($expr)
        };
    }

    macro_rules! poll_exec {
        (@init $value:expr) => {
            Some($value)
        };
        (
            $t:expr;

            on_pending: $p:block

            on_ready: $r:block

            $output:ident on_ready_ok: $ok:block
        ) => {
            match $t {
                Poll::Pending => $p,
                Poll::Ready(None) => {
                    $r;
                    return Poll::Ready(None);
                },
                Poll::Ready(Some($output)) => $ok,
            }
        };
        (
            @loop

            $t:expr;

            on_pending: $p:block

            on_ready: $r:block

            $output:ident on_ready_ok: $ok:block
        ) => {
            match $t {
                Poll::Pending => $p,
                Poll::Ready(None) => {
                    $r;
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(ControlFlow::Break(value))) => {
                    $r;
                    return Poll::Ready(Some(value));
                }
                Poll::Ready(Some(ControlFlow::Continue($output))) => $ok,
            }
        };
    }

    declare_async_all! {
        OptionalAsyncExt,
        <> OptionalAsync
        Option<Self::Output>,
        Option<T>,
        async_option_,
        native_exec,
        poll_exec,
    }
}