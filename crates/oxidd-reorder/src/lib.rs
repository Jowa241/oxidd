//! Functionality to reorder levels in a decision diagram

#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use oxidd_core::{HasLevel, LevelNo, LevelSwapper, LevelView, Manager, Node, VarNo};

use std::time::{Duration, Instant};

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

fn time_over(start: Instant, limit: Duration) -> bool {
    if start.elapsed() >= limit {
        true
    } else {
        false
    }
}

// todo: outdated
/// Find optimal position for 'level'
pub fn sifting_one_level<M: Manager>(
    manager: &M,
    level: LevelNo,
    //time_limit: Duration,
    level_limit: usize,
) -> isize
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    //get time
    let start: Instant = Instant::now();

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
    let mut counter: usize = 0;

    // Maybe we can speed up by only checking each position once, not twice in some cases?
    'outer: for _n in 1..=2 {
        // assuming levels start at index 1
        if direction {
            while current_level > 1 {
                let diff = unsafe { level_down(manager, current_level - 1) }; // move level up
                counter += 1;
                current_level -= 1;

                // check if we found a better place
                if diff < 0 {
                    best += diff;
                    bestpos = current_level;
                }

                if counter >= level_limit {
                    break 'outer;
                }

                /*
                if time_over(start, time_limit) {
                    println!("(sifting) time limit exceeded...moving layer into best position now");
                    break 'outer;
                }; */
            }
        }
        if !direction {
            while current_level < max_level {
                let diff = unsafe { level_down(manager, current_level) }; // move level down
                counter += 1;
                current_level += 1;

                // check if we found a better place
                if diff < 0 {
                    best += diff;
                    bestpos = current_level;
                }

                if counter >= level_limit {
                    break 'outer;
                }
                /*
                if time_over(start, time_limit) {
                    println!("(sifting) time limit exceeded...moving layer into best position now");
                    break 'outer;
                }; */
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

    println!("Needed {:?} ms for sifting one layer", start.elapsed());

    best
}

// todo: outdated
/// sifts the given sequence of levels into the right order
pub fn sifting_variable_sequence<M: Manager>(
    manager: &M,
    seq: &[VarNo],
    time_limit: Duration,
    level_limit: usize,
) where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    let start = Instant::now();
    let mut counter = 0;
    // Execute reordering
    for var in seq {
        let level: LevelNo = manager.var_to_level(*var);
        sifting_one_level(manager, level, level_limit);
        counter += 1;
        if time_over(start, time_limit) {
            break;
        }
    }
    println!("Sifted {} Layers", counter);
}

// todo: normal sifting is outdated
/// implements sifting
pub fn sifting<M: Manager>(manager: &M, time_limit: Duration, level_limit: usize)
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    let mut vars: Vec<u32> = vec![];

    for level in manager.levels() {
        vars.push(manager.level_to_var(level.level_no()));
    }

    sifting_variable_sequence(manager, &vars, time_limit, level_limit);
}

/// Find optimal position for 'level'
pub fn sifting_one_level_optimized<M: Manager>(
    manager: &M,
    level: LevelNo,
    mapping: &Vec<LevelNo>,
    //time_limit: Duration,
    level_limit: usize,
) where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    let start: Instant = Instant::now();

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

    let upper_level_limit: LevelNo = starting_level.saturating_add(level_limit as LevelNo);
    let lower_level_limit: LevelNo = starting_level.saturating_sub(level_limit as LevelNo);

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
                        mapping[upper_var as usize],
                        mapping[current_var as usize],
                    )
                }; // move level up
                current_level -= 1;

                // check if we found a better place
                if diff < 0 {
                    bestpos = current_level;
                }
                if current_level <= lower_level_limit {
                    break;
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
                        mapping[current_var as usize],
                        mapping[lower_var as usize],
                    )
                }; // move level down
                current_level += 1;

                // check if we found a better place
                if diff < 0 {
                    bestpos = current_level;
                }

                if current_level >= upper_level_limit {
                    break;
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
                mapping[upper_var as usize],
                mapping[current_var as usize],
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
                mapping[current_var as usize],
                mapping[lower_var as usize],
            )
        }; // move level down
        current_level += 1;
    }

    println!(
        "(Optimized) Needed {:?} ms for sifting one layer",
        start.elapsed()
    );
}

/// sifts the given sequence of levels into the right order
pub fn sifting_variable_sequence_optimized<M: Manager>(
    manager: &M,
    seq: &[VarNo],
    time_limit: Duration,
    level_limit: usize,
) -> usize
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    let var_to_nodelevels: Vec<LevelNo> = (0..manager.num_vars())
        .map(|var| manager.var_to_level(var))
        .collect();

    let start = Instant::now();
    let mut counter = 0;
    // Execute reordering
    for var in seq {
        let level: LevelNo = manager.var_to_level(*var);
        sifting_one_level_optimized(manager, level, &var_to_nodelevels, level_limit);
        counter += 1;
        if time_over(start, time_limit) {
            break;
        }
    }
    // Fix node levels
    for level in manager.levels() {
        update_level_no(manager, &level);
    }

    println!("(optimized) Sifted {} Layers", counter);
    counter
}

/// implements sifting
pub fn sifting_optimized<M: Manager>(manager: &M, time_limit: Duration, level_limit: usize) -> usize
where
    M::InnerNode: HasLevel,
    M::Rules: LevelSwapper<M>,
{
    let mut vars: Vec<u32> = vec![];

    for level in manager.levels() {
        vars.push(manager.level_to_var(level.level_no()));
    }

    return sifting_variable_sequence_optimized(manager, &vars, time_limit, level_limit);
}

/// Finds and sifts a variable sequence based on the num of nodes on each level
pub fn sifting_heuristic_sequence<M: Manager>(
    manager: &M,
    num: usize,
    time_limit: Duration,
    level_limit: usize,
) -> usize
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

    return sifting_variable_sequence_optimized(manager, &heuristic, time_limit, level_limit);
}
