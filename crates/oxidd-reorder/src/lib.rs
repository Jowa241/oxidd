//! Functionality to reorder levels in a decision diagram

#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::{array, iter::Map};

use oxidd_core::{HasLevel, InnerNode, LevelNo, LevelSwapper, LevelView, Manager, Node, VarNo};

mod set_var_order;
pub use set_var_order::{set_var_order, set_var_order_seq};

/// Swap the level given by `upper_no` with the level directly below.
///
/// # Safety
///
/// Must be called from inside the closure of
/// [`manager.reorder()`][Manager::reorder]. `manager` must be derived from a
/// `&mut M` reference. This function may modify nodes at the level of
/// `upper_no` and `upper_no + 1` (i.e., the level below). There must not be any
/// concurrent modification of any nodes at these levels.
pub unsafe fn level_down<M: Manager>(manager: &M, upper_no: LevelNo) -> isize
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    assert!(upper_no + 1 < manager.num_levels());
    let diff = unsafe { M::Rules::swap(manager, upper_no, upper_no + 1, upper_no, upper_no + 1) };
    // SAFETY (next 2): follows from the above assertion
    update_level_no(manager, &unsafe { manager.level_unchecked(upper_no) });
    update_level_no(manager, &unsafe { manager.level_unchecked(upper_no + 1) });
    diff
}

/// Write the level number `level` to all nodes referenced from the `level`-th
/// level view.
fn update_level_no<M: Manager>(manager: &M, level: &M::LevelView<'_>)
where
    M::InnerNode: HasLevel,
{
    let level_no = level.level_no();
    for e in level.iter() {
        let Node::Inner(node) = manager.get_node(e) else {
            // SAFETY: all edges in level views refer to inner nodes
            unsafe { std::hint::unreachable_unchecked() }
        };
        // SAFETY: setting the level number re-establishes the invariant
        unsafe { node.set_level(level_no) };
    }
}

/// Find optimal position for 'level'
pub fn sifting_one_level<M: Manager>(manager: &M, level: LevelNo) -> isize
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    debug_assert!(level < manager.num_levels()); // todo : assertion correct?
    // get mid position and direction
    let max_level: LevelNo = manager.num_levels() - 1;
    let mid_level: LevelNo = max_level / 2;
    let mut direction: bool = level <= mid_level; // true = go to level 1 first

    // Get state
    // record starting position to skip the way back
    let starting_level: LevelNo = level;
    //let old: isize = 0;
    let mut best = 0;
    let mut bestpos: LevelNo = starting_level;

    let mut current_level: LevelNo = starting_level;

    // Maybe we can speed up by only checking each position once, not twice in some cases?
    for _n in 1..=2 {
        // assuming levels start at index 1
        if direction {
            while current_level > 1 {
                let diff = unsafe { level_down(manager, current_level - 1) }; // move level up
                current_level -= 1;

                // check if we found a better place
                if diff < 0 {
                    best += diff;
                    bestpos = current_level;
                }
            }
        }
        if !direction {
            while current_level < max_level {
                let diff = unsafe { level_down(manager, current_level) }; // move level down
                current_level += 1;

                // check if we found a better place
                if diff < 0 {
                    best += diff;
                    bestpos = current_level;
                }
            }
        }

        direction = !direction; // switch the direction
    }

    while bestpos < current_level {
        unsafe { level_down(manager, current_level - 1) }; // move level up
        current_level -= 1;
    }
    while bestpos > current_level {
        unsafe { level_down(manager, current_level) }; // move level down
        current_level += 1;
    }

    best
}

