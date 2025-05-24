
use std::{collections::HashMap};
use crate::ast::{Expr, UnOp, BinOp};

use super::Val;

pub fn eval(expr: &Expr, env: &HashMap<String, Val>) -> Result<Val, String> {
    match expr {
        Expr::Number { val } => Ok(Val::Number(*val)),
        Expr::Bool { val } => Ok(Val::Bool(*val)),
        Expr::Variable { ident } => env.get(ident).cloned().ok_or_else(|| format!("variable '{}' not found", ident)),
        Expr::Unop { op, expr } => {
            let val = &eval(expr, env)?;
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
            let val1 = &eval(expr1, env)?;
            let val2 = &eval(expr2, env)?;
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
            let val = &eval(cond, env)?;
            match val {
                Val::Bool(b) => if *b { eval(expr1, env) } else { eval(expr2, env) },
                _ => Err(format!("if condition must be a boolean, got {}", val)),
            }
        },
        Expr::Func { params, body } => {
            Ok(Val::Func(params.clone(), body.clone()))
        },
        Expr::FuncApply { func, args } => {
            let val = &eval(func, env)?;

            // match val {
            //     Val::Func(params, body) => {
            //         if params.len() != args.len() { Err("")} 
            //else {
            //             let mut new_env = env.clone();
            //             for (param, arg) in params.iter().zip(args.iter()) {
            //                 let val = &eval(arg, &env)?;
            // X -->           new_env.insert(param.clone(), val.clone());
            //             }
            //             eval(body, &new_env)
            //         }
            //     },
            //     _ => Err(format!("cannot apply non-function {}", val)),
            // }

            // above is a false implementation of function application, which(X)
            // incorrectly handles shadowing of variables

            todo!()
            
        },
        Expr::Action { assns } => todo!(),
    }
}
