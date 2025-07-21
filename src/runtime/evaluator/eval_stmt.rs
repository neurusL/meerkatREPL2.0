use std::{collections::HashSet, hash::Hash};

use crate::ast::{Assn, Decl, Expr};

use super::{Evaluator, Val};

use super::eval_expr::rewrite_expr;


// impl Evaluator {
//     pub fn eval_decl(&mut self, decl: &mut Decl) -> Result<(), String> {
//         match decl {
//             Decl::Import { srv_name } => todo!(),
//             Decl::VarDecl { name, val } => {
//                 self.reactive_names.insert(name.clone());

//                 // var should have no depend
//                 assert!(val.free_var(&HashSet::new()).is_empty());

//                 self.eval_expr(val)?;
//                 self.reactive_name_to_vals.insert(name.clone(), val.clone());
//             }
//             Decl::DefDecl { name, val, is_pub } => {
//                 self.reactive_names.insert(name.clone());

//                 // unevaled expr of def should be stored
//                 self.def_name_to_exprs.insert(name.clone(), val.clone());

//                 // then eval def
//                 self.eval_expr(val)?;
//                 self.reactive_name_to_vals.insert(name.clone(), val.clone());
//             }
//         }

//         Ok(())
//     }

//     pub fn eval_assn(&mut self, assn: &mut Assn) -> Result<(), String> {
//         self.eval_expr(&mut assn.src)?;

//         // since assn only appears in action,
//         // their effect should not protrude to the expression level language's env
//         // self.env.insert(assn.dest.clone(), assn.src.clone());
//         Ok(())
//     }

//     pub fn eval_assert(&mut self, expr: &mut Expr) -> Result<(), String> {
//         self.eval_expr(expr)?;

//         Ok(())
//     }
// }


impl Evaluator {
    pub fn eval_decl(&mut self, decl: &mut Decl) -> Result<(), String> {
        match decl {
            Decl::Import { srv_name: _ } => todo!(),

            Decl::VarDecl { name, val } => {
                // Generate new versioned name
                let new_name = self.version_map.next_version(name);

                // Rewrite all variable references using latest versioned names
                rewrite_expr(val, &self.version_map);
                // var should have no depend
                assert!(val.free_var(&HashSet::new(), &HashSet::new()).is_empty());


                // Substitute variable name with versioned name in its own definition
                let mut renamed_expr = val.clone();
                renamed_expr.substitute(name, &Expr::Variable { ident: new_name.clone() });

                self.reactive_names.insert(new_name.clone());

                // Optional: checking if expression is now closed
                assert!(renamed_expr.free_var(&HashSet::new()).is_empty());

                self.eval_expr(&mut renamed_expr)?;
                // Evaluate expression and store
                self.reactive_name_to_vals.insert(new_name.clone(), renamed_expr.clone());
                
            }

            Decl::DefDecl { name, val, is_pub: _ } => {
                // Generate new versioned name
                let new_name = self.version_map.next_version(name);

                // Rewrite references inside the definition
                rewrite_expr(val, &self.version_map);

                self.reactive_names.insert(new_name.clone());
                self.def_name_to_exprs.insert(new_name.clone(), val.clone());

                
                self.reactive_name_to_vals.insert(new_name.clone(), val.clone());
                self.eval_expr(val)?;
            }
        }

        Ok(())
    }

    pub fn eval_assn(&mut self, assn: &mut Assn) -> Result<(), String> {
        self.eval_expr(&mut assn.src)?;

        // Generate new version name for the destination variable
        let new_name = self.version_map.next_version(&assn.dest);

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

