#![cfg(any(feature = "bcdd", feature = "bdd", feature = "zbdd"))]

// spell-checker:ignore nvars,mref

use oxidd::util::AllocResult;
use oxidd::{
    BooleanFunction, Function, HasLevel, HasWorkers, InnerNode, LevelNo, Manager, ManagerRef, VarNo,
};
use oxidd_core::function::{INodeOfFunc, RulesOfFunc};
use oxidd_core::{LevelSwapper, LevelView};
use oxidd_reorder::{level_down, set_var_order};

#[cfg(feature = "bcdd")]
use oxidd::bcdd::BCDDFunction;
#[cfg(feature = "bdd")]
use oxidd::bdd::BDDFunction;
#[cfg(feature = "zbdd")]
use oxidd::zbdd::ZBDDFunction;

#[cfg(feature = "bdd")]
#[test]
fn reverse_empty_bdd() {
    let mref = oxidd::bdd::new_manager(1024, 128, 2);
    mref.with_manager_exclusive(|manager| {
        let nvars = if cfg!(miri) { 128 } else { 1024 };
        let mut order = Vec::from_iter(manager.add_vars(nvars));

        set_var_order(manager, &order);
        assert!((0..nvars).all(|i| manager.level_to_var(i) == i));

        order.reverse();
        set_var_order(manager, &order);
        assert!((0..nvars).all(|i| manager.level_to_var(i) == nvars - i - 1));
    });
}

fn check_order<M: Manager>(manager: &M, order: &[VarNo]) {
    assert_eq!(manager.num_levels() as usize, order.len());
    for (i, &v) in order.iter().enumerate() {
        assert_eq!(manager.level_to_var(i as LevelNo), v);
    }
}

fn print_order<M: Manager>(manager: &M) {
    let num = manager.num_levels();
    for i in 0..num {
        print!("{} ", manager.level_to_var(i as LevelNo));
    }
}

fn reorder_nonempty<B: BooleanFunction>(mref: B::ManagerRef) -> AllocResult<()>
where
    for<'id> B::Manager<'id>: HasWorkers,
    for<'id> INodeOfFunc<'id, B>: HasLevel,
    for<'id> RulesOfFunc<'id, B>: LevelSwapper<B::Manager<'id>>,
{
    mref.with_manager_exclusive(|manager| manager.add_vars(5));

    let [x1, x3, x4, f] = mref.with_manager_shared(|manager| {
        let x1 = B::var(manager, 1)?;
        let x3 = B::var(manager, 3)?;
        let x4 = B::var(manager, 4)?;
        let f = x1.ite(&x3, &x4)?;
        Ok([x1, x3, x4, f])
    })?;

    let check = |order| {
        mref.with_manager_shared(|manager| {
            // order
            check_order(manager, order);

            // structure
            for level in manager.levels() {
                let i = level.level_no();
                for e in level.iter() {
                    let node = manager.get_node(e).unwrap_inner();
                    assert!(node.children().all(|e| manager.get_node(e).level() > i));
                }
            }
        });

        // semantics
        for a in [false, true] {
            assert_eq!(x1.eval([(1, a)]), a);
            assert_eq!(x3.eval([(3, a)]), a);
            assert_eq!(x4.eval([(4, a)]), a);
            for b in [false, true] {
                for c in [false, true] {
                    assert_eq!(f.eval([(1, a), (3, b), (4, c)]), if a { b } else { c });
                }
            }
        }
    };

    let reorder = |order| mref.with_manager_exclusive(|manager| set_var_order(manager, order));

    check(&[0, 1, 2, 3, 4]);
    reorder(&[2, 3, 1]);
    check(&[0, 2, 3, 1, 4]);
    reorder(&[1, 4, 3]);
    check(&[0, 2, 1, 4, 3]);
    reorder(&[4, 1]);
    check(&[0, 2, 4, 1, 3]);
    reorder(&[3, 1, 4]);
    check(&[0, 2, 3, 1, 4]);

    Ok(())
}

#[cfg(feature = "bcdd")]
#[test]
fn reorder_nonempty_bcdd() -> AllocResult<()> {
    reorder_nonempty::<BCDDFunction>(oxidd::bcdd::new_manager(1024, 128, 2))
}

