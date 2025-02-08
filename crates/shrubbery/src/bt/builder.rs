/* Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
Unauthorized copying of this file, via any medium is strictly prohibited.
Proprietary and confidential. */

use crate::bt::ShrubberyBT;
use crate::control::builder::{CTreeBuilder, CTreeLayerBuilder};
use crate::control::{CTreeNodeID, LeafNode, ROOT_ID};
use crate::executor_mask::LeafDispatch;
use crate::prelude::ControlNode;
use crate::ShrubberyResult;

use super::*;

pub type BTLayerFn<'a, O, H, D> = fn(BTLayer<H, D>) -> O;
pub type BTLayerFnWithDeps<'a, Deps, O, H, D> = fn(Deps, BTLayer<H, D>) -> O;

/// For building control trees
pub struct BTBuilder<H: ActionHandler, D: Decorator = StandardDecorator> {
    inner: CTreeBuilder<D>,
    dispatch: LeafDispatch<H>,
}

impl<H: ActionHandler, D: Decorator> BTBuilder<H, D> {
    pub fn new() -> Self {
        Self {
            inner: CTreeBuilder::new(),
            dispatch: Default::default(),
        }
    }

    pub fn layer_with_deps<Deps, O>(
        &mut self,
        deps: Deps,
        f: BTLayerFnWithDeps<Deps, O, H, D>,
    ) -> O {
        let BTBuilder { inner, dispatch } = self;

        f(
            deps,
            BTLayer {
                control: CTreeLayerBuilder::new(inner, ROOT_ID),
                dispatch,
            },
        )
    }
    pub fn layer<O>(&mut self, f: BTLayerFn<O, H, D>) -> O {
        let BTBuilder { inner, dispatch } = self;

        f(BTLayer {
            control: CTreeLayerBuilder::new(inner, ROOT_ID),
            dispatch,
        })
    }

    /// Build the [`ShrubberyBT`]
    ///
    /// # Errors
    ///
    /// - If a cycle is detected in the tree
    /// - If a control node is left dangling (missing leaf)
    /// - If a decorator has more than one child
    pub fn build(self) -> ShrubberyResult<ShrubberyBT<H, D>> {
        // validate the tree
        let control_tree = self.inner.build()?;

        Ok(ShrubberyBT {
            control_tree,
            dispatch: self.dispatch.into(),
        })
    }

    /// Inject a cycle. This will make [`Self::build`] fail, so don't use it unless you're testing
    /// that.
    #[cfg(test)]
    pub fn inject_cycle(&mut self) {
        self.inner.inject_cycle();
    }
}

impl<H: ActionHandler, D: Decorator> From<ShrubberyBT<H, D>> for BTBuilder<H, D> {
    fn from(value: ShrubberyBT<H, D>) -> Self {
        BTBuilder {
            inner: value.control_tree.into_builder(),
            dispatch: value.dispatch,
        }
    }
}

impl<H: ActionHandler, D: Decorator> Default for BTBuilder<H, D> {
    fn default() -> Self {
        Self::new()
    }
}

/* -- Layer builder -- */

pub struct BTLayer<'a, H: ActionHandler, D: Decorator = StandardDecorator> {
    control: CTreeLayerBuilder<'a, D>,
    dispatch: &'a mut LeafDispatch<H>,
}

impl<'a, H: ActionHandler, D: Decorator> BTLayer<'a, H, D> {
    /// Do something to the layer
    pub fn map<O>(self, f: fn(Self) -> O) -> O {
        f(self)
    }

    /// Do something to the layer
    pub fn update<O>(&mut self, f: fn(&mut Self) -> O) -> O {
        f(self)
    }

    pub fn map_with_deps<Deps>(self, deps: Deps, f: fn(Deps, Self) -> Self) -> Self {
        f(deps, self)
    }

    /// Add an executor node to the tree & dispatch
    pub fn execute(&mut self, executor: impl Into<H::Execute>) -> CTreeNodeID {
        let executor = executor.into();
        let id = self.control.leaf_node(LeafNode::from_executor(&executor));
        self.dispatch.add_executor(id, executor);
        id
    }

    /// Add a conditional node to the tree & dispatch
    pub fn condition(&mut self, conditional: impl Into<H::Condition>) -> CTreeNodeID {
        let conditional = conditional.into();
        let id = self
            .control
            .leaf_node(LeafNode::from_conditional(&conditional));

        self.dispatch.add_conditional(id, conditional);
        id
    }

    pub fn sequence<O>(&mut self, layer_fn: BTLayerFn<'_, O, H, D>) -> O {
        self.control_node(ControlNode::sequence(), layer_fn)
    }
    pub fn sequence_with_deps<Deps, O>(
        &mut self,
        deps: Deps,
        layer_fn: BTLayerFnWithDeps<'_, Deps, O, H, D>,
    ) -> O {
        self.control_node_with_deps(deps, ControlNode::sequence(), layer_fn)
    }

