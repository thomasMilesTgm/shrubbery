/* Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
Unauthorized copying of this file, via any medium is strictly prohibited.
Proprietary and confidential. */

use ahash::HashSet;

use super::CTreeNodeID;
use super::ChildUpdate;
use crate::prelude::Inverter;
use crate::prelude::Repeater;
use crate::prelude::StandardDecorator;
use crate::prelude::Subtree;
use crate::traits::*;
use crate::Status;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlNode<D>
where
    D: Decorator,
{
    pub node_type: ControlNodeType<D>,
    pub status: Option<Status>,
    pub id: Option<CTreeNodeID>,
    pub(crate) reset_requests: Vec<CTreeNodeID>,
}

impl<D: Decorator> ControlNode<D> {
    pub fn reset(&mut self) {
        match &mut self.node_type {
            ControlNodeType::Sequence(s) => s.reset(),
            ControlNodeType::Fallback(s) => s.reset(),
            ControlNodeType::Parallel(s) => s.reset(),
            ControlNodeType::Decorator(s) => s.reset(),
        }
    }
    pub fn sequence() -> Self {
        Self {
            node_type: Sequence::default().into(),
            status: None,
            id: None,
            reset_requests: Default::default(),
        }
    }
    pub fn parallel() -> Self {
        Self {
            node_type: Parallel::default().into(),
            status: None,
            id: None,
            reset_requests: Default::default(),
        }
    }
    pub fn fallback() -> Self {
        Self {
            node_type: Fallback::default().into(),
            status: None,
            id: None,
            reset_requests: Default::default(),
        }
    }
    pub fn decorator(decorator: impl Into<D>) -> Self {
        Self {
            node_type: ControlNodeType::Decorator(decorator.into()),
            status: None,
            id: None,
            reset_requests: Default::default(),
        }
    }
}

impl ControlNode<StandardDecorator> {
    pub fn inverter() -> Self {
        Self::decorator(Inverter::default())
    }
    /// Number of retries **after** the first failure (i.e. `repeats + 1` iterations)
    pub fn repeater(retries: usize) -> Self {
        Self::decorator(Repeater::new(retries))
    }
    pub fn subtree() -> Self {
        Self::decorator(Subtree::default())
    }
}

impl<D: Decorator> Control for ControlNode<D> {
    fn tick(&mut self) -> Status {
        // First time this node has been ticked
        if self.status.is_none() {
            self.status = Some(Status::Running);
        }
        let status = match &mut self.node_type {
            ControlNodeType::Sequence(seq) => seq.tick(),
            ControlNodeType::Fallback(f) => f.tick(),
            ControlNodeType::Parallel(p) => p.tick(),
            ControlNodeType::Decorator(d) => d.status(),
        };
        self.status = Some(status);
        status
    }
    fn child_updated(&mut self, update: ChildUpdate) {
        match &mut self.node_type {
            ControlNodeType::Sequence(seq) => seq.child_updated(update),
            ControlNodeType::Fallback(f) => f.child_updated(update),
            ControlNodeType::Parallel(p) => p.child_updated(update),
            ControlNodeType::Decorator(d) => {
                self.status = Some(d.child_updated(update));
            }
        }
        self.tick();
    }
    fn all_children_seen(&mut self) {
        match &mut self.node_type {
            ControlNodeType::Sequence(seq) => seq.all_children_seen(),
            ControlNodeType::Fallback(f) => f.all_children_seen(),
            ControlNodeType::Parallel(p) => p.all_children_seen(),
            ControlNodeType::Decorator(d) => {
                if let Some(reset) = d.reset_request() {
                    self.reset_requests.push(reset);
                }
                // decorators are only allowed one child, so there is no need to do anything as
                // it's implicit that once they get a `child_updated`, they have seen all their
                // children.
            }
        }
    }
}

/// Defines the control flow of the BT.
///
/// If a [`ControlNode`] reached during DFS returns [`Status::Running`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlNodeType<D: Decorator> {
    /// Run children in order, failing immediately if any child fails
    Sequence(Sequence),

    /// Run children in order until one succeeds, fails if all fail, succeeds if any succeed.
    Fallback(Fallback),

    /// Run all children, regardless of their success or failure
    Parallel(Parallel),

    /// Decorators only have one child, and define custom policy. Common decorator policies are
    /// provided in [`StandardDecorator`]
    Decorator(D),
}

