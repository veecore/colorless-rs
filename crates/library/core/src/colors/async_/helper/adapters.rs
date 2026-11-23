macro_rules! declare_adapters {
    (
        <$($g:ident)?> $color:ident
        $native:ty
    ) => {
        /// An adapter turning any [`Colored<Async>`] into a standard [`Future`].
        pub(crate) enum MakeFuture<C $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
        {
            Running {
                inner: ManuallyDrop<C>,
                _marker: PhantomPinned,
                $(
                    _marker2: PhantomData<$g>,
                )?
            },
            Terminated,
        }

        impl<C $(,$g)?> MakeFuture<C $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
        {
            #[inline(always)]
            pub(crate) fn new(inner: C) -> Self {
                Self::Running {
                    inner: ManuallyDrop::new(inner),
                    _marker: PhantomPinned,
                    $(
                        _marker2: PhantomData::<$g>,
                    )?
                }
            }
        }

        /// Marks a [`MakeFuture`] terminated if polling or ready-finalization unwinds.
        ///
        /// The active colored computation owns its own unwind cleanup. This guard only
        /// updates the outer, ordinary-`Future` state so its `Drop` implementation does
        /// not enter an already destroyed computation.
        struct MarkTerminatedOnDrop<C: Colored<$color$(<$g>)?> $(,$g)?>(*mut MakeFuture<C $(,$g)?>);

        impl<C  $(,$g)?> Drop for MarkTerminatedOnDrop<C $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
        {
            #[inline(always)]
            fn drop(&mut self) {
                // SAFETY: the guard is created from the uniquely borrowed, pinned
                // `MakeFuture` being polled. A live guard is only dropped after the
                // contained computation has completed or has cleaned itself on unwind.
                // Writing `Terminated` does not read or drop the logically dead payload.
                unsafe { write(self.0, MakeFuture::Terminated) }
            }
        }

        #[repr(transparent)]
        pub(crate) struct MakeColored<F: Future $(,$g)?>(pub F $(, pub PhantomData<$g>)?);

        impl<T, C $(,$g)?> Future for MakeFuture<C $(,$g)?>
        where
            C: Colored<$color$(<$g>)?, Output = T>,
        {
            type Output = $native;

            #[inline]
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<$native> {
                // SAFETY: no variant is moved. `Running::inner` remains at the same
                // address for the duration of its pin, and transition to `Terminated`
                // happens only after the inner value is logically dead.
                let me = unsafe { self.as_mut().get_unchecked_mut() };
                let me_ptr = me as *mut Self;
                let inner = match me {
                    MakeFuture::Running { inner, .. } => inner,
                    MakeFuture::Terminated => {
                        panic!(concat!("impl Colored<", stringify!($color$(<$g>)?), "> polled after ready/panic"))
                    }
                };

                let terminated = MarkTerminatedOnDrop(me_ptr);
                match unsafe { Pin::new_unchecked(&mut **inner).poll_colored(cx) } {
                    Poll::Pending => {
                        // `inner` is still live; leave the enum in `Running`.
                        core::mem::forget(terminated);
                        Poll::Pending
                    }
                    Poll::Ready(val) => {
                        unsafe { Pin::new_unchecked(&mut **inner).finalize_ready() };
                        // The payload is now logically dead. The guard changes the
                        // discriminant without trying to drop that payload again.
                        drop(terminated);
                        Poll::Ready(val)
                    }
                }
            }
        }

        impl<C $(,$g)?> Drop for MakeFuture<C $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
        {
            fn drop(&mut self) {
                if let MakeFuture::Running { inner, .. } = self {
                    // A running computation is valid for ordinary cancellation.
                    unsafe { ManuallyDrop::drop(inner) }
                }
            }
        }

        // ── Wrapped Implementation ───────────────────────────────────────────────────

        unsafe impl<T, F $(,$g)?> ColoredInternal<$color$(<$g>)?> for MakeColored<F $(,$g)?>
        where
            F: Future<Output = $native>,
        {
            type OutputInternal = T;

            #[inline(always)]
            unsafe fn poll_colored(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<<$color$(<$g>)? as Present>::Value<T>> {
                // SAFETY: `Wrapped` is #[repr(transparent)] around `F`.
                let f = unsafe { &raw mut self.get_unchecked_mut().0 };

                let _unwind_guard = UnwindGuard(f);
                let poll = unsafe { Pin::new_unchecked(&mut *f).poll(cx) };
                core::mem::forget(_unwind_guard);
                poll
            }

            #[inline(always)]
            unsafe fn finalize_ready(self: Pin<&mut Self>) {
                // A nested `MakeFuture` has already changed itself to `Terminated`, so
                // dropping it here is a no-op. An arbitrary future is likewise valid to
                // drop after returning `Ready`.
                let inner: *mut F = unsafe { &mut self.get_unchecked_mut().0 };
                unsafe { drop_in_place(inner) }
            }
        }

        // the compiler drop is sufficient

        impl<T, F $(,$g)?> Colored<$color$(<$g>)?> for MakeColored<F $(,$g)?>
        where
            F: Future<Output = $native>,
        {
            type Output = T;
        }

        impl<F $(,$g)?> Unpin for MakeColored<F $(,$g)?> where F: Future + Unpin {}


        #[repr(transparent)]
        pub(crate) struct UnwindGuard<F>(pub(crate) *mut F);

        impl<F> Drop for UnwindGuard<F> {
            fn drop(&mut self) {
                unsafe { drop_in_place(self.0) }
            }
        }
    };
}
