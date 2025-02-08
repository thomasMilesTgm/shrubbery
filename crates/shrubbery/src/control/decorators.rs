/* Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
Unauthorized copying of this file, via any medium is strictly prohibited.
Proprietary and confidential. */

use super::CTreeNodeID;
use super::ChildUpdate;
use crate::traits::*;
use crate::Status;

use derive_more::From;

#[derive(Debug, Clone, PartialEq, Eq, Hash, From)]
pub enum StandardDecorator {
    /// Inverts the child's output status
    Invert(Inverter),

    /// Repeatedly re-run the child until it succeeds or [`Repeater::count`] goes runs to zero
    Repeat(Repeater),

    /// Marker to indicate the branch is a subtree.
    Subtree(Subtree),
}

impl StandardDecorator {
    pub fn inverter() -> Self {
        Inverter::default().into()
    }

    pub fn repeater(retries: usize) -> Self {
        Repeater::new(retries).into()
    }
    pub fn subtree() -> Self {
        Subtree::default().into()
    }
}

impl Decorator for StandardDecorator {
    fn child_updated(&mut self, update: ChildUpdate) -> Status {
        match self {
            StandardDecorator::Invert(i) => i.child_updated(update),
            StandardDecorator::Repeat(r) => r.child_updated(update),
            StandardDecorator::Subtree(s) => s.child_updated(update),
        }
    }
    fn init(&mut self) {
        match self {
            StandardDecorator::Invert(i) => i.init(),
            StandardDecorator::Repeat(r) => r.init(),
            StandardDecorator::Subtree(s) => s.init(),
        }
    }
    fn status(&self) -> Status {
        match self {
            StandardDecorator::Invert(i) => i.status(),
            StandardDecorator::Repeat(r) => r.status(),
            StandardDecorator::Subtree(s) => s.status(),
        }
    }
    fn reset(&mut self) {
        match self {
            StandardDecorator::Invert(i) => i.reset(),
            StandardDecorator::Repeat(r) => r.reset(),
            StandardDecorator::Subtree(s) => s.reset(),
        }
    }
    fn reset_request(&mut self) -> Option<CTreeNodeID> {
        match self {
            StandardDecorator::Invert(i) => i.reset_request(),
            StandardDecorator::Repeat(r) => r.reset_request(),
            StandardDecorator::Subtree(s) => s.reset_request(),
        }
    }
    fn name(&self) -> String {
        match self {
            StandardDecorator::Invert(i) => i.name(),
            StandardDecorator::Repeat(r) => r.name(),
            StandardDecorator::Subtree(s) => s.name(),
        }
    }
    fn details(&self) -> Option<String> {
        match self {
            StandardDecorator::Invert(i) => Some(format!("{i:#?}")),
            StandardDecorator::Repeat(r) => Some(format!("{r:#?}")),
            StandardDecorator::Subtree(s) => Some(format!("{s:#?}")),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Subtree {
    status: Option<Status>,
    name: Option<String>,
}

impl Subtree {
    pub fn new(name: String) -> Self {
        Self {
            status: None,
            name: Some(name),
        }
    }
}

impl Decorator for Subtree {
    fn child_updated(&mut self, update: ChildUpdate) -> Status {
        self.status = Some(update.status);
        update.status
    }

    fn init(&mut self) {}

    fn status(&self) -> Status {
        self.status.unwrap_or_default()
    }

    fn reset(&mut self) {
        self.status = None;
    }

    fn name(&self) -> String {
        self.name.clone().unwrap_or("Subtree".to_string())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Inverter {
    child_status: Option<Status>,
}

impl Decorator for Inverter {
    fn child_updated(&mut self, update: ChildUpdate) -> Status {
        self.child_status = Some(update.status);
        !update.status
    }
    fn init(&mut self) {}
    fn status(&self) -> Status {
        !self.child_status.unwrap_or_default()
    }
    fn reset(&mut self) {
        self.child_status = None;
    }
    fn name(&self) -> String {
        "Inverter".to_string()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Repeater {
    /// How many retries are allowed (not including the first attempt)
    pub init_retry: usize,

    /// How many retry attempts are are left
    pub retry: usize,

    /// The current status
    pub status: Option<Status>,

    /// The child to reset if it failed.
    pub reset_request: Option<CTreeNodeID>,
}

impl Repeater {
    /// Number of retries **after** the first failure
    pub fn new(retries: usize) -> Self {
        Self {
            init_retry: retries + 1,
            retry: retries + 1,
            status: None,
            reset_request: None,
        }
    }
    pub fn can_retry(&self) -> bool {
        self.retry > 0
    }
}

impl Decorator for Repeater {
    fn child_updated(&mut self, update: ChildUpdate) -> Status {
        if !self.can_retry() {
            // out of retries, whatever the update was, is what we're gonna get
            self.status = Some(Status::Failure);
            return Status::Failure;
        }
        match update.status {
            Status::Success => {
                self.status = Some(Status::Success);
                self.reset_request = None;
                Status::Success
            }
            Status::Failure => {
                self.retry -= 1;
                self.status = Some(Status::Running);
                self.reset_request = Some(update.child_id);
                Status::Running
            }
            Status::Running => {
                self.status = Some(Status::Running);
                self.reset_request = Some(update.child_id);
                Status::Running
            }
        }
    }
    fn init(&mut self) {
        self.status = Some(Status::Running);
    }
    fn status(&self) -> Status {
        if self.can_retry() {
            self.status.unwrap_or_default()
        } else {
            self.status.unwrap_or(Status::Failure)
        }
    }
    fn reset_request(&mut self) -> Option<CTreeNodeID> {
        if self.can_retry() {
            self.reset_request.take()
        } else {
            None
        }
    }
    fn reset(&mut self) {
        self.reset_request = None;
        self.status = None;
        self.retry = self.init_retry;
    }
    fn name(&self) -> String {
        format!("Repeat({})", self.retry)
    }
}
