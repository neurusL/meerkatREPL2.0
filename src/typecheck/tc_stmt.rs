use crate::ast::Decl;

use super::TypecheckEnv;



impl TypecheckEnv {
    pub fn typecheck_stmt(&mut self, stmt: &Decl) {
        match stmt {
            Decl::Import { srv_name } => todo!(),
            Decl::VarDecl { name, val } => {
                let typ = self.infer_expr(&val);
                self.var_context.insert(name.clone(), typ);
            },
            Decl::DefDecl { name, val, is_pub } => {
                let typ = self.infer_expr(&val);
                self.var_context.insert(name.clone(), typ);
            },
        }
    }
}