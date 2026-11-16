# Safety model

This document is the audit boundary for the hidden local-pinning runtime in
`colorless-rt`. It is not a user-facing promise that arbitrary hand-written
uses of the hidden types are supported.

## `MaybeUnpinned<T>`

`MaybeUnpinned<T>` owns a `T` in `ManuallyDrop<T>` while a Colorless local
borrow stage is active. It has two terminal outcomes:

1. the stage is cancelled, panics, or completes without restoration; dropping
   `MaybeUnpinned<T>` drops `T` exactly once; or
2. generated code consumes `ErasedBorrow<MaybeUnpinned<T>>` through `unpin`,
   which marks the guard as transferred and returns the owned `T`. The caller
   then owns the only remaining drop obligation.

The guard must never be manually dropped after a successful `unpin`, and an
erased token must not be used after it is consumed by `unpin`.

## What inert validation proves

The macro emits an unreachable inert copy of the relevant source code. Rust's
normal borrow checker validates its references, moves, and ordinary source
lifetimes. That is essential evidence that the generated sequence cannot
create a reference the original source would reject.

It is *not* the complete unsafe proof. Code generation must additionally
guarantee all of the following:

- the `MaybeUnpinned<T>` owner is installed before the raw token is created;
- its physical storage stays at a stable address for every pointer-derived
  reference;
- each stage is evaluated sequentially and at most once;
- no pointer-derived reference is used after the token is consumed by `unpin`;
- every branch, cancellation path, and unwind path either transfers ownership
  exactly once or leaves the guard armed for ordinary drop.

The current compiler rejects or marks experimental shapes for which it cannot
yet maintain that argument.

## Test matrix

The runtime suite covers:

- normal guard drop without `unpin`;
- ownership transfer through `unpin`;
- mutable access before transfer;
- both conditional transfer paths;
- panic while the guard is armed;
- panic from `T::drop`; and
- dropping a pending `Async::borrow` computation before it completes.

The matrix does not make manual use of `ErasedBorrow` supported. It protects
the specific codegen protocol above.
