# Publishing the workspace

Colorless is a workspace of published crates, not a single facade-only
package. `colorless` depends on `colorless-core`, `colorless-rt`,
`colorless-std`, `cps-signal`, `cps-try`, and the macro/compiler crates.
Cargo therefore cannot stage the facade on a machine where those versioned
dependencies have not first been published or supplied by a registry.

## Before the first publish

1. Set the canonical public repository URL in `workspace.package.repository`.
   Set `homepage` and `documentation` too if they differ. The audit copy does
   not guess these URLs because its source repository has no configured remote.
2. Confirm every package that needs to be public has a unique crates.io name
   and a release README suitable for that package's audience. The current root
   README is inherited deliberately until package-specific documentation is
   written.
3. Complete the release gate in [RELEASE.md](RELEASE.md), including Miri.
4. Stage/publish dependencies in topological order, using `cargo publish
   --dry-run` first. At the current dependency graph, the important ordering
   is:

   ```text
   cps-signal, cps-try, colorless-core
       -> colorless-rt
       -> colorless-lower
       -> colorless-codegen
       -> colorless-macros
       -> colorless-std
       -> colorless
   ```

   `dev-macros`, and `inference` should either be marked
   `publish = false` or given their own intentional public release plan; they
   are not facade runtime dependencies today.