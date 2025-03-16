use std::{fs, path::PathBuf};

pub enum RollbackOperation {
    RemoveFile(PathBuf),
    RemoveDir(PathBuf),
}

pub struct Transaction {
    rollback_operations: Vec<RollbackOperation>,
}
impl Transaction {
    pub fn new() -> Self {
        Transaction {
            rollback_operations: vec![],
        }
    }

    pub fn add_operation(&mut self, operation: RollbackOperation) {
        self.rollback_operations.push(operation);
    }

    pub fn commit(&mut self) {
        self.rollback_operations.clear();
    }

    pub fn rollback(&mut self) {
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
    }
}
// NOTE: What happens if an error occurs while trying to rollback, then what
impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.rollback_operations.is_empty() {
            log::debug!("⚠️...rolling back operations");
            self.rollback();
        } else {
            log::debug!("...commiting transaction ✅");
            self.commit();
        }
    }
}
