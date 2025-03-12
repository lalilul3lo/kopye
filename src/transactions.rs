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
                    let _ = fs::remove_file(&path);
                }
                RollbackOperation::RemoveFile(path) => {
                    let _ = fs::remove_dir_all(&path);
                }
            }
        }
    }
}
impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.rollback_operations.is_empty() {
            self.rollback();
        } else {
            self.commit();
        }
    }
}
