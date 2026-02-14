```rust
// Infrastructure layer - external concerns (database, filesystem, etc.)
// Implements interfaces defined in application layer

pub mod persistence;
pub mod filesystem;
pub mod messaging;

```