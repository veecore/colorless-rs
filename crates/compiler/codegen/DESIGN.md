# Code-generation design notes

`colorless-codegen` consumes the structured CPS IR emitted by
`colorless-lower` and produces calls to the runtime capabilities in
`colorless-core`.

## Destruction policy

The compiler currently supports one explicit policy:

```rust,ignore
#[codegen(drop_timing = "relaxed")]
```

The acknowledgment is mandatory. Generated code preserves ownership safety,
but it does not promise the source program's implicit destruction points or
implicit sibling destruction order. A value may be destroyed earlier or later
than the end of its source scope, and two otherwise-independent values may be
destroyed in a different order. Source code which observes destruction timing
must use explicit operations understood by Rust, such as `drop(value)`, rather
than relying on an implicit scope endpoint.

There is deliberately no lexical mode. The former runtime `Lazy` and
`LazyOneOfN` protocol was fragile, enlarged the runtime surface, and still hit
the rustc type-checking cliff on sufficiently nested or sequential generated
types. Thunks remain an internal compiler carrier, not a public scheduling
contract.

## Folded output shape

Every folded body has a flat sequence of statically known stages, ordered from
the expression currently emitted to its final value:

- `[]`: an ordinary Rust value;
- `[Colored]`: a `Colored` value;
- `[Colored, Deferred { .. }]`: a colored value producing a thunk;
- `[Deferred { .. }, Colored]`: a thunk producing a colored value;
- `[Colored, EndScope { .. }, Colored]`: a colored borrow stage producing a
  usage continuation whose result is colored.

`FoldedOutput` uses a `VecDeque<YieldPoint>`. `Colored` describes evaluation at
the current physical stage. `Deferred { until }` names the structural scope
allowed to invoke a thunk. `EndScope { end }` marks restoration of a locally
borrowed owner and application of the corresponding usage continuation.

Adjacent `Colored` points are an internal compiler error. They are not invalid
source syntax; codegen must squash them while composing `map` and `and_then`.

## Scope ownership

Lowering assigns a `CpsScopeId` to a continuation that must cross a source
scope. Codegen carries that target with the thunk. A structural owner invokes
only a deferred stage whose ID it owns; a foreign target remains in
`FoldedOutput` for an enclosing owner.

`try_resume_owned_yield` implements that local rule. `resume_all_yield` is the
full-reduction operation used when a construct intentionally consumes all
remaining stages. The latter must not be used as evidence that an arbitrary
structural node owns a continuation.

The original Rust braces are still emitted. They preserve ordinary name and
borrow scopes even though relaxed destruction timing means codegen does not
promise that every implicit drop remains at the same brace.

## Branch joins

Branch arms are folded independently. An arm with a direct result routes to
the final carrier immediately. An arm with a deferred result breaks to an
arm-specific label; the catch outside the source arm invokes the concrete
thunk and then routes the common final result through `OneOfN`.

There is no source-level arm cap. Codegen composes the available `OneOfN`
capabilities into a shallow tree when a source branch needs more carrier slots
than the largest directly supported runtime carrier. Zero slots are rejected
as an internal misuse, one slot is a pass-through, and plain arms are
consolidated before heterogeneous slots are counted.

This thunk route is a best-effort ownership mechanism under the relaxed
destruction policy. It is not a lexical destruction guarantee.

## Borrow usage

`resume_all_yield` can directly apply a known usage continuation to its
restored local, including multiple nested usage stages. That does not by itself
eliminate `BorrowUsage2` through `BorrowUsage12`.

If a branch chooses between heterogeneous usage-closure types before the local
is restored, the selected type must survive across the physical borrow stage.
The numbered `BorrowUsageN` enums are the current allocation-free sum for that
case. They may be removed only after an executable lowering/codegen proof shows
that every such branch can apply its usage before joining without changing
evaluation order or requiring the unavailable local.

The enums compose correctly across a small number of properly nested physical
borrow stages, including a colored stage between two restorations. They do not
scale safely as an unrestricted recursive program representation: a generated
eight-layer `BorrowUsage2` probe crossed 79 seconds on an incremental debug
build and was aborted. Codegen must therefore reject branch routing across
multiple outstanding borrow restorations for now, matching the conservative
legacy restriction. Nested borrows remain valid when no heterogeneous branch
carrier has to cross their restoration boundaries.

A colored operation between the two restoration points does not remove this
recursion. It merely changes the inner usage from `UsageB -> UsageA` to
`UsageB -> Colored<Output = UsageA>`; the outer usage type must still be named
inside every heterogeneous branch arm. Supporting that shape without the
bounded restriction would recreate the deferred-program experiment under
borrow-specific names.

The borrow folding rules are still incomplete. Unit tests of
`resume_all_yield` prove only its local stage algebra; they are not a claim that
the end-to-end borrow lowering is finished.

## Ownership and cloning

Compiler-side cloning of `syn` nodes may materialize a continuation that
lowering deliberately shared through `Rc<CpsBody>`. It must never emit a
source-level `.clone()` call or duplicate runtime ownership. Non-`Clone` drop
guards in the integration tests defend that boundary.

Root capture and internal callback ownership are separate decisions. A root
`Wrap` may borrow its surrounding environment, while a compiler-created CPS
callback owns the preceding stage's output.

## Executable evidence

- `tests/carrier_contract.rs` checks the runtime carrier shape for Identity and
  Async, including nested loops and non-cloneable state.
- `rules::yield_resume` unit tests check target-aware thunk and borrow-usage
  stage reduction.
- the core borrow tests check that a numbered usage carrier preserves
  heterogeneous continuation types for both Identity and Async.
- `branch_planning::sequential_branch_join_cliff_probe` remains feature-gated
  behind `compiler-cliff-probe` because it intentionally exercises an
  expensive generated type shape.
