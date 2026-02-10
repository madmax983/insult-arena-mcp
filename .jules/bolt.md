# Bolt's Journal

**[Stack Buffering for Search]**
**Learning:** Zero-allocation via lazy iterators can introduce hidden CPU costs (O(N*M) instead of O(M)) if normalization logic is re-executed inside hot loops.
**Action:** When optimizing small, frequent string operations, prefer stack-based buffers (like `[char; 256]`) to store normalized data. This avoids heap allocation *and* repeated computation, providing the best of both worlds.