#[cfg(feature = "bdd")]
#[test]
fn reorder_nonempty_bdd() -> AllocResult<()> {
    reorder_nonempty::<BDDFunction>(oxidd::bdd::new_manager(1024, 128, 2))
}

#[cfg(feature = "zbdd")]
#[test]
fn reorder_nonempty_zbdd() -> AllocResult<()> {
    reorder_nonempty::<ZBDDFunction>(oxidd::zbdd::new_manager(1024, 128, 2))
}

#[cfg(feature = "bdd")]
#[test]
fn reorder_drops_child_referenced_only_by_parent() -> AllocResult<()> {
    let mref = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(2));

    let _conj = mref.with_manager_shared(|manager| {
        let x0 = BDDFunction::var(manager, 0)?;
        let x1 = BDDFunction::var(manager, 1)?;
        x0.and(&x1)
    })?;
    // Only `conj` is still alive; the node for `x1 ∧ x2` is referenced by the
    // root node of `conj` alone.

    mref.with_manager_exclusive(|manager| set_var_order(manager, &[1, 0]));
    Ok(())
}

#[test]
fn simple_level_down() -> AllocResult<()> {
    let mref = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(2));

    let _conj = mref.with_manager_shared(|manager| {
        let x0 = BDDFunction::var(manager, 0)?;
        let x1 = BDDFunction::var(manager, 1)?;
        x0.and(&x1)
    })?;
    // Only `conj` is still alive; the node for `x1 ∧ x2` is referenced by the
    // root node of `conj` alone.
    let upper: LevelNo = mref.with_manager_exclusive(|manager| manager.var_to_level(0));

    Ok(())
}

#[test]
fn testing_level_count() -> AllocResult<()> {
    let mref = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(2));

    let _conj: BDDFunction = mref.with_manager_shared(|manager| {
        let x0: BDDFunction = BDDFunction::var(manager, 0)?;
        let x1: BDDFunction = BDDFunction::var(manager, 1)?;
        let cnf: BDDFunction = x0.and(&x1)?;

        Ok(cnf)
    })?;
    // Only `conj` is still alive; the node for `x1 ∧ x2` is referenced by the
    // root node of `conj` alone.
    mref.with_manager_shared(|manager| {
        assert_eq!(manager.num_inner_nodes(), 4);
    });

    Ok(())
}

#[test]
fn sifting_one_level_into_best_position_up_bigger() -> AllocResult<()> {
    let mref: oxidd::bdd::BDDManagerRef = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(10));

    let f: BDDFunction = mref.with_manager_shared(|manager| {
        let x0: BDDFunction = BDDFunction::var(manager, 0)?;
        let x1: BDDFunction = BDDFunction::var(manager, 1)?;
        let x2: BDDFunction = BDDFunction::var(manager, 2)?;
        let x3: BDDFunction = BDDFunction::var(manager, 3)?;
        let x4: BDDFunction = BDDFunction::var(manager, 4)?;
        let x5: BDDFunction = BDDFunction::var(manager, 5)?;
        let x6: BDDFunction = BDDFunction::var(manager, 6)?;
        let x7: BDDFunction = BDDFunction::var(manager, 7)?;
        let x8: BDDFunction = BDDFunction::var(manager, 8)?;
        let x9: BDDFunction = BDDFunction::var(manager, 9)?;

        let clause1 = x0.or(&x8)?;
        let clause2 = x1.or(&x2)?;
        let clause3 = x3.or(&x4)?;
        let clause4 = x5.or(&x6)?;
        let clause5 = x7.or(&x9)?;

        let cnf = clause1
            .and(&clause2)?
            .and(&clause3)?
            .and(&clause4)?
            .and(&clause5)?;

        Ok(cnf)
    })?;

    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 20);
    });

    let check = |order| {
        mref.with_manager_shared(|manager| {
            check_order(manager, order);
        });
    };

    check(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    mref.with_manager_exclusive(|manager| {
        let level_no = manager.var_to_level(8);
        manager.reorder(|reorder_context| {
            oxidd_reorder::sifting_one_level(reorder_context, level_no);
        });
    });
    check(&[0, 8, 1, 2, 3, 4, 5, 6, 7, 9]);
    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 12);
    });

    Ok(())
}