/// Find optimal position for 'level'
pub fn sifting_one_level_optimized<M: Manager>(
    manager: &M,
    level: LevelNo,
    mapping: &std::collections::HashMap<VarNo, LevelNo>,
) where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    debug_assert!(level < manager.num_levels()); // todo : assertion correct?
    // get mid position and direction
    let max_level: LevelNo = manager.num_levels() - 1;
    let mid_level: LevelNo = max_level / 2;
    let mut direction: bool = level <= mid_level; // true = go to level 1 first

    // Get state
    // record starting position to skip the way back
    let starting_level: LevelNo = level;
    //let old: isize = 0;
    let mut bestpos: LevelNo = starting_level;

    let mut current_level: LevelNo = starting_level;

    // Maybe we can speed up by only checking each position once, not twice in some cases?
    for _n in 1..=2 {
        // assuming levels start at index 1
        if direction {
            while current_level > 1 {
                // OLD VERSION : let diff = unsafe { level_down(manager, current_level - 1) }; // move level up
                // get the variables
                let current_var: VarNo = manager.level_to_var(current_level);
                let upper_var: VarNo = manager.level_to_var(current_level - 1);
                let diff = unsafe {
                    M::Rules::swap(
                        manager,
                        current_level - 1,
                        current_level,
                        *mapping.get(&upper_var).unwrap(),
                        *mapping.get(&current_var).unwrap(),
                    )
                }; // move level up
                current_level -= 1;

                // check if we found a better place
                if diff < 0 {
                    bestpos = current_level;
                }
            }
        }
        if !direction {
            while current_level < max_level {
                let current_var: VarNo = manager.level_to_var(current_level);
                let lower_var: VarNo = manager.level_to_var(current_level + 1);
                let diff = unsafe {
                    M::Rules::swap(
                        manager,
                        current_level,
                        current_level + 1,
                        *mapping.get(&current_var).unwrap(),
                        *mapping.get(&lower_var).unwrap(),
                    )
                }; // move level down
                current_level += 1;

                // check if we found a better place
                if diff < 0 {
                    bestpos = current_level;
                }
            }
        }

        direction = !direction; // switch the direction
    }

    while bestpos < current_level {
        let current_var: VarNo = manager.level_to_var(current_level);
        let upper_var: VarNo = manager.level_to_var(current_level - 1);
        unsafe {
            M::Rules::swap(
                manager,
                current_level - 1,
                current_level,
                *mapping.get(&upper_var).unwrap(),
                *mapping.get(&current_var).unwrap(),
            )
        }; // move level up
        current_level -= 1;
    }
    while bestpos > current_level {
        let current_var: VarNo = manager.level_to_var(current_level);
        let lower_var: VarNo = manager.level_to_var(current_level + 1);
        unsafe {
            M::Rules::swap(
                manager,
                current_level,
                current_level + 1,
                *mapping.get(&current_var).unwrap(),
                *mapping.get(&lower_var).unwrap(),
            )
        }; // move level down
        current_level += 1;
    }
}

/// sifts the given sequence of levels into the right order
pub fn sifting_variable_sequence<M: Manager>(manager: &M, seq: &[VarNo])
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    // Execute reordering
    for var in seq {
        let level: LevelNo = manager.var_to_level(*var);
        sifting_one_level(manager, level);
    }
}

/// sifts the given sequence of levels into the right order
pub fn sifting_variable_sequence_optimized<M: Manager>(manager: &M, seq: &[VarNo])
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    let mut var_to_nodelevels: std::collections::HashMap<VarNo, LevelNo> =
        std::collections::HashMap::new();
    for var in seq {
        var_to_nodelevels.insert(*var, manager.var_to_level(*var));
    }
    // Execute reordering
    for var in seq {
        let level: LevelNo = manager.var_to_level(*var);
        sifting_one_level_optimized(manager, level, &var_to_nodelevels);
    }
    // Fix node levels
    for level in manager.levels() {
        update_level_no(manager, &level);
    }
}

/// Finds and sifts a variable sequence based on the num of nodes on each level
pub fn sifting_heuristic_sequence<M: Manager>(manager: &M, num: usize)
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    let mut seq: Vec<(usize, VarNo)> = vec![];

    for level in manager.levels() {
        let nodes = level.len();
        let var = manager.level_to_var(level.level_no());
        seq.push((nodes, var));
    }

    seq.sort();

    let seq: Vec<u32> = seq.into_iter().map(|(_nodes, var)| var).rev().collect();

    let heuristic: &[u32] = if num < seq.len() { &seq[..num] } else { &seq };

    sifting_variable_sequence(manager, &heuristic);
}

/// implements sifting
pub fn sifting<M: Manager>(manager: &M)
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    let mut vars: Vec<u32> = vec![];

    for level in manager.levels() {
        vars.push(manager.level_to_var(level.level_no()));
    }

    sifting_variable_sequence(manager, &vars);
}

/// implements sifting
pub fn sifting_optimized<M: Manager>(manager: &M)
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    let mut vars: Vec<u32> = vec![];

    for level in manager.levels() {
        vars.push(manager.level_to_var(level.level_no()));
    }

    sifting_variable_sequence_optimized(manager, &vars);
}
