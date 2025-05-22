use std::vec;

use super::{Type, TypecheckEnv};
use crate::ast::*;

impl TypecheckEnv {
    pub fn type_infer(&mut self, expr: &Expr) -> Type {
        use Type::*;
        match expr {
            Expr::Number { val } => Int,
            Expr::Bool { val } => Bool,
            Expr::Variable { ident } => {
                self.var_to_typ.get(ident)
                .cloned()
                .expect(&format!("cannot find var {:?} in context", ident))
            },
            Expr::Unop { op, expr } => todo!(),
            Expr::Binop { op, expr1, expr2 } => todo!(),

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
                let old_context = self.var_to_typ.clone();

                // first generate type variables for param, update context 
                let mut param_types = vec![];
                for param in params.iter() {
                    let typevar = self.gen_new_typevar();
                    self.var_to_typ.insert(param.clone(), typevar.clone());
                    param_types.push(typevar);
                }
                // type infer func body mutates acc_subst
                let mut ret_typ = self.type_infer(&body);

                // generate function type signature of canonical form 
                for param_typ in param_types.iter_mut() {
                    *param_typ = self.find(param_typ);
                }
                ret_typ = self.find(&ret_typ);

                Fun(param_types, Box::new(ret_typ))
            },

            Expr::FuncApply { func, args } => todo!(),

            // more todo on Action type
            Expr::Action { assns } => todo!(),
        }
    }
}