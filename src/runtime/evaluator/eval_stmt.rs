use crate::ast::{Assn, Decl, Expr};

use super::{Evaluator, Val};

impl Evaluator {
    pub fn eval_decl(&mut self, decl: &mut Decl) {
        match decl {
            Decl::Import { srv_name } => todo!(),
            Decl::VarDecl { name, val } => {
                self.reactive_names.insert(name.clone());
                self.eval_expr(val);
                self.env.insert(name.clone(), val.clone());
            },
            Decl::DefDecl { name, val, is_pub } => {
                self.reactive_names.insert(name.clone());
                self.eval_expr(val);
                self.env.insert(name.clone(), val.clone());
            },
        }
    }

    pub fn eval_assn(&mut self, assn: &mut Assn) -> Result<(), String> {
        self.eval_expr(&mut assn.src)?;
        self.env.insert(assn.dest.clone(), assn.src.clone());
        Ok(())
    }
}