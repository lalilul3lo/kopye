use std::{fs, marker::PhantomData, path::PathBuf};

/// Enum of possible operations to rollback
pub enum RollbackOperation {
    RemoveFile(PathBuf),
    RemoveDir(PathBuf),
}
/// Active Transaction
pub struct Active;
/// Committed Transaction
pub struct Committed;
/// Canceled Transaction
pub struct Canceled;
/// A trait that tells us if rollback should occur when dropped.
pub trait TransactionState {
    const SHOULD_ROLLBACK: bool;
}
impl TransactionState for Active {
    const SHOULD_ROLLBACK: bool = true;
}
impl TransactionState for Committed {
    const SHOULD_ROLLBACK: bool = false;
}
impl TransactionState for Canceled {
    const SHOULD_ROLLBACK: bool = true;
}
/// Represents the final state of a transaction after it has been explicitly resolved.
///
/// This enum captures whether a [`Transaction<Active>`] was concluded by either committing
/// or canceling it. It wraps the corresponding finalized transaction to preserve context
/// for further inspection or logging if needed.
///
/// This is particularly useful in cases where you want to track or return the outcome of
/// a transaction without immediately discarding the object.
///
/// # Variants
///
/// - `Committed`: The transaction was finalized successfully, and no rollback will occur.
/// - `Canceled`: The transaction was intentionally aborted, and rollback will occur on drop.
///
/// # Example
///
/// ```rust
/// let trx = Transaction::<Active>::new();
/// let final_state = if should_commit {
///     FinalTransactionState::Committed(trx.commit())
/// } else {
///     FinalTransactionState::Canceled(trx.cancel())
/// };
/// ```
#[allow(dead_code)]
pub enum FinalTransactionState {
    Committed(Transaction<Committed>),
    Canceled(Transaction<Canceled>),
}
/// Represents a transactional context that tracks rollback operations.
///
/// This struct is parameterized over a `TransactionState` which determines its behavior
/// when dropped. If the state indicates rollback (`SHOULD_ROLLBACK = true`) and there are
/// registered operations, the `Drop` implementation will attempt to undo those operations,
/// such as removing created files or directories.
///
/// # Type Parameters
///
/// - `State`: A marker type that implements [`TransactionState`], used to indicate
///   whether the transaction is `Active`, `Committed`, or `Canceled`.
///
/// # Usage
///
/// - Use `Transaction<Active>` when beginning a transaction.
/// - Call `.commit()` to finalize the transaction and prevent rollback.
/// - Call `.cancel()` to finalize the transaction while preserving rollback behavior.
///
/// Rollback operations include:
/// - [`RollbackOperation::RemoveFile`]
/// - [`RollbackOperation::RemoveDir`]
///
/// # Example
///
/// ```rust
/// let mut trx = Transaction::<Active>::new();
/// trx.add_operation(RollbackOperation::RemoveFile("some/path".into()));
/// trx.commit(); // No rollback will happen
/// ```
pub struct Transaction<State: TransactionState> {
    rollback_operations: Vec<RollbackOperation>,
    state: PhantomData<State>,
}
impl Transaction<Active> {
    pub fn new() -> Self {
        Transaction {
            rollback_operations: vec![],
            state: PhantomData,
        }
    }
    /// Adds a rollback operation to the current transaction.
    ///
    /// This registers an action that should be reversed if the transaction is canceled
    /// or dropped without being committed. Typical operations include removing
    /// created files or directories.
    pub fn add_operation(&mut self, operation: RollbackOperation) {
        self.rollback_operations.push(operation);
    }
    /// Finalizes the transaction, preventing any rollback from occurring.
    ///
    /// This clears all previously registered rollback operations and returns a
    /// [`Transaction<Committed>`] which does nothing on drop.
    pub fn commit(mut self) -> Transaction<Committed> {
        self.rollback_operations.clear();

        Transaction {
            rollback_operations: vec![],
            state: PhantomData,
        }
    }
    /// Cancels the transaction, preserving the rollback operations.
    ///
    /// This returns a [`Transaction<Canceled>`] that will execute all registered
    /// rollback operations when dropped. Use this when an error or user action
    /// requires undoing all changes made during the transaction.
    pub fn cancel(mut self) -> Transaction<Canceled> {
        let rollback_operations = std::mem::take(&mut self.rollback_operations);

        Transaction {
            rollback_operations,
            state: PhantomData,
        }
    }
}
impl<S: TransactionState> Drop for Transaction<S> {
    fn drop(&mut self) {
        if S::SHOULD_ROLLBACK && !self.rollback_operations.is_empty() {
            log::debug!("⚠️...rolling back operations");
            while let Some(operation) = self.rollback_operations.pop() {
                match operation {
                    RollbackOperation::RemoveDir(path) => {
                        log::debug!("🚨...removing dir: {}", path.display());
                        let _ = fs::remove_dir_all(&path);
                    }
                    RollbackOperation::RemoveFile(path) => {
                        log::debug!("🚨...removing file: {}", path.display());
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        } else if !S::SHOULD_ROLLBACK {
            log::debug!("...committing transaction ✅");
        }
    }
}
