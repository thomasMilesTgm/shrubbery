/* Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
Unauthorized copying of this file, via any medium is strictly prohibited.
Proprietary and confidential. */

//! # Full BT

use crate::control::ControlTree;
use crate::executor_mask::{LeafDispatch, TaskHook};
use crate::graphviz::GraphvizAttrs;
use crate::prelude::{BTBuilder, StandardDecorator};
use crate::traits::*;
use crate::Status;

pub mod builder;

/* 4x generics Bt */

/// Behavior Tree with [`Executor`] and [`Conditional`] dispatch
#[derive(Debug, Clone)]
pub struct ShrubberyBT<Handler: ActionHandler, Decor: Decorator = StandardDecorator> {
    pub(crate) control_tree: ControlTree<Decor>,
    pub(crate) dispatch: LeafDispatch<Handler>,
}

impl<H: ActionHandler, D: Decorator> Default for ShrubberyBT<H, D> {
    fn default() -> Self {
        Self {
            control_tree: Default::default(),
            dispatch: Default::default(),
        }
    }
}

impl<H: ActionHandler, D: Decorator> ShrubberyBT<H, D> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> BTBuilder<H, D> {
        BTBuilder::new()
    }

    pub fn into_builder(self) -> BTBuilder<H, D> {
        BTBuilder::from(self)
    }

    pub fn run(&mut self, blackboard: &mut H::Bb) -> Status {
        let mut control_tree = std::mem::take(&mut self.control_tree);
        let dispatch = &self.dispatch;

        let mut task_hook = TaskHook {
            dispatch,
            blackboard,
        };
        control_tree.run(&mut task_hook)
    }
}

impl<H: ActionHandler, D: Decorator + GraphvizAttrs> ShrubberyBT<H, D> {
    pub fn run_save_animation(
        &mut self,
        blackboard: &mut H::Bb,
        file_name: &str,
        frame_time: f32,
    ) -> Status {
        let mut task_hook = TaskHook {
            dispatch: &self.dispatch,
            blackboard,
        };
        self.control_tree
            .run_save_animation(&mut task_hook, file_name, frame_time)
    }

    pub fn save_dot(&self, name: &str) {
        self.control_tree.save_dot(name);
    }
}
