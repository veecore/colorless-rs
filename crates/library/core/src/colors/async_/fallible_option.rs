use core::marker::PhantomData;

/// The async version of [FallibleOption](crate::FallibleOption)
#[doc(alias = "AsyncResultOption")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FallibleOptionAsync<E>(pub(crate) PhantomData<E>);

pub use impls::FallibleOptionAsyncExt;

mod impls {
    use crate::FallibleOptionAsync;

    macro_rules! native_exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                Ok(None) => Ok(None),
                Ok(Some($v)) => $ok,
                Err(e) => Err(e),
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                Ok(None) => Ok(None),
                Ok(Some($v)) => (Ok(Some($ok))),
                Err(e) => (Err(e)),
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                Ok(None) => return (Ok(None)),
                Ok(Some(ControlFlow::Break(b))) => return (Ok(Some(b))),
                Ok(Some(ControlFlow::Continue($v))) => $ok,
                Err(e) => return (Err(e)),
            }
        };
        (@init $expr:expr) => {
            (Ok(Some($expr)))
        };
    }

    macro_rules! poll_exec {
        (@init $value:expr) => {
            Ok(Some($value))
        };
        (
            $t:expr;

            on_pending: $p:block

            on_ready: $r:block

            $output:ident on_ready_ok: $ok:block
        ) => {
            match $t {
                Poll::Pending => $p,
                Poll::Ready(Err(e)) => {
                    $r;
                    return Poll::Ready(Err(e));
                }
                Poll::Ready(Ok(None)) => {
                    $r;
                    return Poll::Ready(Ok(None));
                },
                Poll::Ready(Ok(Some($output))) => $ok,
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
                Poll::Ready(Err(e)) => {
                    $r;
                    return Poll::Ready(Err(e));
                }
                Poll::Ready(Ok(None)) => {
                    $r;
                    return Poll::Ready(Ok(None));
                },
                Poll::Ready(Ok(Some(ControlFlow::Break(value)))) => {
                    $r;
                    return Poll::Ready(Ok(Some(value)));
                }
                Poll::Ready(Ok(Some(ControlFlow::Continue($output)))) => $ok,
            }
        };
    }

    declare_async_all! {
        FallibleOptionAsyncExt,
        <E> FallibleOptionAsync
        Result<Option<Self::Output>, E>,
        Result<Option<T>, E>,
        async_result_option_,
        native_exec,
        poll_exec,
    }
}