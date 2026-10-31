## Summary

## Task / issue

## Scope confirmation
- [ ] Only approved scope implemented
- [ ] No unrelated refactor

## Safety
- [ ] No privileged operation without justification
- [ ] No arbitrary shell/PowerShell execution
- [ ] No system modification outside approved action contract
- [ ] Rollback / recovery covered where applicable

## Validation
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace --all-targets`
- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Windows CI
- [ ] Ubuntu CI

## Evidence

## Known limitations
