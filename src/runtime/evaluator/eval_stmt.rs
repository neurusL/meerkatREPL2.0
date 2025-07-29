use std::collections::HashSet;

use crate::ast::{Assn, Decl, Expr};

use super::{Evaluator, Val};

use super::eval_expr::rewrite_expr;



impl Evaluator {
    pub fn eval_decl(&mut self, decl: &mut Decl) -> Result<(), String> {
        match decl {
            Decl::Import { srv_name: _ } => todo!(),

            Decl::VarDecl { name, val } => {
                // Generate new versioned name
                let new_name = self.version_map.declare(name);

                // Rewrite all variable references using latest versioned names
                let mut rewritten_val = val.clone();
                rewrite_expr(&mut rewritten_val, &self.version_map);


                // Substitute variable name with versioned name in its own definition
                rewritten_val.substitute(name, &Expr::Variable { ident: new_name.clone() });

               
                // Evaluate expression and store
                self.reactive_names.insert(new_name.clone());
                self.eval_expr(&mut rewritten_val)?;
                self.reactive_name_to_vals.insert(new_name.clone(), rewritten_val);
                
                
            }

            Decl::DefDecl { name, val, is_pub: _ } => {
                // Generate new versioned name
                let new_name = self.version_map.next_version(name);

                // Rewrite references inside the definition
                let mut rewritten_val = val.clone();
                rewrite_expr(&mut rewritten_val, &self.version_map);

                self.reactive_names.insert(new_name.clone());
                self.eval_expr(&mut rewritten_val)?;
                self.def_name_to_exprs.insert(new_name.clone(), rewritten_val.clone());

                
                self.reactive_name_to_vals.insert(new_name.clone(), rewritten_val);
                
            }
        }

        Ok(())
    }

    pub fn eval_assn(&mut self, assn: &mut Assn) -> Result<(), String> {
        self.eval_expr(&mut assn.src)?;

        // Generate new version name for the destination variable
        let new_name = self.version_map.next_version(&assn.dest);
        assn.dest = new_name.clone();

        // Store the result in the evaluator's environment
        self.reactive_names.insert(new_name.clone());
        self.reactive_name_to_vals.insert(new_name, assn.src.clone());


        Ok(())
    }

    pub fn eval_assert(&mut self, expr: &mut Expr) -> Result<(), String> {
        self.eval_expr(expr)?;
        Ok(())
    }
}

