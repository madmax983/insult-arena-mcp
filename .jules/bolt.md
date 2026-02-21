# Bolt's Journal

**[Stack Buffering for Search]**
**Learning:** Zero-allocation via lazy iterators can introduce hidden CPU costs (O(N*M) instead of O(M)) if normalization logic is re-executed inside hot loops.
**Action:** When optimizing small, frequent string operations, prefer stack-based buffers (like `[char; 256]`) to store normalized data. This avoids heap allocation *and* repeated computation, providing the best of both worlds.

**[Static Strings vs Buffer Reuse]**
**Learning:** Reusing an existing heap allocation (like `String::clear` + `push_str`) is slower than simply dropping it and using a static reference (`&'static str`) when the target data is constant. `memcpy` is not zero-cost.
**Action:** Always check if the data source is static. If so, prefer `Cow<'static, str>` or `&'static str` over mutable `String` buffers, even if a buffer is already allocated.
