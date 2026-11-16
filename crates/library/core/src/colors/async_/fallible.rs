use core::marker::PhantomData;

/// The async version of [Fallible](crate::Fallible)
#[doc(alias = "AsyncResult")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FallibleAsync<E>(pub(crate) PhantomData<E>);

pub use impls::FallibleAsyncExt;

mod impls {
    use crate::FallibleAsync;

    macro_rules! native_exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                Err(err) => Err(err),
                Ok($v) => $ok,
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                Err(err) => Err(err),
                Ok($v) => Ok($ok),
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                Ok(ControlFlow::Break(b)) => return Ok(b),
                Ok(ControlFlow::Continue($v)) => $ok,
                Err(e) => return Err(e),
            }
        };
        (@init $expr:expr) => {
            Ok($expr)
        };
    }

    macro_rules! poll_exec {
        (@init $value:expr) => {
            Ok($value)
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
                Poll::Ready(Ok($output)) => $ok,
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
                Poll::Ready(Ok(ControlFlow::Break(value))) => {
                    $r;
                    return Poll::Ready(Ok(value));
                }
                Poll::Ready(Ok(ControlFlow::Continue($output))) => $ok,
            }
        };
    }
    
    declare_async_all! {
        FallibleAsyncExt,
        <E> FallibleAsync
        Result<Self::Output, E>,
        Result<T, E>,
        async_result_,
        native_exec,
        poll_exec,
    }
}