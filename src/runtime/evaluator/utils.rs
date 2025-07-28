use std::collections::{HashMap, HashSet};

use crate::ast::Expr;

use super::{Evaluator, Val};

impl Evaluator {
    pub fn gen_var(&mut self) -> String {
        self.var_id_cnt += 1;
        let var = format!("x{}", self.var_id_cnt);
        var
    }

    pub fn fold(&mut self, val: Expr, identity: Expr, operation: Expr) -> Expr {
        let func_apply = Expr::FuncApply { func: Box::new(operation), args: vec![identity, val] };
        self.eval_expr(&mut func_apply.clone());
        func_apply
    }

    pub fn search_table(&mut self, name: &String) -> Result<Expr, String> {
        if let Some(table) = self.reactive_name_to_vals.get(name).cloned() {
            Ok(table)
        }else {
            return Err(format!("Table {} not found", name));
        }
    }

    /// subst all variables in expr if exists in var_to_expr
    pub fn subst(&mut self, expr: &mut Expr, var_to_expr: &HashMap<String, Expr>) {
        match expr {
            Expr::Number { val } => {}
            Expr::Bool { val } => {}
            Expr::String {val} => {}
            Expr::Vector { val } => {
                for expr in val {
                    self.subst(expr, var_to_expr);
                }
            }
            Expr::Variable { ident } => {
                if let Some(e) = var_to_expr.get(ident) {
                    *expr = e.clone();
                }
            }
            Expr::KeyVal { key, value } => {
                self.subst(value, var_to_expr);
            }
            Expr::Unop { op, expr } => {
                self.subst(expr, var_to_expr);
            }
            Expr::Binop { op, expr1, expr2 } => {
                self.subst(expr1, var_to_expr);
                self.subst(expr2, var_to_expr);
            }
            Expr::If { cond, expr1, expr2 } => {
                self.subst(cond, var_to_expr);
                self.subst(expr1, var_to_expr);
                self.subst(expr2, var_to_expr);
            }
            Expr::Func { params, body } => {
                // assume we want to subst x with m
                let param_set: HashSet<String> = params.clone().into_iter().collect();
                for (x, m) in var_to_expr.iter() {
                    // - if x in params, do nothing since its binded by function
                    if param_set.contains(x) {
                        continue;
                    }

                    // - if params contain free var in M, alpha-rename such params
                    // expr to avoid "capture"
                    let free_var_in_m = m.free_var(&self.reactive_names,&self.reactive_names);

                    let mut renames = HashMap::new();
                    for param in params.iter_mut() {
                        if free_var_in_m.contains(param) {
                            let new_var = self.gen_var();
                            renames.insert(param.clone(), new_var.clone());
                            *param = new_var;
                        }
                    }
                    body.alpha_rename(&self.reactive_names, &renames);
                }
                // now it's safe to substitute
                self.subst(body, var_to_expr);
            }

            Expr::FuncApply { func, args } => {
                self.subst(func, var_to_expr);
                for arg in args {
                    self.subst(arg, var_to_expr);
                }
            }
            Expr::Action { assns, inserts } => {
                for assn in assns {
                    // dest should not be substituted, only src should
                    self.subst(&mut assn.src, var_to_expr);
                }
                for insert in inserts {
                    self.subst(&mut insert.row, var_to_expr);
                }
            }
            Expr::Select { table_name, column_names, where_clause } => todo!(),
            Expr::TableColumn { table_name, column_name } => todo!(),
            Expr::Table { .. } => todo!(),
            Expr::Rows { val } => {
                for row in val {
                    self.subst(row, var_to_expr);
                }
            },
            Expr::Fold { args } => todo!()
        }
    }
}
