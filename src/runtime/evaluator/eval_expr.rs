use crate::ast::{Assn, BinOp, Expr, UnOp};
use std::{collections::HashMap, iter::zip, ops::Deref};

use super::{Evaluator, Val};

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
            Expr::String {val} => Ok(()),
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

            Expr::Action { assns , inserts} => {
                // for assn in assns.iter_mut() {
                //     self.eval_assn(assn)?;
                // }
                Ok(())
            }
            Expr::Select { table_name, where_clause } => {  

                let Some(table_data) = self.table_name_to_data.get(table_name).cloned() else {
                    return Err(format!("Table {} data not found", table_name));
                };

                let mut selected_rows = Vec::new();

                let original_reactive_name_to_vals = self.reactive_name_to_vals.clone();  // making a copy of the original

                for row in table_data.iter() {
                    for entry in row.val.iter() {
                        self.reactive_name_to_vals.insert(entry.name.clone(), entry.val.clone()); // adding every entry's name (column name) and value (expression) as context for evaluating that row  
                    }

                    let mut evaluated_where = where_clause.deref().clone();    // where clause expression
                    self.eval_expr(&mut evaluated_where)?;     // evaluating where clause with the help of all entries' context in that row (goes over to TableColumn eval below)

                    if let Expr::Bool { val } = *evaluated_where {
                        if val {
                            selected_rows.push(row.clone());     // if condition comes out to true, push that entire row into the vector
                        }
                    } else {
                        self.reactive_name_to_vals = original_reactive_name_to_vals.clone();   // if condition is not bool, return to original context
                        return Err(format!("Where must evaluate to boolean"));
                    }

                    for entry in row.val.iter() {
                        self.reactive_name_to_vals.remove(&entry.name);   // after a row is evaluated, remove all added context so there's no conflicts with the next row's values
                    }
                }

                self.reactive_name_to_vals = original_reactive_name_to_vals;   // return to original context after evaluating all rows

                *expr = Expr::Table { name: table_name.to_string(), rows: selected_rows };   // return table with the rows which passed the check

                println!("Select result: {}", *expr);
              
                Ok(())

            }
            Expr::TableColumn { table_name, column_name } => {
                if let Some(val) = self.reactive_name_to_vals.get(column_name) {   // get the value from the column name in the context which was added in the select eval
                    *expr = val.clone();

                    return Ok(());
                }
                Err(format!("TableColumn {}.{} cannot be evaluated outside of a row context", table_name, column_name))
            },
            Expr::Table { name, rows } => Ok(()),
        }
    }
}

/* TODO:
1. Select eval is inefficient, may look into context stack
*/ 
