# Forge's Journal

**[Borrow Checker & Option Pattern]**
**Learning:** When a struct field is `Option<T>` and you need to inspect its state before performing a mutation, you cannot hold `self.field.as_mut()` while calling another `&self` method, as this creates simultaneous mutable and immutable borrows.
**Action:** Use `self.field.as_ref()` to inspect state (and copy/clone what's needed for the decision), let that borrow expire, and *then* call `self.field.as_mut()` to perform the action.

**[Clean Display Implementations]**
**Learning:** `std::fmt::Display` implementations often grow into "God Functions" with complex matching logic.
**Action:** Extract complex branches into private "formatter helper" functions (e.g., `fmt_exchange_processed`) that take the formatter and necessary data. This keeps the main `fmt` clean and readable.

**[Refactoring Hygiene]**
**Learning:** Extracting methods often exposes redundant clones or unnecessary closures that were hidden in nested logic.
**Action:** Always run `cargo clippy` immediately after a refactor to catch these newly exposed optimizations (like `clippy::redundant_clone` or `clippy::redundant_closure_for_method_calls`).
