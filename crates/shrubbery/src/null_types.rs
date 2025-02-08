//! Null types for making bts that don't do anything for testing purposes

use crate::prelude::*;
use derive_more::From;

/// Null [`ShrubberyBT`] type
pub type NullBT = ShrubberyBT<NullHandler, StandardDecorator>;

/// Null [`BTBuilder`] type
pub type NullBTBuilder = BTBuilder<NullHandler, StandardDecorator>;

#[derive(Default, Debug, Clone)]
pub struct NullHandler;

impl ActionHandler for NullHandler {
    type Bb = Null;

    type Execute = SimpleExecutors;

    type Condition = SimpleConditionals;
}

/// Useless [`Blackboard`]
#[derive(Default, Clone, Copy, Debug)]
pub struct Null;

/// Either [`FailConditional`] or [`PassConditional`]
#[derive(Clone, Copy, Debug, From)]
pub enum SimpleConditionals {
    Fail(FailConditional),
    Pass(PassConditional),
}

impl Conditional<Null> for SimpleConditionals {
    fn conditional(&self, blackboard: &Null) -> Status {
        match self {
            SimpleConditionals::Fail(f) => f.conditional(blackboard),
            SimpleConditionals::Pass(p) => p.conditional(blackboard),
        }
    }
    fn name(&self) -> Option<String> {
        match self {
            SimpleConditionals::Fail(f) => f.name(),
            SimpleConditionals::Pass(p) => p.name(),
        }
    }
    fn details(&self) -> Option<String> {
        match self {
            SimpleConditionals::Fail(f) => f.details(),
            SimpleConditionals::Pass(p) => p.details(),
        }
    }
}

/// [`Conditional`] that always fails
#[derive(Default, Clone, Copy, Debug)]
pub struct FailConditional;

impl Conditional<Null> for FailConditional {
    fn conditional(&self, _: &Null) -> Status {
        Status::Failure
    }
    fn name(&self) -> Option<String> {
        Some("FailConditional".to_string())
    }
    fn details(&self) -> Option<String> {
        Some("Always fails".to_string())
    }
}

/// [`Conditional`] that always passes
#[derive(Default, Clone, Copy, Debug)]
pub struct PassConditional;

impl Conditional<Null> for PassConditional {
    fn conditional(&self, _: &Null) -> Status {
        Status::Success
    }
    fn name(&self) -> Option<String> {
        Some("PassConditional".to_string())
    }
    fn details(&self) -> Option<String> {
        Some("Always passes".to_string())
    }
}

/// Either [`FailExecutor`] or [`PassExecutor`]
#[derive(Clone, Copy, Debug, From)]
pub enum SimpleExecutors {
    Fail(FailExecutor),
    Pass(PassExecutor),
}

impl Executor<Null> for SimpleExecutors {
    fn execute(&self, blackboard: &mut Null) -> Status {
        match self {
            SimpleExecutors::Fail(f) => f.execute(blackboard),
            SimpleExecutors::Pass(p) => p.execute(blackboard),
        }
    }
    fn name(&self) -> Option<String> {
        match self {
            SimpleExecutors::Fail(f) => f.name(),
            SimpleExecutors::Pass(p) => p.name(),
        }
    }
    fn details(&self) -> Option<String> {
        match self {
            SimpleExecutors::Fail(f) => f.details(),
            SimpleExecutors::Pass(p) => p.details(),
        }
    }
}

/// [`Executor`] that always fails
#[derive(Default, Clone, Copy, Debug)]
pub struct FailExecutor;

impl Executor<Null> for FailExecutor {
    fn execute(&self, _: &mut Null) -> Status {
        Status::Failure
    }
    fn name(&self) -> Option<String> {
        Some("FailExecutor".to_string())
    }
    fn details(&self) -> Option<String> {
        Some("Always fails".to_string())
    }
}

/// [`Executor`] that always succeeds
#[derive(Default, Clone, Copy, Debug)]
pub struct PassExecutor;

impl Executor<Null> for PassExecutor {
    fn execute(&self, _: &mut Null) -> Status {
        Status::Success
    }
    fn name(&self) -> Option<String> {
        Some("PassExecutor".to_string())
    }
    fn details(&self) -> Option<String> {
        Some("Always passes".to_string())
    }
}
