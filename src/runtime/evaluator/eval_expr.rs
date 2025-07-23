use log::info;

use crate::ast::{Assn, BinOp, Expr, Record, UnOp};
use std::{collections::HashMap, iter::zip, ops::Deref, vec};

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
        } else if let (Expr::String { val: val1 }, Expr::String { val: val2 }) = (expr1, expr2) {
            
            match op {
                BinOp::Eq => Ok(Expr::Bool { val: val1 == val2 }),
                _ => panic!(),
            }
        } else if let (Expr::Table {records: records1,.. }, Expr::Table { records: records2, .. }) = (expr1,expr2) {
            // println!("First table: {:?}", records1);
            // println!("Second table: {:?}", records2);
            match op {
                BinOp::Eq => Ok(Expr::Bool { val: records1 == records2 }),
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
                    (Number { .. }, Number { .. }) | (Bool { .. }, Bool { .. }) | (String { .. }, String { .. }) | (Table {..},Table{..}) => {
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

            Expr::Action { assns, inserts} => {
                // for assn in assns.iter_mut() {
                //     self.eval_assn(assn)?;
                // }
                Ok(())
            }
            Expr::Select {
                table_name,
                column_names,
                where_clause,
            } => {
                
                let table = self.search_table(table_name)?;
                //info!("Table found is: {}", table);

                let original_context = self.reactive_name_to_vals.clone();

                if let Expr::Table {schema, records , ..} = table {
                    let mut selected_schema = Vec::new();
                    // Build new schema for selected columns
                    if column_names.len()!=0 {   
                    selected_schema = schema.iter()
                        .filter(|field| column_names.contains(&field.name))
                        .cloned()
                        .collect();
                    } else {              // if no column names mentioned, select all columns
                        selected_schema = schema.clone();
                    }
                    let mut selected_records = Vec::new();

                    for record in records {
                        for(field, value) in schema.iter().zip(record.val.iter()) {
                            self.reactive_name_to_vals.insert(field.name.clone(), value.clone());
                        }
                        let mut evaluated_where = where_clause.deref().clone();
                        // evaluate where condition with the value from current record
                        // for example, select age from sample where age > 21;
                        // before evaluating where, we're updating context for each record
                        // so right now, the status is age: 18 (let's say first record's age is 18) 
                        if let Err(e) = self.eval_expr(&mut evaluated_where) {
                            info!("Error while evaluating where: {}", e);
                            return Err(e);
                        }
                        if let Expr::Bool { val } = *evaluated_where {        // if the condition satisfies for that record, add that record
                            if val {
                                // Build new record with only selected columns
                                let mut new_vals = Vec::new();
                                if column_names.len()==0 {       // if no columns mentioned, add all
                                    selected_records.push(record.clone());
                                }
                                else {
                                    for col in &mut *column_names {
                                    if let Some((i, _)) = schema.iter().enumerate().find(|(_, f)| &f.name == col) {
                                        new_vals.push(record.val[i].clone());
                                    } else {
                                        return Err(format!("Column '{}' not found in schema", col));
                                    }
                                }
                                selected_records.push(Record { val: new_vals });
                            }
                            }
                        } else {
                            self.reactive_name_to_vals = original_context;
                            return Err(format!("Where must evaluate to boolean"));
                        }
                    }
                    self.reactive_name_to_vals = original_context;
                    *expr = Expr::Table {
                        schema: selected_schema,
                        records: selected_records,
                    };
                    info!("Select result: {}", *expr);
                }
                Ok(())

            }
            Expr::Table { .. } => Ok(()),
            Expr::Rows { val } => {
                let mut vals = Vec::new();

                for row in val {
                    vals.push(Record{val: row.val.iter().map(|entry| entry.val.clone()).collect()});
                }
                *expr = Expr::Table {                 // return a table expression with empty schema and extracted records, may just pass it as rows and handle equality above
                    schema: Vec::new(),
                    records: vals
                };
                Ok(())
            },

            Expr::TableColumn { table_name, column_name } => {
                info!("Eval tablecolumn");
                if let Some(val) = self.reactive_name_to_vals.get(column_name) {   // get the value from the column name in the context which was added in the select eval
                    *expr = val.clone();
                    // println!("{}",val.clone());
                
                    return Ok(());
                } 
                Err(format!("TableColumn {}.{} cannot be evaluated outside of a row context", table_name, column_name))
            }            // will remove tablecolumn if no use 
            Expr::Fold { args } => {
                if let Expr::TableColumn { table_name, column_name } = &args[0] {
                    let table = self.search_table(table_name)?;
                    if let Expr::Table { schema, records } = table {
                        let column_id = schema.iter().position(|f| &f.name == column_name).ok_or_else(|| format!("Column not found"))?;
                        let mut accum = args[2].clone();
                            for record in records {
                                let val = record.val[column_id].clone();
                                let mut result = self.fold(val, accum, args[1].clone());
                                self.eval_expr(&mut result)?;
                                accum = result;     // recursive since this accum with result value is used in the next iteration as identity var
                                
                            }
                            // println!("Evaled result: {}",  accum);
                            *expr = accum;
                            
                            Ok(())
                    } else {
                        Err(format!("{} table not found", table_name))
                    }
                } else {
                    Err(format!("First arg should be a iterator (column for now)"))
                }
            }
        }
    }
}

