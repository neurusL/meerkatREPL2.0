use std::collections::{HashMap, HashSet};

use super::Expr;

impl Expr {
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

    /// alpha renaming of expression e
    /// rename x1, x2, ..., x_n to y1, y2, ..., y_n if x is free in expresion e
    pub fn alpha_rename(&mut self, 
        var_binded: &HashSet<String>, 
        renames: &HashMap<String, String>
    ) {
        match self {
            Expr::Number { .. } |
            Expr::Bool { .. } => {},
            Expr::Variable { ident } => {
                if !var_binded.contains(ident) && renames.contains_key(ident) {
                    *ident = renames.get(ident).unwrap().clone();
                }
            },
            Expr::Unop { op, expr } => {
                expr.alpha_rename(var_binded, renames);
            },
            Expr::Binop { op, expr1, expr2 } => {
                expr1.alpha_rename(var_binded, renames);
                expr2.alpha_rename(var_binded, renames);
            },
            Expr::If { cond, expr1, expr2 } => {
                cond.alpha_rename(var_binded, renames);
                expr1.alpha_rename(var_binded, renames);
                expr2.alpha_rename(var_binded, renames);
            },
            Expr::Func { params, body } => {
                let mut new_binds = var_binded.clone();
                new_binds.extend(params.iter().cloned());
                body.alpha_rename(&new_binds, renames);
            },
            Expr::FuncApply { func, args } => {
                func.alpha_rename(var_binded, renames);
                for arg in args {
                    arg.alpha_rename(var_binded, renames);
                }
            },

            Expr::Action { assns } => {
                for assn in assns {
                    // dest should never be renamed, not influenced by capture
                    // let dest = &mut assn.dest;
                    // if !var_binded.contains(dest) && renames.contains_key(dest){
                    //     *dest = renames.get(dest).unwrap().clone();
                    // }
                    assn.src.alpha_rename(var_binded, renames);
                }
            },
        }
    }
}