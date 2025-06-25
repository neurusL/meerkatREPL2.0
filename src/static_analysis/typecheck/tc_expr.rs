use core::panic;

use super::{Type, TypecheckEnv};
use crate::ast::*;

impl TypecheckEnv {
    pub fn infer_expr(&mut self, expr: &Expr) -> Type {
        use Type::*;
        match expr {
            Expr::Number { val: _ } => Int,
            Expr::Bool { val: _ } => Bool,
            Expr::String {val: _} => String,
            Expr::Variable { ident } => self
                .var_context
                .get(ident)
                .cloned()
                .expect(&format!("cannot find var {:?} in context", ident)),

            Expr::Unop { op, expr } => match op {
                UnOp::Neg => {
                    let typ = self.infer_expr(expr);
                    if self.unify(&typ, &Int) {
                        Int
                    } else {
                        panic!("cannot unify {:?} and int", typ)
                    }
                }
                UnOp::Not => {
                    let typ = self.infer_expr(expr);
                    if self.unify(&typ, &Bool) {
                        Bool
                    } else {
                        panic!("cannot unify {:?} and bool", typ)
                    }
                }
            },

            Expr::Binop { op, expr1, expr2 } => match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    let typ1 = self.infer_expr(expr1);
                    let typ2 = self.infer_expr(expr2);
                    if !self.unify(&typ1, &Int) {
                        panic!("cannot unify left hand side {:?} and int", typ1)
                    } else if !self.unify(&typ2, &Int) {
                        panic!("cannot unify right hand side {:?} and int", typ2)
                    } else {
                        Int
                    }
                }
                BinOp::Lt | BinOp::Gt => {
                    let typ1 = self.infer_expr(expr1);
                    let typ2 = self.infer_expr(expr2);
                    if !self.unify(&typ1, &Int) {
                        panic!("cannot unify left hand side {:?} and int", typ1)
                    } else if !self.unify(&typ2, &Int) {
                        panic!("cannot unify right hand side {:?} and int", typ2)
                    } else {
                        Bool
                    }
                }

                BinOp::And | BinOp::Or => {
                    let typ1 = self.infer_expr(expr1);
                    let typ2 = self.infer_expr(expr2);
                    if !self.unify(&typ1, &Bool) {
                        panic!("cannot unify left hand side {:?} and bool", typ1)
                    } else if !self.unify(&typ2, &Bool) {
                        panic!("cannot unify right hand side {:?} and bool", typ2)
                    } else {
                        Bool
                    }
                }

                BinOp::Eq => {
                    let typ1 = self.infer_expr(expr1);
                    let typ2 = self.infer_expr(expr2);
                    if !self.unify(&typ1, &typ2) {
                        panic!("cannot unify {:?} and {:?}", typ1, typ2)
                    } else {
                        Bool
                    }
                }
            },

            Expr::If { cond, expr1, expr2 } => {
                let cond_typ = self.infer_expr(cond);
                if !self.unify(&cond_typ, &Bool) {
                    panic!("cannot unify {:?} and bool", cond_typ);
                }
                let typ1 = self.infer_expr(expr1);
                let typ2: Type = self.infer_expr(expr2);
                if !self.unify(&typ1, &typ2) {
                    panic!("cannot unify {:?} and {:?}", typ1, typ2);
                }

                assert!(self.find(&typ1) == self.find(&typ2));
                self.find(&typ1)
            }

            Expr::Func { params, body } => {
                // check params are unique
                let mut param_set = std::collections::HashSet::new();
                for param in params.iter() {
                    if !param_set.insert(param) {
                        panic!("duplicate param name: {}", param);
                    }
                }

                // frozen current context
                let old_context = self.var_context.clone();

                // first generate type variables for param, update context
                let mut param_types = vec![];
                for param in params.iter() {
                    let typ = self.gen_typevar();
                    self.var_context.insert(param.clone(), typ.clone());
                    param_types.push(typ);
                }

                // type infer func body mutates acc_subst
                let mut ret_typ = self.infer_expr(&body);

                // generate function type signature of canonical form
                for param_typ in param_types.iter_mut() {
                    *param_typ = self.find(param_typ);
                }
                ret_typ = self.find(&ret_typ);

                // restore old context
                self.var_context = old_context;

                Fun(param_types, Box::new(ret_typ))
            }

            Expr::FuncApply { func, args } => {
                let func_typ = self.infer_expr(func);
                if let Type::Fun(arg_typs, ret_typ) = func_typ {
                    if arg_typs.len() != args.len() {
                        panic!("wrong number of argument to apply");
                    } else {
                        for (i, arg) in args.iter().enumerate() {
                            let typ_i: &Type = &arg_typs[i];
                            let typ_i_actual = self.infer_expr(arg);
                            if !self.unify(typ_i, &typ_i_actual) {
                                panic!(
                                    "cannot unify {:?}th argument, 
                                    expect {:?} got {:?}",
                                    i, typ_i, typ_i_actual
                                )
                            }
                        }
                        self.find(&ret_typ)
                    }
                } else {
                    let ret_typ = self.gen_typevar();
                    let arg_typs = args.iter().map(|a| self.infer_expr(a)).collect();

                    let func_typ_actual = Type::Fun(arg_typs, Box::new(ret_typ.clone()));

                    if !self.unify(&func_typ, &func_typ_actual) {
                        panic!(
                            "cannot unify function type, expected {:?} got {:?}",
                            func_typ, func_typ_actual
                        );
                    }

                    self.find(&ret_typ)
                }
            }

            // more todo on Action type
            Expr::Action { assns , inserts} => {
                assns.iter().for_each(|assn| self.typecheck_assn(assn));
                inserts.iter().for_each(|insert| self.typecheck_insert(insert));

                Action
            }
            Expr::Select { table_name, where_clause } => {
                let table = self.table_context.get(table_name).unwrap_or_else(|| panic!("Table {} for selection not found",table_name));
                
                self.typecheck_column_access(where_clause, table_name, table);

                let cond_type = self.infer_expr(where_clause);
                if cond_type != Type::Bool {
                    panic!("Select where clause must be boolean, got {}", cond_type);
                }
                
                Table
            }
            Expr::TableColumn { table_name, column_name } => {
                let found_table = self.table_context.get(table_name);

                match found_table {
                    None => panic!("Table {} for selection not found", table_name),
                    Some(table) =>{
                        let found_column = table.iter().find(|column| column.name == *column_name);
                        match found_column {
                            None => panic!("Column {} not found in {}", column_name, table_name),
                            Some(column) => {
                                match column.type_ {
                                    DataType::Bool => Type::Bool,
                                    DataType::Number => Type::Int,
                                    DataType::String => Type::String,
                                }
                            }
                        }
                    
                    }
                }
            }
            Expr::Table { rows } => {
                Table
            }
        }
}
}

// TODO List
// (priority) implement statics for actions
// 1. more efficient implementation of var context
// 2. add language feature: let binding
