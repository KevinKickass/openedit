use crate::cursor::Cursor;
use crate::edit::EditOp;

/// A transaction groups multiple edit operations that should be undone together.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub ops: Vec<EditOp>,
    /// Cursor state before this transaction.
    pub cursor_before: Cursor,
    /// Cursor state after this transaction.
    pub cursor_after: Cursor,
}

/// Operation-based undo/redo manager.
///
/// Maintains a stack of transactions. When a new transaction is pushed
/// after an undo, the redo stack is cleared.
#[derive(Debug)]
pub struct UndoManager {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
    /// If set, we're currently recording operations into a transaction.
    current_transaction: Option<Transaction>,
    /// Whether we're inside begin_transaction/commit_transaction.
    in_transaction: bool,
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_transaction: None,
            in_transaction: false,
        }
    }

    /// Begin a transaction that groups multiple edit ops.
    pub fn begin_transaction(&mut self, cursor: Cursor) {
        self.current_transaction = Some(Transaction {
            ops: Vec::new(),
            cursor_before: cursor,
            cursor_after: cursor,
        });
        self.in_transaction = true;
    }

    /// Record an edit operation into the current transaction.
    pub fn record(&mut self, op: EditOp, cursor: Cursor) {
        if let Some(ref mut txn) = self.current_transaction {
            txn.ops.push(op);
            txn.cursor_after = cursor;
        } else {
            // Auto-create a single-op transaction
            let txn = Transaction {
                ops: vec![op],
                cursor_before: cursor,
                cursor_after: cursor,
            };
            self.undo_stack.push(txn);
            self.redo_stack.clear();
        }
    }

    /// Commit the current transaction to the undo stack.
    pub fn commit_transaction(&mut self, cursor: Cursor) {
        if let Some(mut txn) = self.current_transaction.take() {
            txn.cursor_after = cursor;
            if !txn.ops.is_empty() {
                self.undo_stack.push(txn);
                self.redo_stack.clear();
            }
        }
        self.in_transaction = false;
    }

    /// Pop the last transaction for undoing. Returns the transaction to apply.
    pub fn undo(&mut self) -> Option<Transaction> {
        self.undo_stack.pop().inspect(|txn| {
            self.redo_stack.push(txn.clone());
        })
    }

    /// Pop the last redo transaction. Returns the transaction to apply.
    pub fn redo(&mut self) -> Option<Transaction> {
        self.redo_stack.pop().inspect(|txn| {
            self.undo_stack.push(txn.clone());
        })
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current_transaction = None;
        self.in_transaction = false;
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::Cursor;
    use crate::edit::EditOp;

    #[test]
    fn test_single_op_undo() {
        let mut mgr = UndoManager::new();
        let cursor = Cursor::new(0, 5);
        mgr.record(
            EditOp::Insert {
                offset: 0,
                text: "hello".into(),
            },
            cursor,
        );
        assert!(mgr.can_undo());
        let txn = mgr.undo().unwrap();
        assert_eq!(txn.ops.len(), 1);
        assert!(!mgr.can_undo());
        assert!(mgr.can_redo());
    }

    #[test]
    fn test_transaction_grouping() {
        let mut mgr = UndoManager::new();
        let cursor = Cursor::new(0, 0);
        mgr.begin_transaction(cursor);
        mgr.record(
            EditOp::Insert {
                offset: 0,
                text: "a".into(),
            },
            Cursor::new(0, 1),
        );
        mgr.record(
            EditOp::Insert {
                offset: 1,
                text: "b".into(),
            },
            Cursor::new(0, 2),
        );
        mgr.commit_transaction(Cursor::new(0, 2));

        assert!(mgr.can_undo());
        let txn = mgr.undo().unwrap();
        assert_eq!(txn.ops.len(), 2);
    }

    #[test]
    fn test_undo_clears_redo() {
        let mut mgr = UndoManager::new();
        let cursor = Cursor::new(0, 0);
        mgr.record(
            EditOp::Insert {
                offset: 0,
                text: "a".into(),
            },
            cursor,
        );
        mgr.undo();
        assert!(mgr.can_redo());

        // New edit clears redo
        mgr.record(
            EditOp::Insert {
                offset: 0,
                text: "b".into(),
            },
            cursor,
        );
        assert!(!mgr.can_redo());
    }
}
