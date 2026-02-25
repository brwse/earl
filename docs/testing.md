# Testing

This document describes how to write and run tests for Earl.

## Practices

**Test behaviour, not implementation.** A test that breaks when you rename a private function is not testing behaviour — it is testing structure. Tests should survive refactors. If changing how something works internally requires rewriting tests, the tests were coupled to the wrong thing.

**Optimise for failure clarity, not coverage.** A test that fails with a clear message pointing to the exact broken assumption is worth more than ten tests that pad a coverage number. Coverage tells you which lines were executed; it says nothing about whether the right things were asserted. Chasing 100% coverage produces tests that call code without checking its results.

**Every test should be able to fail.** Before committing a test, verify it catches a real defect by temporarily breaking the code it covers. A test that always passes regardless of the code is worse than no test — it creates false confidence.

**Test the unhappy path.** Most bugs live in error handling, edge cases, and boundary conditions, not in the main success path. Prioritise tests for malformed input, missing values, empty collections, and off-by-one conditions.

**One concept per test.** A test with five assertions is five tests that share a name. When it fails you must read the whole test to find out which assertion fired. Split it up. The overhead is worth the precision.

**Avoid logic in tests.** Loops, conditionals, and helper functions inside a test body make the test itself a candidate for bugs. Keep test bodies flat and obvious. If setup is complex, extract it into a named fixture function — but keep assertions in the test.

**Prefer real dependencies over mocks.** Mocks that mirror the interface of a real dependency can silently diverge from it. Use real implementations where the cost (speed, flakiness, setup) is acceptable. Reserve mocks for I/O that is genuinely hard to control in tests (time, randomness, external network). When a real dependency can run in a container, use [testcontainers](https://rust.testcontainers.org/) to spin it up in-process rather than marking the test `#[ignore]` and relying on a manually provisioned service.

**Do not test the framework.** Earl uses `hcl-rs`, `minijinja`, `tokio`, and other libraries. Tests should not verify that these libraries behave correctly — they have their own test suites. Test Earl's logic built on top of them.

**Failing tests are high priority.** A flaky or ignored test is noise that erodes trust in the suite. Fix or delete it. A test suite that developers distrust is not a safety net.

## Running Tests

Run the full test suite:

```sh
cargo test
```

Run tests for a specific crate or module:

```sh
cargo test -p earl-core
cargo test templates::
```

Run a single test by name:

```sh
cargo test test_http_get
```

Run tests with output visible (useful when a test panics):

```sh
cargo test -- --nocapture
```

Some integration tests are marked `#[ignore]` because they require external services. Run them explicitly:

```sh
cargo test -- --ignored
```

## Test Organization

Tests live in three places:

**Unit tests** sit in the same file as the code they test, inside a `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bearer_token() {
        // ...
    }
}
```

**Integration tests** live under `tests/` at the crate root. Each file is compiled as a separate crate, so only the public API is accessible:

```
crate/
  src/
  tests/
    http.rs
    graphql.rs
```

**Doc tests** appear in `///` doc comments and serve as both documentation and tests:

```rust
/// Encodes a value as base64.
///
/// ```
/// assert_eq!(encode("hello"), "aGVsbG8=");
/// ```
pub fn encode(s: &str) -> String { ... }
```

## Writing Unit Tests

Keep unit tests close to the code. Test one behaviour per test. Name tests after the condition and expected outcome, not the function:

```rust
#[test]
fn missing_required_arg_returns_error() { ... }

#[test]
fn optional_arg_defaults_to_empty_string() { ... }
```

Use `assert_eq!` for values and `assert!(matches!(...))` for enum variants. Prefer `unwrap()` over `?` in tests — panics give clearer failure messages than propagated errors.

For error cases, match the variant rather than the message string so tests do not break when wording changes:

```rust
let err = parse("bad input").unwrap_err();
assert!(matches!(err, Error::InvalidSyntax { .. }));
```

## Writing Integration Tests

Integration tests exercise end-to-end behaviour through the public API. Avoid mocking internal details — test real code paths.

Structure each test file around a single concern (one protocol, one command). Use a helper function or fixture to build common inputs rather than repeating setup across tests.

For tests that require a live external service, annotate them with `#[ignore]`:

```rust
#[test]
#[ignore = "requires running Postgres"]
fn executes_sql_query() { ... }
```

Document in a comment what the test needs and how to satisfy it before running.

## Testing Templates

Template tests should cover:

- **Parsing**: a well-formed HCL template deserialises without error.
- **Rendering**: given a set of arguments, the rendered output matches the expected value.
- **Validation**: a template with a missing required field or wrong type is rejected with a clear error.

Fixtures belong in `tests/fixtures/` as `.hcl` files. Load them with `include_str!` or read them at runtime using a path relative to `CARGO_MANIFEST_DIR`:

```rust
let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/simple_get.hcl");
let source = std::fs::read_to_string(path).unwrap();
```

## Testing CLI Behaviour

Use `assert_cmd` to drive the binary as a subprocess and assert on exit code, stdout, and stderr:

```rust
use assert_cmd::Command;

#[test]
fn unknown_command_exits_nonzero() {
    Command::cargo_bin("earl")
        .unwrap()
        .args(["call", "nonexistent.command"])
        .assert()
        .failure();
}
```

Keep subprocess tests minimal and focused on CLI-specific concerns (argument parsing, exit codes, output format). Push logic into library code where it can be unit-tested directly.

## Parallelism and Isolation

Cargo runs tests in parallel by default. Tests must not share mutable state — no global variables, no shared files, no fixed ports. Use temporary directories (`tempfile::tempdir()`) and random or OS-assigned ports.

When tests must run serially (for example, because they modify the same environment variable), use the `serial_test` crate:

```rust
#[serial]
#[test]
fn sets_env_var() { ... }
```

## Continuous Integration

All tests run in CI on every pull request. The CI matrix covers:

- Stable Rust (minimum supported version)
- Linux, macOS

Ignored tests do not run in CI unless the workflow explicitly opts in.

Keep the test suite fast. Slow tests belong behind `#[ignore]` or in a dedicated job that runs on a schedule.
