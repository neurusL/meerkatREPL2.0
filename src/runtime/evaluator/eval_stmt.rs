use std::collections::HashSet;

use crate::ast::{Assn, Decl, Expr};

use super::{Evaluator, Val};

impl Evaluator {
    pub fn eval_decl(&mut self, decl: &mut Decl) -> Result<(), String> {
        match decl {
            Decl::Import { srv_name } => todo!(),
            Decl::VarDecl { name, val } => {
                self.reactive_names.insert(name.clone());

                // var should have no depend
                assert!(val.free_var(&HashSet::new()).is_empty());

                self.eval_expr(val)?;
                self.reactive_name_to_vals.insert(name.clone(), val.clone());
            }
            Decl::DefDecl { name, val, is_pub } => {
                self.reactive_names.insert(name.clone());

                // unevaled expr of def should be stored
                self.def_name_to_exprs.insert(name.clone(), val.clone());

                // then eval def
                self.eval_expr(val)?;
                self.reactive_name_to_vals.insert(name.clone(), val.clone());
            }
            Decl::TableDecl { name, records } => {
                // do nothing
            }
        }

        Ok(())
    }

    pub fn eval_assn(&mut self, assn: &mut Assn) -> Result<(), String> {
        self.eval_expr(&mut assn.src)?;

        // since assn only appears in action,
        // their effect should not protrude to the expression level language's env
        // self.env.insert(assn.dest.clone(), assn.src.clone());
        Ok(())
    }

    pub fn eval_assert(&mut self, expr: &mut Expr) -> Result<(), String> {
        self.eval_expr(expr)?;

        Ok(())
    }
}
