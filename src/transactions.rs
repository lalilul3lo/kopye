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
pub struct Transaction<State: TransactionState> {
    rollback_operations: Vec<RollbackOperation>,
    _state: PhantomData<State>,
}
impl Transaction<Active> {
    pub fn new() -> Self {
        Transaction {
            rollback_operations: vec![],
            _state: PhantomData,
        }
    }

    pub fn add_operation(&mut self, operation: RollbackOperation) {
        self.rollback_operations.push(operation);
    }

    pub fn commit(mut self) -> Transaction<Committed> {
        self.rollback_operations.clear();

        Transaction {
            rollback_operations: vec![],
            _state: PhantomData,
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
