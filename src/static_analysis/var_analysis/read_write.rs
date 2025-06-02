
use std::{collections::HashSet, hash::Hash};

use crate::ast::{Assn, Expr};

impl Expr {
    /// return free variables in expr wrt var_binded, used in eval
    /// can also be used as variables read by the expression when var_binded set to empty
    pub fn free_var(&self, 
        var_binded: &HashSet<String>, // should be initialized by all reactive name declared in the service
    ) -> HashSet<String> {
        match self {
            Expr::Number { .. } |
            Expr::Bool { .. } => HashSet::new(),
            Expr::Variable { ident } => {
                if var_binded.contains(ident) { HashSet::new() }
                else { HashSet::from([ident.clone()]) }
            }
            Expr::Unop { op, expr } => {
                expr.free_var(var_binded)
            },
            Expr::Binop { op, expr1, expr2 } => {
                let mut free_vars = expr1.free_var(var_binded);
                free_vars.extend(expr2.free_var(var_binded));
                free_vars
            },
            Expr::If { cond, expr1, expr2 } => {
                let mut free_vars = cond.free_var(var_binded);
                free_vars.extend(expr1.free_var(var_binded));
                free_vars.extend(expr2.free_var(var_binded));
                free_vars
            },
            Expr::Func { params, body } => {
                let mut new_binds = var_binded.clone();
                new_binds.extend(params.iter().cloned());
                body.free_var(&new_binds)
            },
            Expr::FuncApply { func, args } => {
                let mut free_vars = func.free_var(var_binded);
                for arg in args {
                    free_vars.extend(arg.free_var(var_binded));
                }
                free_vars
            },

            // x in FV(l) => x in FV(action { ..., l = r, ...})
            // x in FV(r) => x in FV(action { ..., l = r, ...})
            Expr::Action { assns } => {
                let mut free_vars = HashSet::new();
                for assn in assns {
                    // dest should never be free, each dest should be declared before use 
                    // if var_binded.contains(&assn.dest) { 
                    //     free_vars.insert(assn.dest.clone());
                    // }
                    free_vars.extend(assn.src.free_var(var_binded));
                }
                free_vars
            },
        }
    }
}

pub fn calc_read_set(assns: &Vec<Assn>) -> HashSet<String> {
    let mut reads = HashSet::new();
    for assn in assns {
        reads.extend(assn.src.free_var(&HashSet::new()));
    }
    reads
}

pub fn calc_write_set(assns: &Vec<Assn>) -> HashSet<String> {
    let mut writes = HashSet::new();
    for assn in assns {
        writes.insert(assn.dest.clone());
    }
    writes
}