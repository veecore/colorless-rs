// @init
macro_rules! declare_wrappers {
    (
        $exec:ident,
        <$($g:ident)?> $color:ident
    ) => {
        /// A ready value wrapper for the asynchronous context.
        #[repr(transparent)]
        struct ReadyValue<T $(,$g)?>(pub ManuallyDrop<T> $(, pub PhantomData<$g>)?);

        /// Compact deferred closure used by Async wrap and lazy operations.
        #[repr(transparent)]
        struct AsyncWrap<F: ThunkOnce $(,$g)?>(ManuallyDrop<F>$(, pub PhantomData<$g>)?);

        /// Compact state produced by [`Cps::map`] in the asynchronous color.
        ///
        /// This is explicit rather than an `async` block so the source flow is stored
        /// once. A nested async adapter can otherwise make a composed flow's layout
        /// approximately double at every step.
        struct AsyncMap<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
            F: CallOnce<C::Output>,
        {
            flow: ManuallyDrop<C>,
            map: ManuallyDrop<F>,
            $(_marker: PhantomData<$g>)?
        }

        enum AsyncWrapFlatten<F $(,$g)?>
        where
            F: ColoredThunkOnce<$color$(<$g>)?>,
        {
            Unresumed {
                f: ManuallyDrop<F>,
            },
            Flow(F::Flow),
        }

        enum AsyncAndThen<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
            F: ColoredCallOnce<$color$(<$g>)?, C::Output>,
        {
            First {
                flow: ManuallyDrop<C>,
                map: ManuallyDrop<F>,
            },
            Second(F::Flow),
        }

        /// Manual async loop state machine for [`Loop::fold_while`].
        struct AsyncFold<S, F $(,$g)?>
        where
            F: ColoredCallFold<$color$(<$g>)?, S>,
        {
            f: ManuallyDrop<F>,
            phase: LoopFoldPhase<S, F::Flow>,
        }

        enum LoopFoldPhase<S, F> {
            Unresumed(ManuallyDrop<S>),
            Deciding(ManuallyDrop<F>),
        }

        /// Manual async borrow state machine for [`LocalBorrow::borrow_flat`].
        ///
        /// Unlike the `async move` path used by [`LocalBorrow::borrow`], this state machine
        /// overlays the scope and the running flow in the same union-sized enum slot, so
        /// stacked borrows grow O(N) instead of O(N²).
        struct AsyncBorrow<L, S $(,$g)?>
        where
            S: FlatBorrowScope<$color$(<$g>)?, L>,
        {
            local: ManuallyDrop<core::cell::UnsafeCell<L>>,
            phase: BorrowPhase<S, S::Flow>,
        }

        enum BorrowPhase<S, F> {
            NotStarted(ManuallyDrop<S>),
            Running(ManuallyDrop<F>),
        }

        // it seems better to stay on colored's side
        #[cfg(feature = "alloc")]
        pub struct AsyncErased<'a, Output $(,$g)?>(::alloc::boxed::Box<dyn Colored<$color$(<$g>)?, Output = Output, OutputInternal = Output> + 'a>);

        // ── ReadyValue Implementation ────────────────────────────────────────────────
        unsafe impl<T $(,$g)?> ColoredInternal<$color$(<$g>)?> for ReadyValue<T $(,$g)?> {
            type OutputInternal = T;

            #[inline(always)]
            unsafe fn poll_colored(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<<$color$(<$g>)? as Present>::Value<T>> {
                // SAFETY: the safety contract of this function is that the caller tracks completion.
                // We need to drop in-case we were not polled at all
                let value = unsafe { ManuallyDrop::take(&mut self.0) };
                Poll::Ready($exec!(@init value))
            }

            #[inline(always)]
            unsafe fn finalize_ready(self: Pin<&mut Self>) {
                // already used
            }
        }

        impl<T $(,$g)?> Drop for ReadyValue<T $(,$g)?> {
            fn drop(&mut self) {
                // We were never polled
                unsafe {
                    ManuallyDrop::drop(&mut self.0);
                }
            }
        }

        impl<T $(,$g)?> Colored<$color$(<$g>)?> for ReadyValue<T $(,$g)?> {
            type Output = T;
        }

        impl<T $(,$g)?> Unpin for ReadyValue<T $(,$g)?> {}

        // ── AsyncWrap Implementation ───────────────────────────────────────────────────

        unsafe impl<F $(,$g)?> ColoredInternal<$color$(<$g>)?> for AsyncWrap<F $(,$g)?>
        where
            F: ThunkOnce,
        {
            type OutputInternal = F::Value;

            #[inline(always)]
            unsafe fn poll_colored(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<<$color$(<$g>)? as Present>::Value<F::Value>> {
                let thunk = unsafe { ManuallyDrop::take(&mut self.0) };
                Poll::Ready($exec!(@init thunk()))
            }

            #[inline(always)]
            unsafe fn finalize_ready(self: Pin<&mut Self>) {
                // already used and dropped
            }
        }

        impl<F $(,$g)?> Drop for AsyncWrap<F $(,$g)?>
        where
            F: ThunkOnce,
        {
            fn drop(&mut self) {
                // We were never polled
                unsafe {
                    ManuallyDrop::drop(&mut self.0);
                }
            }
        }

        impl<F $(,$g)?> Colored<$color$(<$g>)?> for AsyncWrap<F $(,$g)?>
        where
            F: ThunkOnce,
        {
            type Output = F::Value;
        }

        impl<F $(,$g)?> Unpin for AsyncWrap<F $(,$g)?> where F: ThunkOnce {}

        // ── AsyncMap Implementation ───────────────────────────────────────────────────

        unsafe impl<C, F $(,$g)?> ColoredInternal<$color$(<$g>)?> for AsyncMap<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
            F: CallOnce<C::Output>,
        {
            type OutputInternal = F::Value;

            #[inline(always)]
            unsafe fn poll_colored(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>
            ) -> Poll<<$color$(<$g>)? as Present>::Value<F::Value>> {
                let this = unsafe { self.get_unchecked_mut() };

                let _map_unwind_guard = UnwindGuard(&mut *this.map);

                // Poll flow in-place
                macro_rules! on_ready {
                    ($map:ident) => {
                        unsafe { Pin::new_unchecked(&mut *this.flow).finalize_ready() };

                        core::mem::forget(_map_unwind_guard);
                        let $map = unsafe { ManuallyDrop::take(&mut this.map) };
                    };
                }

                $exec! {
                    unsafe { Pin::new_unchecked(&mut *this.flow).poll_colored(cx) };

                    on_pending: {
                        core::mem::forget(_map_unwind_guard);
                        Poll::Pending
                    }

                    on_ready: {
                        on_ready!(map);
                        drop(map)
                    }

                    output on_ready_ok: {
                        on_ready!(map);
                        Poll::Ready($exec!(@init map(output)))
                    }
                }
            }

            #[inline(always)]
            unsafe fn finalize_ready(self: Pin<&mut Self>) {
                // map is already used and flow dropped on completion
            }
        }

        impl<C, F $(,$g)?> Drop for AsyncMap<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
            F: CallOnce<C::Output>,
        {
            fn drop(&mut self) {
                unsafe {
                    // Must happen before dropping flow.
                    let map = ManuallyDrop::take(&mut self.map);

                    // On panic, stack unwinding drops `map`.
                    ManuallyDrop::drop(&mut self.flow);

                    // On success, explicitly finish the cleanup.
                    drop(map);
                }
            }
        }

        impl<C, F $(,$g)?> Colored<$color$(<$g>)?> for AsyncMap<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
            F: CallOnce<C::Output>,
        {
            type Output = F::Value;
        }

        impl<C, F $(,$g)?> Unpin for AsyncMap<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?> + Unpin,
            F: CallOnce<C::Output>,
        {
        }

        // ── AsyncWrapFlatten Implementation ───────────────────────────────────────────────────

        unsafe impl<F $(,$g)?> ColoredInternal<$color$(<$g>)?> for AsyncWrapFlatten<F $(,$g)?>
        where
            F: ColoredThunkOnce<$color$(<$g>)?>,
        {
            type OutputInternal = F::Value;

            #[inline(always)]
            unsafe fn poll_colored(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>
            ) -> Poll<<$color$(<$g>)? as Present>::Value<F::Value>> {
                loop {
                    let this = unsafe { self.as_mut().get_unchecked_mut() };
                    match this {
                        AsyncWrapFlatten::Unresumed { f } => {
                            let thunk = unsafe { ManuallyDrop::take(f) };
                            unsafe { core::ptr::write(this, AsyncWrapFlatten::Flow(thunk())) };
                        }
                        AsyncWrapFlatten::Flow(flow) => {
                            return unsafe { Pin::new_unchecked(flow).poll_colored(cx) };
                        }
                    }
                }
            }

            #[inline(always)]
            unsafe fn finalize_ready(mut self: Pin<&mut Self>) {
                let me = unsafe { self.as_mut().get_unchecked_mut() };
                match me {
                    AsyncWrapFlatten::Flow(flow) => {
                        unsafe { Pin::new_unchecked(flow).finalize_ready() };
                    }
                    _ => unsafe { unreachable_unchecked() },
                }
            }
        }

        impl<F $(,$g)?> Drop for AsyncWrapFlatten<F $(,$g)?>
        where
            F: ColoredThunkOnce<$color$(<$g>)?>,
        {
            fn drop(&mut self) {
                if let &mut AsyncWrapFlatten::Unresumed { ref mut f } = self {
                    // We were never polled
                    unsafe {
                        ManuallyDrop::drop(f);
                    }
                }
            }
        }

        impl<F $(,$g)?> Colored<$color$(<$g>)?> for AsyncWrapFlatten<F $(,$g)?>
        where
            F: ColoredThunkOnce<$color$(<$g>)?>,
        {
            type Output = F::Value;
        }

        impl<F $(,$g)?> Unpin for AsyncWrapFlatten<F $(,$g)?>
        where
            F: ColoredThunkOnce<$color$(<$g>)?>,
            F::Flow: Unpin
        {
        }

        // ── AsyncAndThen Implementation ───────────────────────────────────────────────────

        unsafe impl<C, F $(,$g)?> ColoredInternal<$color$(<$g>)?> for AsyncAndThen<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
            F: ColoredCallOnce<$color$(<$g>)?, C::Output>,
        {
            type OutputInternal = F::Value;

            #[inline(always)]
            unsafe fn poll_colored(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>
            ) -> Poll<<$color$(<$g>)? as Present>::Value<F::Value>> {
                loop {
                    let this = unsafe { self.as_mut().get_unchecked_mut() };
                    match this {
                        AsyncAndThen::First { flow, map } => {
                            let _map_unwind_guard = UnwindGuard(map.deref_mut());

                            macro_rules! on_ready {
                                ($map:ident) => {
                                    unsafe { Pin::new_unchecked(flow.deref_mut()).finalize_ready() };

                                    core::mem::forget(_map_unwind_guard);

                                    let $map = unsafe { ManuallyDrop::take(map) };
                                };
                            }

                            $exec! {
                                unsafe { Pin::new_unchecked(flow.deref_mut()).poll_colored(cx) };

                                on_pending: {
                                    core::mem::forget(_map_unwind_guard);
                                    return Poll::Pending;
                                }

                                on_ready: {
                                    on_ready!(map);
                                    drop(map)
                                }

                                output on_ready_ok: {
                                    on_ready!(map);

                                    unsafe { core::ptr::write(this, AsyncAndThen::Second(map(output))) }
                                }
                            }
                        }
                        AsyncAndThen::Second(second) => {
                            return unsafe { Pin::new_unchecked(second).poll_colored(cx) };
                        }
                    }
                }
            }

            #[inline(always)]
            unsafe fn finalize_ready(mut self: Pin<&mut Self>) {
                let me = unsafe { self.as_mut().get_unchecked_mut() };
                match me {
                    AsyncAndThen::Second(flow) => {
                        unsafe { Pin::new_unchecked(flow).finalize_ready() };
                    }
                    _ => unsafe { unreachable_unchecked() },
                }
            }
        }

        impl<C, F $(,$g)?> Drop for AsyncAndThen<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
            F: ColoredCallOnce<$color$(<$g>)?, C::Output>,
        {
            fn drop(&mut self) {
                unsafe {
                    if let AsyncAndThen::First { flow, map } = self {
                        // Must happen before dropping flow.
                        let map = ManuallyDrop::take(map);

                        // On panic, stack unwinding drops `map`.
                        ManuallyDrop::drop(flow);

                        // On success, explicitly finish the cleanup.
                        drop(map);
                    }
                }
            }
        }

        impl<C, F $(,$g)?> Colored<$color$(<$g>)?> for AsyncAndThen<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?>,
            F: ColoredCallOnce<$color$(<$g>)?, C::Output>,
        {
            type Output = F::Value;
        }

        impl<C, F $(,$g)?> Unpin for AsyncAndThen<C, F $(,$g)?>
        where
            C: Colored<$color$(<$g>)?> + Unpin,
            F: ColoredCallOnce<$color$(<$g>)?, C::Output>,
            F::Flow: Unpin
        {
        }

        // ── AsyncFold Implementation ───────────────────────────────────────────────────

        unsafe impl<S, F $(,$g)?> ColoredInternal<$color$(<$g>)?> for AsyncFold<S, F $(,$g)?>
        where
            F: ColoredCallFold<$color$(<$g>)?, S>,
        {
            type OutputInternal = F::Break;

            #[inline(always)]
            unsafe fn poll_colored(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<<$color$(<$g>)? as Present>::Value<F::Break>> {
                let this = unsafe { self.get_unchecked_mut() };

                loop {
                    let state = match &mut this.phase {
                        LoopFoldPhase::Deciding(flow) => {
                            let _f_unwind_guard = UnwindGuard(&raw mut *this.f);

                            macro_rules! on_ready {
                                () => {
                                     // If this panics, `f` and `state` are dropped by unwinding.
                                    unsafe {
                                        Pin::new_unchecked(flow.deref_mut()).finalize_ready();
                                    }
                                };
                            }

                            $exec! {
                                @loop

                                unsafe { Pin::new_unchecked(flow.deref_mut()).poll_colored(cx) };

                                on_pending: {
                                    core::mem::forget(_f_unwind_guard);
                                    return Poll::Pending;
                                }

                                on_ready: {
                                    on_ready!();
                                    drop(_f_unwind_guard);
                                }

                                state on_ready_ok: {
                                    on_ready!();
                                    core::mem::forget(_f_unwind_guard);

                                    state
                                }
                            }
                        }
                        LoopFoldPhase::Unresumed(state) => {
                            // saw 8% performance with ref
                            core::hint::cold_path();

                            unsafe { ManuallyDrop::take(state) }
                        }
                    };

                    let mut map = unsafe { ManuallyDrop::take(&mut this.f) };
                    let flow = map(state);
                    this.f = ManuallyDrop::new(map);
                    this.phase = LoopFoldPhase::Deciding(ManuallyDrop::new(flow));
                }
            }

            #[inline(always)]
            unsafe fn finalize_ready(self: Pin<&mut Self>) {
                // we already dropped on complete
            }
        }

        impl<S, F $(,$g)?> Drop for AsyncFold<S, F $(,$g)?>
        where
            F: ColoredCallFold<$color$(<$g>)?, S>,
        {
            fn drop(&mut self) {
                unsafe {
                    // Protect `f` before dropping the potentially panicking phase.
                    let f = ManuallyDrop::take(&mut self.f);

                    match &mut self.phase {
                        LoopFoldPhase::Unresumed(state) => {
                            ManuallyDrop::drop(state);
                        }
                        LoopFoldPhase::Deciding(flow) => {
                            ManuallyDrop::drop(flow);
                        }
                    }

                    drop(f);
                }
            }
        }

        impl<S, F $(,$g)?> Colored<$color$(<$g>)?> for AsyncFold<S, F $(,$g)?>
        where
            F: ColoredCallFold<$color$(<$g>)?, S>,
        {
            type Output = F::Break;
        }

        // ── AsyncBorrow Implementation ─────────────────────────────────────────────────

        unsafe impl<L, S $(,$g)?> ColoredInternal<$color$(<$g>)?> for AsyncBorrow<L, S $(,$g)?>
        where
            S: FlatBorrowScope<$color$(<$g>)?, L>,
        {
            type OutputInternal = S::Resume;

            unsafe fn poll_colored(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<<$color$(<$g>)? as Present>::Value<S::Resume>> {
                let this = unsafe { self.get_unchecked_mut() };
                loop {
                    match &mut this.phase {
                        BorrowPhase::NotStarted(scope) => {
                            core::hint::cold_path();
                            let s = unsafe { ManuallyDrop::take(scope) };
                            // SAFETY: `this` is pinned; `this.local` is never moved.
                            // ErasedBorrow erases the lifetime via NonNull, so S::Flow
                            // carries no borrow of `this.local`.
                            let ptr: *mut L = this.local.get();
                            let _local_guard = UnwindGuard(ptr);
                            let flow = s.resume_mut_flat(unsafe { &mut *ptr });
                            core::mem::forget(_local_guard);
                            this.phase = BorrowPhase::Running(ManuallyDrop::new(flow));
                        }
                        BorrowPhase::Running(flow) => {
                            let ptr: *mut L = this.local.get();
                            let _local_guard = UnwindGuard(ptr);
                            let poll = unsafe {
                                Pin::new_unchecked(flow.deref_mut()).poll_colored(cx)
                            };
                            core::mem::forget(_local_guard);
                            return poll;
                        }
                    }
                }
            }

            unsafe fn finalize_ready(self: Pin<&mut Self>) {
                let this = unsafe { self.get_unchecked_mut() };
                match &mut this.phase {
                    BorrowPhase::Running(flow) => {
                        let ptr: *mut L = this.local.get();
                        let _local_guard = UnwindGuard(ptr);
                        unsafe { Pin::new_unchecked(flow.deref_mut()).finalize_ready() };
                        drop(_local_guard);
                    }
                    _ => unsafe { unreachable_unchecked() },
                }
            }
        }

        impl<L, S $(,$g)?> Drop for AsyncBorrow<L, S $(,$g)?>
        where
            S: FlatBorrowScope<$color$(<$g>)?, L>,
        {
            fn drop(&mut self) {
                unsafe {
                    let ptr: *mut L = self.local.get();
                    let _local_guard = UnwindGuard(ptr);
                    match &mut self.phase {
                        BorrowPhase::NotStarted(scope) => {
                            ManuallyDrop::drop(scope);
                        }
                        BorrowPhase::Running(flow) => {
                            ManuallyDrop::drop(flow);
                        }
                    }
                    drop(_local_guard);
                }
            }
        }

        impl<L, S $(,$g)?> Colored<$color$(<$g>)?> for AsyncBorrow<L, S $(,$g)?>
        where
            S: FlatBorrowScope<$color$(<$g>)?, L>,
        {
            type Output = S::Resume;
        }

        unsafe impl<L: Send, S: Send $(,$g)?> Sync for AsyncBorrow<L, S $(,$g)?>
        where
            S: FlatBorrowScope<$color$(<$g>)?, L>,
        {}

        // ── AsyncErased Implementation ───────────────────────────────────────────────────

        #[cfg(feature = "alloc")]
        unsafe impl<'a, Output $(,$g)?> ColoredInternal<$color$(<$g>)?> for AsyncErased<'a, Output $(,$g)?> {
            type OutputInternal = Output;

            #[inline(always)]
            unsafe fn poll_colored(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<<$color$(<$g>)? as Present>::Value<Output>> {
                unsafe {Pin::map_unchecked_mut(self, |this| this.0.deref_mut()).poll_colored(cx)}
            }

            #[inline(always)]
            unsafe fn finalize_ready(self: Pin<&mut Self>) {
                unsafe {Pin::map_unchecked_mut(self, |this| this.0.deref_mut()).finalize_ready()}
            }
        }

        #[cfg(feature = "alloc")]
        impl<Output $(,$g)?> Colored<$color$(<$g>)?> for AsyncErased<'_, Output $(,$g)?>
        {
            type Output = Output;
        }

        use $crate::colors::async_::helper::*;

        impl_one_of_all! {
            <$($g)?> $color,

            OneOf2, OneOf2Outcome;
            T0 => arm_0,
            T1 => arm_1;

            OneOf3, OneOf3Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2;

            OneOf4, OneOf4Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3;

            OneOf5, OneOf5Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4;

            OneOf6, OneOf6Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5;

            OneOf7, OneOf7Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6;

            OneOf8, OneOf8Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7;

            OneOf9, OneOf9Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7,
            T8 => arm_8;

            OneOf10, OneOf10Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7,
            T8 => arm_8,
            T9 => arm_9;

            OneOf11, OneOf11Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7,
            T8 => arm_8,
            T9 => arm_9,
            T10 => arm_10;

            OneOf12, OneOf12Outcome;
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7,
            T8 => arm_8,
            T9 => arm_9,
            T10 => arm_10,
            T11 => arm_11
        }
    };
}
