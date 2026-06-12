use std::any::Any;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::error::{OpenCadError, Result};

/// A reversible change applied to a document.
pub trait TransactionAction: fmt::Debug + Send + Sync {
    fn description(&self) -> &str;
    fn apply(&self) -> Result<()>;
    fn rollback(&self) -> Result<()>;
}

/// Transaction lifecycle: begin → apply actions → commit or rollback.
#[derive(Debug)]
pub struct Transaction {
    description: String,
    actions: Vec<Box<dyn TransactionAction>>,
    committed: bool,
}

impl Transaction {
    pub fn begin(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            actions: Vec::new(),
            committed: false,
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn push_action(&mut self, action: Box<dyn TransactionAction>) {
        self.actions.push(action);
    }

    pub fn apply_all(&mut self) -> Result<()> {
        for (index, action) in self.actions.iter().enumerate() {
            action.apply().map_err(|err| {
                OpenCadError::transaction(format!(
                    "failed at action {index} ({}): {err}",
                    action.description()
                ))
            })?;
        }
        Ok(())
    }

    pub fn commit(mut self) -> Result<()> {
        self.apply_all()?;
        self.committed = true;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        if self.committed {
            return Err(OpenCadError::transaction(
                "cannot rollback a committed transaction",
            ));
        }
        for action in self.actions.iter().rev() {
            action.rollback()?;
        }
        Ok(())
    }

    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

/// Atomic counter action used by tests and early prototypes.
#[derive(Debug)]
pub struct CounterAction {
    counter: Arc<AtomicU32>,
    delta: i32,
    description: String,
}

impl CounterAction {
    pub fn new(counter: Arc<AtomicU32>, delta: i32, description: impl Into<String>) -> Self {
        Self {
            counter,
            delta,
            description: description.into(),
        }
    }
}

impl TransactionAction for CounterAction {
    fn description(&self) -> &str {
        &self.description
    }

    fn apply(&self) -> Result<()> {
        if self.delta >= 0 {
            self.counter.fetch_add(self.delta as u32, Ordering::Relaxed);
        } else {
            self.counter
                .fetch_sub(self.delta.unsigned_abs(), Ordering::Relaxed);
        }
        Ok(())
    }

    fn rollback(&self) -> Result<()> {
        if self.delta >= 0 {
            self.counter.fetch_sub(self.delta as u32, Ordering::Relaxed);
        } else {
            self.counter
                .fetch_add(self.delta.unsigned_abs(), Ordering::Relaxed);
        }
        Ok(())
    }
}

/// Type-erased document snapshot for future undo stacks.
#[derive(Debug, Default)]
pub struct TransactionLog {
    entries: Vec<TransactionRecord>,
}

#[derive(Debug)]
struct TransactionRecord {
    #[allow(dead_code)]
    description: String,
    #[allow(dead_code)]
    payload: Box<dyn Any + Send + Sync>,
}

impl TransactionLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn record(&mut self, description: impl Into<String>, payload: Box<dyn Any + Send + Sync>) {
        self.entries.push(TransactionRecord {
            description: description.into(),
            payload,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_commit_and_rollback() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut tx = Transaction::begin("increment twice");
        tx.push_action(Box::new(CounterAction::new(counter.clone(), 1, "add 1")));
        tx.push_action(Box::new(CounterAction::new(
            counter.clone(),
            1,
            "add 1 again",
        )));

        tx.apply_all().expect("apply");
        assert_eq!(counter.load(Ordering::Relaxed), 2);

        tx.rollback().expect("rollback");
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn commit_applies_actions() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut tx = Transaction::begin("commit");
        tx.push_action(Box::new(CounterAction::new(counter.clone(), 5, "add 5")));
        tx.commit().expect("commit");
        assert_eq!(counter.load(Ordering::Relaxed), 5);
    }
}