    pub fn fallback<O>(&mut self, layer_fn: BTLayerFn<'_, O, H, D>) -> O {
        self.control_node(ControlNode::fallback(), layer_fn)
    }
    pub fn fallback_with_deps<Deps, O>(
        &mut self,
        deps: Deps,
        layer_fn: BTLayerFnWithDeps<'_, Deps, O, H, D>,
    ) -> O {
        self.control_node_with_deps(deps, ControlNode::fallback(), layer_fn)
    }

    pub fn parallel<O>(&mut self, layer_fn: BTLayerFn<'_, O, H, D>) -> O {
        self.control_node(ControlNode::parallel(), layer_fn)
    }
    pub fn parallel_with_deps<Deps, O>(
        &mut self,
        deps: Deps,
        layer_fn: BTLayerFnWithDeps<'_, Deps, O, H, D>,
    ) -> O {
        self.control_node_with_deps(deps, ControlNode::parallel(), layer_fn)
    }

    pub fn decorator<O>(&mut self, decorator: impl Into<D>, layer_fn: BTLayerFn<'_, O, H, D>) -> O {
        let node = ControlNode::decorator(decorator.into());
        self.control_node(node, layer_fn)
    }
    pub fn decorator_with_deps<Deps, O>(
        &mut self,
        deps: Deps,
        decorator: impl Into<D>,
        layer_fn: BTLayerFnWithDeps<'_, Deps, O, H, D>,
    ) -> O {
        let node = ControlNode::decorator(decorator.into());
        self.control_node_with_deps(deps, node, layer_fn)
    }

    pub fn control_node<O>(
        &mut self,
        node: impl Into<ControlNode<D>>,
        layer_fn: BTLayerFn<'_, O, H, D>,
    ) -> O {
        let next_layer = self.control.next_layer(node);
        layer_fn(BTLayer {
            control: next_layer,
            dispatch: &mut self.dispatch,
        })
    }

    pub fn control_node_with_deps<Deps, O>(
        &mut self,
        deps: Deps,
        node: impl Into<ControlNode<D>>,
        layer_fn: BTLayerFnWithDeps<'_, Deps, O, H, D>,
    ) -> O {
        let next_layer = self.control.next_layer(node);
        layer_fn(
            deps,
            BTLayer {
                control: next_layer,
                dispatch: &mut self.dispatch,
            },
        )
    }
}

impl<'a, H: ActionHandler, D: Decorator + From<StandardDecorator>> BTLayer<'a, H, D> {
    pub fn repeater<O>(&mut self, retries: usize, layer_fn: BTLayerFn<'_, O, H, D>) -> O {
        let decorator = D::from(StandardDecorator::repeater(retries));
        let node = ControlNode::decorator(decorator);
        self.control_node(node, layer_fn)
    }
    pub fn repeater_with_deps<Deps, O>(
        &mut self,
        deps: Deps,
        retries: usize,
        layer_fn: BTLayerFnWithDeps<'_, Deps, O, H, D>,
    ) -> O {
        let decorator = D::from(StandardDecorator::repeater(retries));
        let node = ControlNode::decorator(decorator);
        self.control_node_with_deps(deps, node, layer_fn)
    }

    pub fn inverter<O>(&mut self, layer_fn: BTLayerFn<'_, O, H, D>) -> O {
        let decorator = D::from(StandardDecorator::inverter());
        let node = ControlNode::decorator(decorator);
        self.control_node(node, layer_fn)
    }

    pub fn inverter_with_deps<Deps, O>(
        &mut self,
        deps: Deps,
        layer_fn: BTLayerFnWithDeps<'_, Deps, O, H, D>,
    ) -> O {
        let decorator = D::from(StandardDecorator::inverter());
        let node = ControlNode::decorator(decorator);
        self.control_node_with_deps(deps, node, layer_fn)
    }

    pub fn subtree<O>(&mut self, layer_fn: BTLayerFn<'_, O, H, D>) -> O {
        let decorator = D::from(StandardDecorator::subtree());
        let node = ControlNode::decorator(decorator);
        self.control_node(node, layer_fn)
    }
    pub fn subtree_with_deps<Deps, O>(
        &mut self,
        deps: Deps,
        layer_fn: BTLayerFnWithDeps<'_, Deps, O, H, D>,
    ) -> O {
        let decorator = D::from(StandardDecorator::subtree());
        let node = ControlNode::decorator(decorator);
        self.control_node_with_deps(deps, node, layer_fn)
    }
}
