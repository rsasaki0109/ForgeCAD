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
    /// Number of actions that have applied successfully and still need to be
    /// rolled back.  Failed actions are deliberately not included.
    applied_actions: usize,
    committed: bool,
}

impl Transaction {
    pub fn begin(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            actions: Vec::new(),
            applied_actions: 0,
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
        for index in self.applied_actions..self.actions.len() {
            let action = &self.actions[index];
            if let Err(err) = action.apply() {
                let apply_error = OpenCadError::transaction(format!(
                    "failed at action {index} ({}): {err}",
                    action.description()
                ));
                return match self.rollback_applied_actions() {
                    Ok(()) => Err(apply_error),
                    Err(rollback_error) => Err(OpenCadError::transaction(format!(
                        "{apply_error}; {rollback_error}"
                    ))),
                };
            }
            self.applied_actions = index + 1;
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
        self.rollback_applied_actions()
    }

    fn rollback_applied_actions(&mut self) -> Result<()> {
        let mut first_error = None;
        for index in (0..self.applied_actions).rev() {
            let action = &self.actions[index];
            if let Err(err) = action.rollback() {
                if first_error.is_none() {
                    first_error = Some(OpenCadError::transaction(format!(
                        "failed to rollback action {index} ({}): {err}",
                        action.description()
                    )));
                }
            }
        }
        self.applied_actions = 0;
        match first_error {
            Some(err) => Err(err),
            None => Ok(()),
        }
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

    #[derive(Debug)]
    struct FailingAction {
        rollback_calls: Arc<AtomicU32>,
    }

    impl TransactionAction for FailingAction {
        fn description(&self) -> &str {
            "fail"
        }

        fn apply(&self) -> Result<()> {
            Err(OpenCadError::validation("injected failure"))
        }

        fn rollback(&self) -> Result<()> {
            self.rollback_calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn failed_commit_rolls_back_only_successfully_applied_actions() {
        let counter = Arc::new(AtomicU32::new(0));
        let failed_rollback_calls = Arc::new(AtomicU32::new(0));
        let mut tx = Transaction::begin("rollback on failure");
        tx.push_action(Box::new(CounterAction::new(
            counter.clone(),
            1,
            "add before failure",
        )));
        tx.push_action(Box::new(FailingAction {
            rollback_calls: failed_rollback_calls.clone(),
        }));

        tx.commit().expect_err("injected failure");
        assert_eq!(counter.load(Ordering::Relaxed), 0);
        assert_eq!(failed_rollback_calls.load(Ordering::Relaxed), 0);
    }
}
