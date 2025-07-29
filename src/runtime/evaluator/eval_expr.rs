use crate::ast::{Assn, BinOp, Expr, UnOp};
use std::{collections::HashMap, iter::zip, ops::Deref};

use super::{Evaluator, Val};

fn is_versioned(name: &str) -> bool {
    name.chars().last().map_or(false, |c| c.is_ascii_digit())
}


/// Keeps track of the version history of all variables/definitions
#[derive(Debug, Default, Clone)]
pub struct VersionMap {
    map: HashMap<String, usize>,
}

impl VersionMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get_base(&self, versioned: &str) -> String {
        versioned.chars()
            .take_while(|c| !c.is_numeric())
            .collect()
    }


    pub fn get_latest(&self, base: &str) -> String {
        if self.map.contains_key(base) {
            format!("{base}{}", self.map[base])
        } else {
        // Check if the name is already versioned like "x0"
        // If so, return it as-is
            if base.chars().last().unwrap_or('a').is_numeric() {
                return base.to_string();
            }

            format!("{base}0")
        }
    }


    /// Increment version and return new name
    pub fn next_version(&mut self, base: &str) -> String {
        let entry = self.map.entry(base.to_string()).or_insert(0);
        *entry += 1;
        format!("{base}{entry}")
    }

    /// Reset all versions
    pub fn reset(&mut self) {
        self.map.clear();
    }

    pub fn declare(&mut self, base: &str) -> String {
        self.map.entry(base.to_string()).or_insert(0);
        self.get_latest(base)
    }

}

pub fn rewrite_expr(expr: &mut Expr, version_map: &VersionMap) {
    match expr {
        Expr::Variable { ident } => {
            if !is_versioned(ident) {
                *ident = version_map.get_latest(ident);
            }
        }
        Expr::Binop { expr1, expr2, .. } => {
            rewrite_expr(expr1, version_map);
            rewrite_expr(expr2, version_map);
        }
        Expr::Unop { expr, .. } => {
            rewrite_expr(expr, version_map);
        }
        Expr::Func { body, .. } => {
            rewrite_expr(body, version_map);
        }
        Expr::FuncApply { func, args } => {
            rewrite_expr(func, version_map);
            for arg in args {
                rewrite_expr(arg, version_map);
            }
        }
        Expr::Action { assns } => {
            for assn in assns {
                rewrite_expr(&mut assn.src, version_map);

                if !is_versioned(&assn.dest) {
                    assn.dest = version_map.get_latest(&assn.dest);
                }

                println!("After rewrite: dest = {}", assn.dest);
            }
        }
        _ => {}
    }
}


impl Evaluator {

    pub fn calc_unop(op: UnOp, expr: &Expr) -> Result<Expr, String> {
        if let Expr::Number { val } = expr {
            match op {
                UnOp::Neg => Ok(Expr::Number { val: -val }),
                _ => panic!(),
            }
        } else if let Expr::Bool { val } = expr {
            match op {
                UnOp::Not => Ok(Expr::Bool { val: !val }),
                _ => panic!(),
            }
        } else {
            Err(format!(
                "calculate unop expression cannot be 
                applied to {}",
                *expr
            ))
        }
    }

    pub fn calc_binop(op: BinOp, expr1: &Expr, expr2: &Expr) -> Result<Expr, String> {
        if let (Expr::Number { val: val1 }, Expr::Number { val: val2 }) = (expr1, expr2) {
            let (val1, val2) = (*val1, *val2);
            match op {
                BinOp::Add => Ok(Expr::Number { val: val1 + val2 }),
                BinOp::Sub => Ok(Expr::Number { val: val1 - val2 }),
                BinOp::Mul => Ok(Expr::Number { val: val1 * val2 }),
                BinOp::Div => Ok(Expr::Number { val: val1 / val2 }),
                BinOp::Eq => Ok(Expr::Bool { val: val1 == val2 }),
                BinOp::Lt => Ok(Expr::Bool { val: val1 < val2 }),
                BinOp::Gt => Ok(Expr::Bool { val: val1 > val2 }),
                _ => panic!(),
            }
        } else if let (Expr::Bool { val: val1 }, Expr::Bool { val: val2 }) = (expr1, expr2) {
            let (val1, val2) = (*val1, *val2);
            match op {
                BinOp::And => Ok(Expr::Bool { val: val1 && val2 }),
                BinOp::Or => Ok(Expr::Bool { val: val1 || val2 }),
                _ => panic!(),
            }
        } else {
            Err(format!(
                "calculate binop expression cannot be applied 
                on {} {:?} {}",
                *expr1, op, *expr2
            ))
        }
    }