#[test]
fn sifting_one_level_into_best_position_up_simple() -> AllocResult<()> {
    let mref: oxidd::bdd::BDDManagerRef = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(4));

    let f: BDDFunction = mref.with_manager_shared(|manager| {
        let x0: BDDFunction = BDDFunction::var(manager, 0)?;
        let x1: BDDFunction = BDDFunction::var(manager, 1)?;
        let x2: BDDFunction = BDDFunction::var(manager, 2)?;
        let x3: BDDFunction = BDDFunction::var(manager, 3)?;

        let clause1 = x0.or(&x3)?;
        let clause2 = x1.or(&x2)?;
        let cnf = clause1.and(&clause2)?;

        Ok(cnf)
    })?;

    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 8);
    });

    let check = |order| {
        mref.with_manager_shared(|manager| {
            check_order(manager, order);
        });
    };

    check(&[0, 1, 2, 3]);

    mref.with_manager_exclusive(|manager| {
        let level_no = manager.var_to_level(3);
        manager.reorder(|reorder_context| {
            oxidd_reorder::sifting_one_level(reorder_context, level_no);
        });
    });
    check(&[0, 3, 1, 2]);
    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 6);
    });

    Ok(())
}

#[test]
fn sifting_one_level_into_best_position_down_bigger() -> AllocResult<()> {
    let mref: oxidd::bdd::BDDManagerRef = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(10));

    let f: BDDFunction = mref.with_manager_shared(|manager| {
        let x0: BDDFunction = BDDFunction::var(manager, 0)?;
        let x1: BDDFunction = BDDFunction::var(manager, 1)?;
        let x2: BDDFunction = BDDFunction::var(manager, 2)?;
        let x3: BDDFunction = BDDFunction::var(manager, 3)?;
        let x4: BDDFunction = BDDFunction::var(manager, 4)?;
        let x5: BDDFunction = BDDFunction::var(manager, 5)?;
        let x6: BDDFunction = BDDFunction::var(manager, 6)?;
        let x7: BDDFunction = BDDFunction::var(manager, 7)?;
        let x8: BDDFunction = BDDFunction::var(manager, 8)?;
        let x9: BDDFunction = BDDFunction::var(manager, 9)?;

        let clause2 = x0.or(&x2)?;
        let clause1 = x1.or(&x9)?;
        let clause3 = x3.or(&x4)?;
        let clause4 = x5.or(&x6)?;
        let clause5 = x7.or(&x8)?;

        let cnf = clause1
            .and(&clause2)?
            .and(&clause3)?
            .and(&clause4)?
            .and(&clause5)?;

        Ok(cnf)
    })?;

    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 20);
    });

    let check = |order| {
        mref.with_manager_shared(|manager| {
            check_order(manager, order);
        });
    };

    check(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    mref.with_manager_exclusive(|manager| {
        let level_no = manager.var_to_level(1);
        manager.reorder(|reorder_context| {
            oxidd_reorder::sifting_one_level(reorder_context, level_no);
        });
    });
    check(&[0, 2, 3, 4, 5, 6, 7, 8, 9, 1]);
    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 12);
    });

    Ok(())
}

#[test]
fn sifting_two_levels_into_best_position() -> AllocResult<()> {
    let mref: oxidd::bdd::BDDManagerRef = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(10));

    let f: BDDFunction = mref.with_manager_shared(|manager| {
        let x0: BDDFunction = BDDFunction::var(manager, 0)?;
        let x1: BDDFunction = BDDFunction::var(manager, 1)?;
        let x2: BDDFunction = BDDFunction::var(manager, 2)?;
        let x3: BDDFunction = BDDFunction::var(manager, 3)?;
        let x4: BDDFunction = BDDFunction::var(manager, 4)?;
        let x5: BDDFunction = BDDFunction::var(manager, 5)?;
        let x6: BDDFunction = BDDFunction::var(manager, 6)?;
        let x7: BDDFunction = BDDFunction::var(manager, 7)?;
        let x8: BDDFunction = BDDFunction::var(manager, 8)?;
        let x9: BDDFunction = BDDFunction::var(manager, 9)?;

        let clause2 = x0.or(&x4)?;
        let clause1 = x1.or(&x2)?;
        let clause3 = x3.or(&x5)?;
        let clause4 = x6.or(&x9)?;
        let clause5 = x7.or(&x8)?;

        let cnf = clause1
            .and(&clause2)?
            .and(&clause3)?
            .and(&clause4)?
            .and(&clause5)?;

        Ok(cnf)
    })?;

    let check = |order| {
        mref.with_manager_shared(|manager| {
            check_order(manager, order);
        });
    };

    check(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 18);
    });

    mref.with_manager_exclusive(|manager| {
        let level1 = manager.var_to_level(4);
        let level2 = manager.var_to_level(9);
        let seq: [u32; 2] = [level1, level2];
        manager.reorder(|reorder_context| {
            oxidd_reorder::sifting_variable_sequence(reorder_context, &seq);
        });
    });
    check(&[0, 4, 1, 2, 3, 5, 9, 6, 7, 8]);
    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 12);
    });

    Ok(())
}

