# Changelog

All notable changes to Colorless are documented here.

This project follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and will use Semantic Versioning once the macro's accepted source-language
subset and destruction semantics are stable.

## Unreleased

### Added

- Release-oriented safety coverage for `MaybeUnpinned` ownership transfer and
  cancellation paths.
- Macro UI compile-pass and compile-fail coverage.
- A documented alpha release contract and CI release gate.

### Changed

- The public documentation now states the compiler's relaxed implicit
  destruction semantics directly.

### Known limitations

- Exact lexical destruction timing and sibling destruction order are not
  preserved by macro-generated continuations.
- Local pinning, automatic borrow preparation, and several complex control-flow
  combinations remain experimental.
