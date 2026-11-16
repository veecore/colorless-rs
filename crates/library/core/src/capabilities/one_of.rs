macro_rules! declare_traits {
    (
        $trait_name:ident,
        $(
            $Tn:ident => $arm_n:ident
        ),+
    ) => {
        declare_traits! {
            @collected
            $trait_name,
            {generics: <$($Tn,)* Output>}
            {bounds: where $(
                $Tn: $crate::Colored<Self, Output = Output>
            ),*;}
            arms: $($Tn => $arm_n)+
        }
    };
    (
        @collected
        $trait_name:ident,
        {generics: $($generics:tt)*}
        {bounds: $($bounds:tt)*}
        arms: $($Tn:ident => $arm_n:ident)*
    ) => {
        /// # Heterogeneous Branching Without Heap Allocation
        ///
        /// In standard Rust, every arm of an `if/else` or `match` expression must evaluate to the
        /// exact same concrete type:
        ///
        /// ```rust,compile_fail
        /// fn branch(condition: bool) -> impl Future<Output = i32> {
        ///     if condition {
        ///         async { 10 } // Type A (anonymous future)
        ///     } else {
        ///         async { 20 } // Type B (distinct anonymous future)
        ///     }
        /// }
        /// ```
        ///
        /// In application code, developers usually resolve this by boxing: `Box::pin(async { ... })`.
        ///
        /// In a zero-cost library, however, heap-allocating on every branch or match arm defeats the primary
        /// purpose of the crate. A parser or protocol decoder may branch hundreds of times per message;
        /// forcing an allocator round-trip on every branch would make generic code significantly slower
        /// than handwritten synchronous code.
        ///
        /// ---
        ///
        /// ### The `OneOf` Unboxed Union
        ///
        /// `colorless` solves branching through a family of unboxed carrier traits: [`OneOf2`] through [`OneOf12`].
        ///
        /// Rather than imposing a single global enum that penalizes synchronous callers, each `Color` runtime
        /// implements `OneOf` in the manner most optimal for its execution model:
        ///
        /// - **In synchronous colors ([`Identity`](crate::Identity), [`Optional`](crate::Optional), [`Fallible`](crate::Fallible))**:
        ///   Synchronous branches do not need to exist in memory simultaneously. The active arm executes eagerly
        ///   and unwraps directly to its value. Branching introduces zero enum tags and zero memory footprint.
        /// - **In asynchronous colors ([`Async`](crate::Async), [`FallibleAsync`](crate::FallibleAsync))**:
        ///   Arms are unified into an unboxed enum (`OneOf2<A, B>`). The enum lives directly inside the parent
        ///   future's frame.
        ///
        /// ```rust,ignore
        /// let flow = match condition {
        ///     true  => <C as OneOf2>::arm_0(fetch_from_cache(key)),
        ///     false => <C as OneOf2>::arm_1(fetch_from_remote(key)),
        /// };
        /// ```
        ///
        /// Procedural macros accompanying this crate automatically desugar `match` and `if/else` expressions
        /// into the appropriate `OneOf` calls.
        pub trait $trait_name: Sized {
            /// The concrete unified flow outcome returned by all arm functions in this color.
            type Outcome$($generics)*: $crate::Colored<Self, Output = Output>
                $($bounds)*

            declare_traits! {
                @arm
                {generics: $($generics)*}
                {bounds: $($bounds)*}
                {arms: $($Tn => $arm_n)*}
            }
        }
    };
    (
        @arm
        {generics: $($generics:tt)*}
        {bounds: $($bounds:tt)*}
        {arms: $T:ident => $arm:ident $($Tn:ident => $arm_n:ident)*}
    ) => {
        /// Lifts a flow arm into the unified branch outcome.
        fn $arm$($generics)*(flow: $T) -> Self::Outcome$($generics)*
            $($bounds)*

        declare_traits! {
            @arm
            {generics: $($generics)*}
            {bounds: $($bounds)*}
            {arms: $($Tn => $arm_n)*}
        }
    };
    (
        @arm
        {generics: $($generics:tt)*}
        {bounds: $($bounds:tt)*}
        {arms: }
    ) => {}
}

declare_traits! {
    OneOf2,
    T0 => arm_0,
    T1 => arm_1
}

declare_traits! {
    OneOf3,
    T0 => arm_0,
    T1 => arm_1,
    T2 => arm_2
}

declare_traits! {
    OneOf4,
    T0 => arm_0,
    T1 => arm_1,
    T2 => arm_2,
    T3 => arm_3
}

declare_traits! {
    OneOf5,
    T0 => arm_0,
    T1 => arm_1,
    T2 => arm_2,
    T3 => arm_3,
    T4 => arm_4
}

declare_traits! {
    OneOf6,
    T0 => arm_0,
    T1 => arm_1,
    T2 => arm_2,
    T3 => arm_3,
    T4 => arm_4,
    T5 => arm_5
}

declare_traits! {
    OneOf7,
    T0 => arm_0,
    T1 => arm_1,
    T2 => arm_2,
    T3 => arm_3,
    T4 => arm_4,
    T5 => arm_5,
    T6 => arm_6
}

declare_traits! {
    OneOf8,
    T0 => arm_0,
    T1 => arm_1,
    T2 => arm_2,
    T3 => arm_3,
    T4 => arm_4,
    T5 => arm_5,
    T6 => arm_6,
    T7 => arm_7
}

declare_traits! {
    OneOf9,
    T0 => arm_0,
    T1 => arm_1,
    T2 => arm_2,
    T3 => arm_3,
    T4 => arm_4,
    T5 => arm_5,
    T6 => arm_6,
    T7 => arm_7,
    T8 => arm_8
}

declare_traits! {
    OneOf10,
    T0 => arm_0,
    T1 => arm_1,
    T2 => arm_2,
    T3 => arm_3,
    T4 => arm_4,
    T5 => arm_5,
    T6 => arm_6,
    T7 => arm_7,
    T8 => arm_8,
    T9 => arm_9
}

declare_traits! {
    OneOf11,
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
    T10 => arm_10
}

declare_traits! {
    OneOf12,
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