impl<D: Decorator> ControlNode<D> {
    pub fn try_as_sequence(&self) -> Option<&Sequence> {
        if let ControlNodeType::Sequence(s) = &self.node_type {
            Some(s)
        } else {
            None
        }
    }
    pub fn is_sequence(&self) -> bool {
        self.try_as_sequence().is_some()
    }
    pub fn try_as_fallback(&self) -> Option<&Fallback> {
        if let ControlNodeType::Fallback(f) = &self.node_type {
            Some(f)
        } else {
            None
        }
    }
    pub fn is_fallback(&self) -> bool {
        self.try_as_fallback().is_some()
    }
    pub fn try_as_parallel(&self) -> Option<&Parallel> {
        if let ControlNodeType::Parallel(p) = &self.node_type {
            Some(p)
        } else {
            None
        }
    }
    pub fn is_parallel(&self) -> bool {
        self.try_as_parallel().is_some()
    }
    pub fn try_as_decorator(&self) -> Option<&D> {
        if let ControlNodeType::Decorator(d) = &self.node_type {
            Some(d)
        } else {
            None
        }
    }
    pub fn is_decorator(&self) -> bool {
        self.try_as_decorator().is_some()
    }
}

impl<D: Decorator> From<Sequence> for ControlNodeType<D> {
    fn from(seq: Sequence) -> Self {
        ControlNodeType::Sequence(seq)
    }
}

impl<D: Decorator> From<Fallback> for ControlNodeType<D> {
    fn from(fallback: Fallback) -> Self {
        ControlNodeType::Fallback(fallback)
    }
}
impl<D: Decorator> From<Parallel> for ControlNodeType<D> {
    fn from(parallel: Parallel) -> Self {
        ControlNodeType::Parallel(parallel)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Sequence {
    /// how many children are pending
    pub pending: HashSet<CTreeNodeID>,
    pub failed: Option<CTreeNodeID>,
    pub status: Option<Status>,
    pub finished: bool,
}

impl Sequence {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn reset(&mut self) {
        self.pending.clear();
        self.failed = None;
        self.status = None;
        self.finished = false;
    }
}

impl Control for Sequence {
    fn tick(&mut self) -> Status {
        if self.failed.is_some() {
            self.status = Some(Status::Failure);
            return Status::Failure;
        }
        if self.finished {
            self.status = Some(Status::Success);
            return Status::Success;
        }
        if self.status.is_none() {
            self.status = Some(Status::Running);
        }
        self.status.unwrap_or_default()
    }

    fn child_updated(&mut self, update: ChildUpdate) {
        match update.status {
            Status::Running => {
                self.pending.insert(update.child_id);
            }
            Status::Success => {
                self.pending.remove(&update.child_id);
            }
            Status::Failure => {
                self.failed = Some(update.child_id);
            }
        }
    }

    fn all_children_seen(&mut self) {
        if self.pending.is_empty() {
            self.finished = true;
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Fallback {
    pub status: Option<Status>,
}

impl Fallback {
    pub fn reset(&mut self) {
        self.status = None;
    }
}

impl Control for Fallback {
    fn tick(&mut self) -> Status {
        self.status.unwrap_or_default()
    }

    fn child_updated(&mut self, update: ChildUpdate) {
        if update.status.is_success() {
            self.status = Some(Status::Success);
        }
    }

    fn all_children_seen(&mut self) {
        self.status = self
            .status
            .map(|s| s.into_failure_if_running())
            .or(Some(Status::Failure));
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Parallel {
    pub success: HashSet<CTreeNodeID>,
    pub failure: HashSet<CTreeNodeID>,
    pub pending: HashSet<CTreeNodeID>,
    pub finished: bool,
}

impl Parallel {
    pub fn reset(&mut self) {
        self.success.clear();
        self.failure.clear();
        self.pending.clear();
        self.finished = false;
    }
}

impl Control for Parallel {
    /// All child nodes are to run, regardless of success/failure state.
    fn tick(&mut self) -> Status {
        //
        if self.finished {
            if self.failure.is_empty() {
                Status::Success
            } else {
                Status::Failure
            }
        } else {
            Status::Running
        }
    }

    fn child_updated(&mut self, update: ChildUpdate) {
        let ChildUpdate { status, child_id } = update;
        match status {
            Status::Success => {
                self.pending.remove(&child_id);
                self.success.insert(child_id);
            }
            Status::Failure => {
                self.pending.remove(&child_id);
                self.failure.insert(child_id);
            }
            Status::Running => {
                self.pending.insert(child_id);
            }
        }
    }

    fn all_children_seen(&mut self) {
        if self.pending.is_empty() {
            self.finished = true;
        }
    }
}
