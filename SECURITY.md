# Security policy

Colorless is pre-1.0 and includes carefully scoped unsafe runtime code used by
procedural macro expansion. Please report suspected memory-safety issues,
double drops, use-after-move behavior, or violations of the documented
`MaybeUnpinned` contract privately to the maintainers before opening a public
issue.

Semantic mismatches in relaxed-drop mode are also important reports, even when
they are memory safe. Include a minimal source program, the expanded code if
available, the Rust toolchain version, and whether the issue occurs with
`Identity`, `Async`, or a fallible color.

There is no stable security-support window before the first public alpha.
