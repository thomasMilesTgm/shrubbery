# Shrubbery

![animation](https://github.com/thomasMilesTgm/shrubbery/blob/main/crates/shrubbery/doc/shrub-dark.gif?raw=true)

![it is a good shrubbery](https://github.com/thomasMilesTgm/shrubbery/blob/main/crates/shrubbery/doc/shrubbery.jpg?raw=true)

## Motivation

Fundamentally behavior trees are _simple_, leaf nodes perform some action and return one of
three states: Success, Failure, or Running. All other nodes nodes determine how tree traversal
is done -- effectively controlling which execution nodes get executed and in what order.

## BT vs FSM

FSMs are more explicit in their state transitions, but can be more difficult to manage as the
number of states grows, especially when it comes to modification of behavior. In contrast,
BTs are more flexible and can be more easily modified, since behavior is localized strictly
to the sub-tree where the behavior is defined, changes to a sub-tree do not impact the program
outside of that sub-tree.

The trade offs are:

- Reactivity: FSMs are more reactive, since state transitions are explicit, in a BT the
  execution flow is essentially compiled into the shape of the tree.

## Motivation for Shrubbery (why not keep using `bonsai_bt`)

IMO bonsai has a few insurmountable issues that make it unsuitable:

- API: The way the BT gets defined is by constructing a single, ultra-nested enum of
  `Behavior<T>` upfront that cannot be changed once the `BT` is instantiated. This is both
  cumbersome to and error prone to write (a problem I tried to mitigate in `bt_functional`), and
  more importantly, very difficult to compose subtrees with different `T` into a greater
  behavior tree and there is no mechanism for helping with this

**Node seperation**

Control (internal) nodes and Executor (leaf) nodes are not delineated (they are all `Behavior<T>`).

- To `&mut` or not to `&mut`: typically execution nodes have two flavors:
    - Read a value from the blackboard and return `Success` if it's present, `Failure` if
      it's not, or `Running` to indicate a loop should continue executing.
    - Execute a task & update the blackboard with the outcome, return `Success`, `Failure`,
      or `Running` to reflect the outcome of the task.
    - In bonsai, you always have an `&mut Blackboard` when handling an action in a tick and
      the read vs. execute and write distinction doesn't exist, this makes it tempting to do
      work in places that it's not appropriate.

- If Control is independent of Execution, `T` (the executable behavior type) is independent
  of Control, thus by building the `BT` out of indices into a `Vec<ControlNode>` and a
  `Vec<ExecutorNode>` (which can be done implicitly since nodes are executors iff they are
  a leaf, and nodes are control iff they are not executors). Seperating these in this way
  makes sub-tree composition trivially easy.

**Graphviz**

This feels like a nit, but I don't think it is. A lot of the debugging behavior trees is made a
lot easier with the ability to look at the tree itself.

A nicely formatted graphviz tree goes a long way to make it easy to parse what is going on &
there are a standard set of symbols typically used to do so:

![sample bt](https://github.com/thomasMilesTgm/shrubbery/blob/main/crates/shrubbery/doc/sample_bt.png?raw=true)

Petgraphs' `Dot` implementation isn't great since you can't style nodes differently and it relies
purely on the implementation of [`std::fmt::Debug`] (something very useless if your nodes are ids),
but at least you can use [`grahviz-rust`] & tree traveral to DIY it.

Bonsai uses petgraph, but doens't expose the actual graph in the public API, they just wrap the
[`petgraph::Dot`] implementation and return a string. We could vendor `bonsai` or try and land a
PR to get around this, but the other issues I have mean I really don't care.


