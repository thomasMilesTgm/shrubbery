/* Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
Unauthorized copying of this file, via any medium is strictly prohibited.
Proprietary and confidential. */

//! # Connects [`LeafNode(s)`](crate::control::LeafNode) to [`Executor`] & [`Conditional`]
//! implementations.

use ahash::HashMap;
use derive_more::From;

use crate::{
    control::{CTreeNodeID, LeafNode},
    traits::*,
    Status,
};

#[derive(Debug, Clone, Copy, From)]
enum TaskID {
    Executor(ExecutorID),
    Conditional(ConditionalID),
}

#[derive(Debug, Clone, Copy)]
struct ExecutorID(usize);

#[derive(Debug, Clone, Copy)]
struct ConditionalID(usize);

/// Short lived reference to a [`LeafMask`] and `&mut` [`Blackboard`] to dispatch [`Executor`] and
/// [`Conditional`] behavior when leaf nodes are ticked.
///
/// Importantly, this type implements [`ExecutorHook`], so it can plug into
pub struct TaskHook<'a, H: ActionHandler> {
    pub dispatch: &'a LeafDispatch<H>,
    pub blackboard: &'a mut H::Bb,
}

impl<H: ActionHandler> ExecutorHook for TaskHook<'_, H> {
    fn hook(&mut self, leaf: &LeafNode) -> Status {
        let TaskHook {
            dispatch: leaf_mask,
            blackboard,
        } = self;
        let Some(leaf_id) = leaf.id.as_ref() else {
            log::error!("LeafNode must have an ID");
            return Status::Failure;
        };
        let Some(target_id) = leaf_mask.mask.get(leaf_id) else {
            log::error!("Leaf id {:?} is not handled by this LeafMask", leaf_id);
            return Status::Failure;
        };

        match *target_id {
            TaskID::Executor(e) => leaf_mask[e].execute(blackboard),
            TaskID::Conditional(c) => leaf_mask[c].conditional(blackboard),
        }
    }
}

/// Dispatch to [`Conditional`]/[`Executor`] implementers when  [`LeafNode`] is ticked.
#[derive(Debug, Clone)]
pub struct LeafDispatch<Handler: ActionHandler> {
    /// Leaf nodes that are [`Conditional`] (read-only)
    conditionals: Vec<Handler::Condition>,
    /// Leaf nodes that are [`Executor`] (read-write)
    executors: Vec<Handler::Execute>,
    /// Maps which leaf node corresponds to which [`Executor`]/[`Conditional`]
    mask: HashMap<CTreeNodeID, TaskID>,
}

impl<H: ActionHandler> Default for LeafDispatch<H> {
    fn default() -> Self {
        Self {
            conditionals: Default::default(),
            executors: Default::default(),
            mask: Default::default(),
        }
    }
}

impl<H: ActionHandler> LeafDispatch<H> {
    /// Assign an [`Executor`] to a particular [`CTreeNodeID`]
    pub fn add_executor(&mut self, id: CTreeNodeID, executor: H::Execute) {
        let target_id: TaskID = ExecutorID(self.executors.len()).into();
        self.executors.push(executor);
        self.mask.insert(id, target_id);
    }

    /// Assign a [`Conditional`] to a particular [`CTreeNodeID`]
    pub fn add_conditional(&mut self, id: CTreeNodeID, conditional: H::Condition) {
        let target_id: TaskID = ConditionalID(self.conditionals.len()).into();
        self.conditionals.push(conditional);
        self.mask.insert(id, target_id);
    }
}

/* --- Boilerplate --- */

impl<H: ActionHandler> std::ops::Index<ConditionalID> for LeafDispatch<H> {
    type Output = H::Condition;
    fn index(&self, id: ConditionalID) -> &Self::Output {
        &self.conditionals[id.0]
    }
}

impl<H: ActionHandler> std::ops::IndexMut<ConditionalID> for LeafDispatch<H> {
    fn index_mut(&mut self, index: ConditionalID) -> &mut Self::Output {
        &mut self.conditionals[index.0]
    }
}

impl<H: ActionHandler> std::ops::Index<ExecutorID> for LeafDispatch<H> {
    type Output = H::Execute;
    fn index(&self, id: ExecutorID) -> &Self::Output {
        &self.executors[id.0]
    }
}

impl<H: ActionHandler> std::ops::IndexMut<ExecutorID> for LeafDispatch<H> {
    fn index_mut(&mut self, index: ExecutorID) -> &mut Self::Output {
        &mut self.executors[index.0]
    }
}