#[test]
fn sifting_easy_test() -> AllocResult<()> {
    let mref: oxidd::bdd::BDDManagerRef = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(10));

    let f: BDDFunction = mref.with_manager_shared(|manager| {
        let x0: BDDFunction = BDDFunction::var(manager, 0)?;
        let x1: BDDFunction = BDDFunction::var(manager, 1)?;
        let x2: BDDFunction = BDDFunction::var(manager, 2)?;
        let x3: BDDFunction = BDDFunction::var(manager, 3)?;
        let x4: BDDFunction = BDDFunction::var(manager, 4)?;
        let x5: BDDFunction = BDDFunction::var(manager, 5)?;
        let x6: BDDFunction = BDDFunction::var(manager, 6)?;
        let x7: BDDFunction = BDDFunction::var(manager, 7)?;
        let x8: BDDFunction = BDDFunction::var(manager, 8)?;
        let x9: BDDFunction = BDDFunction::var(manager, 9)?;

        let clause2 = x0.or(&x4)?;
        let clause1 = x1.or(&x2)?;
        let clause3 = x3.or(&x5)?;
        let clause4 = x6.or(&x9)?;
        let clause5 = x7.or(&x8)?;

        let cnf = clause1
            .and(&clause2)?
            .and(&clause3)?
            .and(&clause4)?
            .and(&clause5)?;

        Ok(cnf)
    })?;

    let check = |order| {
        mref.with_manager_shared(|manager| {
            check_order(manager, order);
        });
    };

    check(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 18);
    });

    mref.with_manager_exclusive(|manager| {
        manager.reorder(|reorder_context| {
            oxidd_reorder::sifting(reorder_context);
        });
        print_order(manager);
    });

    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 12);
    });

    Ok(())
}

#[test]
fn sifting_optimized_easy_test() -> AllocResult<()> {
    let mref: oxidd::bdd::BDDManagerRef = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(10));

    let f: BDDFunction = mref.with_manager_shared(|manager| {
        let x0: BDDFunction = BDDFunction::var(manager, 0)?;
        let x1: BDDFunction = BDDFunction::var(manager, 1)?;
        let x2: BDDFunction = BDDFunction::var(manager, 2)?;
        let x3: BDDFunction = BDDFunction::var(manager, 3)?;
        let x4: BDDFunction = BDDFunction::var(manager, 4)?;
        let x5: BDDFunction = BDDFunction::var(manager, 5)?;
        let x6: BDDFunction = BDDFunction::var(manager, 6)?;
        let x7: BDDFunction = BDDFunction::var(manager, 7)?;
        let x8: BDDFunction = BDDFunction::var(manager, 8)?;
        let x9: BDDFunction = BDDFunction::var(manager, 9)?;

        let clause2 = x0.or(&x4)?;
        let clause1 = x1.or(&x2)?;
        let clause3 = x3.or(&x5)?;
        let clause4 = x6.or(&x9)?;
        let clause5 = x7.or(&x8)?;

        let cnf = clause1
            .and(&clause2)?
            .and(&clause3)?
            .and(&clause4)?
            .and(&clause5)?;

        Ok(cnf)
    })?;

    let check = |order| {
        mref.with_manager_shared(|manager| {
            check_order(manager, order);
        });
    };

    check(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 18);
    });

    mref.with_manager_exclusive(|manager| {
        manager.reorder(|reorder_context| {
            oxidd_reorder::sifting_optimized(reorder_context);
        });
        print_order(manager);
    });

    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 12);
    });

    Ok(())
}