    /// inplace evaluator of Expr
    /// todo: change implementation to context stack,
    /// - better performance
    /// - not necessary to mut self
    pub fn eval_expr(&mut self, expr: &mut Expr) -> Result<(), String> {
        match expr {
            Expr::Number { val } => Ok(()),
            Expr::Bool { val } => Ok(()),
            Expr::Variable { ident } => {
                let val = self
                    .reactive_name_to_vals
                    .get(ident)
                    .cloned()
                    .ok_or_else(|| format!("variable '{}' not found", ident));

                val.map(|val| *expr = val)
            }

            Expr::Unop { op, expr: expr1 } => {
                self.eval_expr(expr1)?;
                match expr1.as_mut() {
                    // note: as_mut() has same effect as &mut **expr1 here
                    Expr::Number { .. } | Expr::Bool { .. } => {
                        *expr = Self::calc_unop(*op, expr1)?;
                        Ok(())
                    }
                    _ => Err(format!(
                        "unary operator {:?} cannot be applied to 
                        {}",
                        op, **expr1
                    )),
                }
            }

            Expr::Binop { op, expr1, expr2 } => {
                self.eval_expr(expr1)?;
                self.eval_expr(expr2)?;
                use Expr::*;
                match (expr1.as_mut(), expr2.as_mut()) {
                    (Number { .. }, Number { .. }) | (Bool { .. }, Bool { .. }) => {
                        *expr = Self::calc_binop(*op, expr1, expr2)?;
                        Ok(())
                    }
                    _ => Err(format!(
                        "binary operator {:?} cannot be applied to 
                        {} and {}",
                        op, **expr1, **expr2
                    )),
                }
            }

            Expr::If { cond, expr1, expr2 } => {
                self.eval_expr(cond)?;
                match **cond {
                    Expr::Bool { val } => {
                        let new_expr = if val {
                            std::mem::take(expr1)
                        } else {
                            std::mem::take(expr2)
                        };
                        *expr = *new_expr;
                        self.eval_expr(expr)
                    }
                    _ => Err(format!("if condition must be a boolean, got {}", **cond)),
                }
            }

            Expr::Func { params, body } => {
                // functions are values
                Ok(())
            }

            Expr::FuncApply { func, args } => {
                self.eval_expr(func)?;

                match func.as_mut() {
                    Expr::Func { params, body } => {
                        if params.len() != args.len() {
                            Err(format!(
                                "function expects {} arguments, got {}",
                                params.len(),
                                args.len()
                            ))
                        } else {
                            // to correctly evaluate shadowing and avoid capture,
                            // there are two ways to implement:
                            // 1. functional: by immediate substitution
                            // 2. imperative: by maintaining a stack of environments

                            for arg in args.iter_mut() {
                                self.eval_expr(arg)?;
                            }

                            let var_to_expr = zip(params, args)
                                .map(|(arg, val)| (arg.clone(), val.clone()))
                                .collect::<HashMap<String, Expr>>();

                            self.subst(body, &var_to_expr);

                            *expr = std::mem::take(body);
                            self.eval_expr(expr)
                        }
                    }
                    _ => Err(format!("cannot apply non-function")),
                }
            }

            Expr::Action { assns } => {
                Ok(())
            }
        }
    }
}
