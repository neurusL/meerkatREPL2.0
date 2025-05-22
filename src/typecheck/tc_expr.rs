use core::panic;

use super::{Type, TypecheckEnv};
use crate::ast::*;

impl TypecheckEnv {
    pub fn type_infer(&mut self, expr: &Expr) -> Type {
        use Type::*;
        match expr {
            Expr::Number { val: _ } => Int,
            Expr::Bool { val: _ } => Bool,
            Expr::Variable { ident } => {
                self.var_context.get(ident)
                .cloned()
                .expect(&format!("cannot find var {:?} in context", ident))
            },
            
            Expr::Unop { op, expr } => {
                match op {
                    UnOp::Neg => {
                        let typ = self.type_infer(expr);
                        if self.unify(&typ, &Int) { Int }
                        else { panic!("cannot unify {:?} and int", typ) }
                    },
                    UnOp::Not => {
                        let typ = self.type_infer(expr);
                        if self.unify(&typ, &Bool) { Bool }
                        else { panic!("cannot unify {:?} and bool", typ) }
                    },
                }
            },

            Expr::Binop { op, expr1, expr2 } => {
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        let typ1 = self.type_infer(expr1);
                        let typ2 = self.type_infer(expr2);
                        if !self.unify(&typ1, &Int) {
                            panic!("cannot unify left hand side {:?} and int", typ1)
                        } else if !self.unify(&typ2, &Int) {
                            panic!("cannot unify right hand side {:?} and int", typ2)
                        } else { Int }
                    },
                    BinOp::Lt | BinOp::Gt => {
                        let typ1 = self.type_infer(expr1);
                        let typ2 = self.type_infer(expr2);
                        if !self.unify(&typ1, &Int) {
                            panic!("cannot unify left hand side {:?} and int", typ1)
                        } else if !self.unify(&typ2, &Int) {
                            panic!("cannot unify right hand side {:?} and int", typ2)
                        } else { Bool }
                    },

                    BinOp::And | BinOp::Or => {
                        let typ1 = self.type_infer(expr1);
                        let typ2 = self.type_infer(expr2);
                        if !self.unify(&typ1, &Bool) {
                            panic!("cannot unify left hand side {:?} and bool", typ1)
                        } else if !self.unify(&typ2, &Bool) {
                            panic!("cannot unify right hand side {:?} and bool", typ2)
                        } else { Bool }
                    },

                    BinOp::Eq => {
                        let typ1 = self.type_infer(expr1);
                        let typ2 = self.type_infer(expr2);
                        if !self.unify(&typ1, &typ2) {
                            panic!("cannot unify {:?} and {:?}", typ1, typ2)
                        } else { Bool }
                    },
                }
            },

            Expr::If { cond, expr1, expr2 } => {
                let cond_typ = self.type_infer(cond);
                if !self.unify(&cond_typ, &Bool) {
                    panic!("cannot unify {:?} and bool", cond_typ);
                } 
                let typ1 = self.type_infer(expr1);
                let typ2: Type = self.type_infer(expr2);
                if !self.unify(&typ1, &typ2) {
                    panic!("cannot unify {:?} and {:?}", typ1, typ2);
                }
                
                assert!(self.find(&typ1) == self.find(&typ2));
                self.find(&typ1)
            },

            Expr::Func { params, body } => {
                // frozen current context 
                let old_context = self.var_context.clone();

                // first generate type variables for param, update context 
                let mut param_types = vec![];
                for param in params.iter() {
                    let typevar = self.gen_new_typevar();
                    self.var_context.insert(param.clone(), typevar.clone());
                    param_types.push(typevar);
                }

                // type infer func body mutates acc_subst
                let mut ret_typ = self.type_infer(&body);

                // generate function type signature of canonical form 
                for param_typ in param_types.iter_mut() {
                    *param_typ = self.find(param_typ);
                }
                ret_typ = self.find(&ret_typ);

                // restore old context
                self.var_context = old_context;

                Fun(param_types, Box::new(ret_typ))
            },

            Expr::FuncApply { func, args } => {
                let func_typ = self.type_infer(func);
                if let Type::Fun(arg_typs, ret_typ) = func_typ {
                    if arg_typs.len() != args.len() {
                        panic!("wrong number of argument to apply");
                    } else {
                        for (i, arg) in args.iter().enumerate() {
                            let typ_i: &Type = &arg_typs[i];
                            let typ_i_actual = self.type_infer(arg);
                            if !self.unify(typ_i, &typ_i_actual) {
                                panic!("cannot unify {:?}th argument, 
                                    expect {:?} got {:?}",
                                    i, typ_i, typ_i_actual)
                            } 
                        }
                        self.find(&ret_typ)
                    }
                } else { panic!("cannot apply a non-function") }
            },

            // more todo on Action type
            Expr::Action { assns } => Action,
        }
    }
}

// TODO List 
// 1. more efficient implementation of var context 
// 2. add language feature: let binding