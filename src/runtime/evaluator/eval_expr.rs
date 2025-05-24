
use std::{collections::HashMap, iter::zip};
use crate::ast::{Assn, BinOp, Expr, UnOp};

use super::{Evaluator, Val};

impl Evaluator {
    pub fn eval_expr(&mut self, expr: &mut Expr) -> Result<Val, String> {
        match expr {
            Expr::Number { val } => Ok(Val::Number(*val)),
            Expr::Bool { val } => Ok(Val::Bool(*val)),
            Expr::Variable { ident } => self.env.get(ident).cloned().ok_or_else(|| format!("variable '{}' not found", ident)),
            Expr::Unop { op, expr } => {
                let val = &self.eval_expr(expr)?;
                match op {
                    UnOp::Neg => match val {
                        Val::Number(i) => Ok(Val::Number(-i)),
                        _ => Err(format!("unary operator {:?} cannot be applied to {}", op, val)),
                    },
                    UnOp::Not => match val {
                        Val::Bool(b) => Ok(Val::Bool(!b)),
                        _ => Err(format!("unary operator {:?} cannot be applied to {}", op, val)),
                    },
                }
            },
            Expr::Binop { op, expr1, expr2 } => {
                let val1 = &self.eval_expr(expr1)?;
                let val2 = &self.eval_expr(expr2)?;
                match op {
                    BinOp::Add => match (val1, val2) {
                        (Val::Number(i1), Val::Number(i2)) => Ok(Val::Number(i1 + i2)),
                        _ => Err(format!("binary operator {:?} cannot be applied to {} and {}", op, val1, val2)),
                    },
                    BinOp::Sub => match (val1, val2) {
                        (Val::Number(i1), Val::Number(i2)) => Ok(Val::Number(i1 - i2)),
                        _ => Err(format!("binary operator {:?} cannot be applied to {} and {}", op, val1, val2)),
                    },
                    BinOp::Mul => match (val1, val2) {
                        (Val::Number(i1), Val::Number(i2)) => Ok(Val::Number(i1 * i2)),
                        _ => Err(format!("binary operator {:?} cannot be applied to {} and {}", op, val1, val2)),
                    },
                    BinOp::Div => match (val1, val2) {
                        (Val::Number(i1), Val::Number(i2)) => Ok(Val::Number(i1 / i2)),
                        _ => Err(format!("binary operator {:?} cannot be applied to {} and {}", op, val1, val2)),
                    },
                    BinOp::Eq => match (val1, val2) {
                        (Val::Number(i1), Val::Number(i2)) => Ok(Val::Bool(i1 == i2)),
                        (Val::Bool(b1), Val::Bool(b2)) => Ok(Val::Bool(b1 == b2)),
                        _ => Err(format!("binary operator {:?} cannot be applied to {} and {}", op, val1, val2)),
                    },
                    BinOp::Gt => match (val1, val2) {
                        (Val::Number(i1), Val::Number(i2)) => Ok(Val::Bool(i1 > i2)),
                        _ => Err(format!("binary operator {:?} cannot be applied to {} and {}", op, val1, val2)),
                    },
                    BinOp::Lt => match (val1, val2) {
                        (Val::Number(i1), Val::Number(i2)) => Ok(Val::Bool(i1 < i2)),
                        _ => Err(format!("binary operator {:?} cannot be applied to {} and {}", op, val1, val2)),  
                    },
                    BinOp::And => match (val1, val2) {
                        (Val::Bool(b1), Val::Bool(b2)) => Ok(Val::Bool(*b1 && *b2)),
                        _ => Err(format!("binary operator {:?} cannot be applied to {} and {}", op, val1, val2)),
                    },
                    BinOp::Or => match (val1, val2) {
                        (Val::Bool(b1), Val::Bool(b2)) => Ok(Val::Bool(*b1 || *b2)),
                        _ => Err(format!("binary operator {:?} cannot be applied to {} and {}", op, val1, val2)),
                    },

                }
            },

            Expr::If { cond, expr1, expr2 } => {
                let val = &self.eval_expr(cond)?;
                match val {
                    Val::Bool(b) => if *b { self.eval_expr(expr1) } else { self.eval_expr(expr2) },
                    _ => Err(format!("if condition must be a boolean, got {}", val)),
                }
            },

            Expr::Func { params, body } => {
                Ok(Val::Func(params.clone(), body.clone()))
            },

            Expr::FuncApply { func, args } => {
                let func_val = self.eval_expr(func)?;

                match func_val {
                    Val::Func(params, mut body) => {
                        if params.len() != args.len() {
                            Err(format!("function expects {} arguments, got {}", 
                                params.len(), args.len()))
                        } else {
                            // to correctly evaluate shadowing and avoid capture, 
                            // there are two ways to implement: 
                            // 1. functional: by immediate substitution 
                            // 2. imperative: by maintaining a stack of environments

                            let arg_vals = args.iter_mut()
                                .map(|arg| self.eval_expr(arg))
                                .collect::<Result<Vec<Val>, String>>()?;

                            let var_to_expr= zip(params, arg_vals)
                                .map(|(arg, val)| (arg.clone(), val))
                                .collect::<HashMap<String, Val>>();

                            self.subst(&mut body, &var_to_expr);
                            
                            self.eval_expr(&mut body)
                        }
                    },
                    _ => Err(format!("cannot apply non-function {}", func_val)),
                }
            },

            Expr::Action { assns } => {
                for assn in assns.iter_mut() {
                    self.eval_assn(assn)?;
                }
                Ok(Val::Action(assns.clone()))
            }, 
        }
    }
}