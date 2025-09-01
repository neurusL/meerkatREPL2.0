use std::collections::{HashMap, HashSet};

use crate::ast::Service;

use super::{ServiceEnv, TypecheckEnv};

impl TypecheckEnv {
    pub fn typecheck_service(&mut self, srvs: &Service) {
        self.open_service_name = Some(srvs.name.clone());
        self.services.insert(
            srvs.name.clone(),
            ServiceEnv {
                name_context: HashMap::new(),
                imports: HashSet::new(),
            },
        );
        for stmt in srvs.decls.iter() {
            self.typecheck_decl(stmt);
        }
        self.open_service_name = None;
    }
}