#[test]
fn sifting_seq_easy_test() -> AllocResult<()> {
    let mref: oxidd::bdd::BDDManagerRef = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(10));

    let f: BDDFunction = mref.with_manager_shared(|manager| {
        let x0: BDDFunction = BDDFunction::var(manager, 0)?;
        let x1: BDDFunction = BDDFunction::var(manager, 1)?;
        let x2: BDDFunction = BDDFunction::var(manager, 2)?;
        let x3: BDDFunction = BDDFunction::var(manager, 3)?;
        let x4: BDDFunction = BDDFunction::var(manager, 4)?;
        let x5: BDDFunction = BDDFunction::var(manager, 5)?;
        let x6: BDDFunction = BDDFunction::var(manager, 6)?;
        let x7: BDDFunction = BDDFunction::var(manager, 7)?;
        let x8: BDDFunction = BDDFunction::var(manager, 8)?;
        let x9: BDDFunction = BDDFunction::var(manager, 9)?;

        let clause2 = x0.or(&x4)?;
        let clause1 = x1.or(&x2)?;
        let clause3 = x3.or(&x5)?;
        let clause4 = x6.or(&x9)?;
        let clause5 = x7.or(&x8)?;

        let cnf = clause1
            .and(&clause2)?
            .and(&clause3)?
            .and(&clause4)?
            .and(&clause5)?;

        Ok(cnf)
    })?;

    let check = |order| {
        mref.with_manager_shared(|manager| {
            check_order(manager, order);
        });
    };

    check(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 18);
    });

    let vec: Vec<u32> = vec![1, 2, 3, 4, 0, 7, 6, 5, 8, 9];
    mref.with_manager_exclusive(|manager| {
        manager.reorder(|reorder_context| {
            oxidd_reorder::sifting_variable_sequence(reorder_context, &vec);
        });
        print_order(manager);
    });

    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 14);
    });

    Ok(())
}

#[test]
fn sifting_sequence_heuristic_easy_test() -> AllocResult<()> {
    let mref: oxidd::bdd::BDDManagerRef = oxidd::bdd::new_manager(1024, 128, 1);
    mref.with_manager_exclusive(|manager| manager.add_vars(10));

    let f: BDDFunction = mref.with_manager_shared(|manager| {
        let x0: BDDFunction = BDDFunction::var(manager, 0)?;
        let x1: BDDFunction = BDDFunction::var(manager, 1)?;
        let x2: BDDFunction = BDDFunction::var(manager, 2)?;
        let x3: BDDFunction = BDDFunction::var(manager, 3)?;
        let x4: BDDFunction = BDDFunction::var(manager, 4)?;
        let x5: BDDFunction = BDDFunction::var(manager, 5)?;
        let x6: BDDFunction = BDDFunction::var(manager, 6)?;
        let x7: BDDFunction = BDDFunction::var(manager, 7)?;
        let x8: BDDFunction = BDDFunction::var(manager, 8)?;
        let x9: BDDFunction = BDDFunction::var(manager, 9)?;

        let clause2 = x0.or(&x4)?;
        let clause1 = x1.or(&x2)?;
        let clause3 = x3.or(&x5)?;
        let clause4 = x6.or(&x9)?;
        let clause5 = x7.or(&x8)?;

        let cnf = clause1
            .and(&clause2)?
            .and(&clause3)?
            .and(&clause4)?
            .and(&clause5)?;

        Ok(cnf)
    })?;

    let check = |order| {
        mref.with_manager_shared(|manager| {
            check_order(manager, order);
        });
    };

    check(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 18);
    });

    mref.with_manager_exclusive(|manager| {
        manager.reorder(|reorder_context| {
            oxidd_reorder::sifting_heuristic_sequence(reorder_context, 20);
        });
        print_order(manager);
    });

    mref.with_manager_shared(|_manager| {
        assert_eq!(f.node_count(), 12);
    });

    Ok(())
}
