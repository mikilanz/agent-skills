use std::sync::LockResult;

/// Extension methods for [`LockResult`].
pub trait LockResultExt {
    type Guard;

    /// Returns the lock guard even if the mutex is poisoned.
    fn ignore_poison(self) -> Self::Guard;
}

impl<Guard> LockResultExt for LockResult<Guard> {
    type Guard = Guard;

    fn ignore_poison(self) -> Guard {
        self.unwrap_or_else(|e| e.into_inner())
    }
}
