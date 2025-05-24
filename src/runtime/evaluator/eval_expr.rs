
use std::{collections::HashMap, iter::zip};
use crate::ast::{Assn, BinOp, Expr, UnOp};

use super::{Evaluator, Val};

impl Evaluator {
    pub fn eval(&mut self, expr: &mut Expr, env: &HashMap<String, Val>) -> Result<Val, String> {
        match expr {
            Expr::Number { val } => Ok(Val::Number(*val)),
            Expr::Bool { val } => Ok(Val::Bool(*val)),
            Expr::Variable { ident } => env.get(ident).cloned().ok_or_else(|| format!("variable '{}' not found", ident)),
            Expr::Unop { op, expr } => {
                let val = &self.eval(expr, env)?;
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
                let val1 = &self.eval(expr1, env)?;
                let val2 = &self.eval(expr2, env)?;
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
                let val = &self.eval(cond, env)?;
                match val {
                    Val::Bool(b) => if *b { self.eval(expr1, env) } else { self.eval(expr2, env) },
                    _ => Err(format!("if condition must be a boolean, got {}", val)),
                }
            },

            Expr::Func { params, body } => {
                Ok(Val::Func(params.clone(), body.clone()))
            },

            Expr::FuncApply { func, args } => {
                let func_val = self.eval(func, env)?;

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
                                .map(|arg| self.eval(arg, env))
                                .collect::<Result<Vec<Val>, String>>()?;

                            let var_to_expr= zip(params, arg_vals)
                                .map(|(arg, val)| (arg.clone(), val))
                                .collect::<HashMap<String, Val>>();

                            self.subst(&mut body, &var_to_expr);
                            
                            self.eval(&mut body, env)
                        }
                    },
                    _ => Err(format!("cannot apply non-function {}", func_val)),
                }
            },

            Expr::Action { assns } => {
                let mut assn_vals = vec![];
                for assn in assns {
                    let val = self.eval(&mut assn.src, env)?;
                    assn_vals.push( Assn { 
                        dest: assn.dest.clone(), 
                        src: Expr::from(val) 
                    });
                }
                Ok(Val::Action(assn_vals))
            }, 
        }
    }
}