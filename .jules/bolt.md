**[Zero-allocation string normalization]**
**Learning:** When normalizing strings (e.g. for case-insensitive comparison), prefer iterators (`chars().flat_map(char::to_lowercase)`) over allocating new `String`s (`collect().to_lowercase()`). This avoids O(N) heap allocations in hot loops.
**Action:** Check string normalization logic in hot paths and refactor to use iterator chains.
