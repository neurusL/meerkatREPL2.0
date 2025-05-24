use crate::ast::{Assn, Decl, Expr};

use super::{Evaluator, Val};

impl Evaluator {
    pub fn eval_decl(&mut self, decl: &Decl) {
        todo!()
    }

    pub fn eval_assn(&mut self, assn: &mut Assn) -> Result<(), String> {
        let val = self.eval_expr(&mut assn.src)?;
        self.env.insert(assn.dest.clone(), val.clone());
        assn.src = Expr::from(val);
        Ok(())
    }
}