use core::marker::PhantomData;

/// The async version of [OptionalResult](crate::OptionalResult)
#[doc(alias = "AsyncOptionResult")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptionalResultAsync<E>(pub(crate) PhantomData<E>);

pub use impls::OptionalResultAsyncExt;

mod impls {
    use crate::OptionalResultAsync;

    macro_rules! native_exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                None => (None),
                Some(Ok($v)) => $ok,
                Some(Err(e)) => (Some(Err(e))),
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                None => (None),
                Some(Ok($v)) => (Some(Ok($ok))),
                Some(Err(e)) => (Some(Err(e))),
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                None => return (None),
                Some(Ok(ControlFlow::Break(b))) => return (Some(Ok(b))),
                Some(Ok(ControlFlow::Continue($v))) => $ok,
                Some(Err(e)) => return (Some(Err(e))),
            }
        };
        (@init $expr:expr) => {
            (Some(Ok($expr)))
        };
    }

    macro_rules! poll_exec {
        (@init $value:expr) => {
            Some(Ok($value))
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
                }
                Poll::Ready(Some(Err(e))) => {
                    $r;
                    return Poll::Ready(Some(Err(e)));
                },
                Poll::Ready(Some(Ok($output))) => $ok,
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
                Poll::Ready(Some(Err(e))) => {
                    $r;
                    return Poll::Ready(Some(Err(e)));
                },
                Poll::Ready(Some(Ok(ControlFlow::Break(value)))) => {
                    $r;
                    return Poll::Ready(Some(Ok(value)));
                }
                Poll::Ready(Some(Ok(ControlFlow::Continue($output)))) => $ok,
            }
        };
    }

    declare_async_all! {
        OptionalResultAsyncExt,
        <E> OptionalResultAsync
        Option<Result<Self::Output, E>>,
        Option<Result<T, E>>,
        async_option_result_,
        native_exec,
        poll_exec,
    }
}