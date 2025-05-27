//! dependency analysis for var/def node in meerkat 
//! 

use std::{collections::{HashMap, HashSet}, fmt::Display};

use crate::ast;

pub mod free_var;
pub mod alpha_rename;
pub mod dep_analysis;

pub struct DependAnalysis {
    pub vars: HashSet<String>,
    pub defs: HashSet<String>,
    pub dep_graph: HashMap<String, HashSet<String>>,
    pub dep_transtive: HashMap<String, HashSet<String>>, // transitively dependent vars/defs 
    pub dep_vars: HashMap<String, HashSet<String>>,      // transitively dependent vars  
}

impl Display for DependAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dependency Graph \n")?;
        for (name, deps) in self.dep_graph.iter() {
            if deps.len() > 0 {
                write!(f, "{} -> ", name)?;
                for dep in deps.iter() {
                    write!(f, "{},", dep)?;
                }
                write!(f, "\n")?;
            }
        }
        write!(f, "Transitive Dependency (Var only) \n")?;
        for (name, deps) in self.dep_vars.iter() {
            if deps.len() > 0 {
                write!(f, "{} -> ", name)?;
                for dep in deps.iter() {
                    write!(f, "{},", dep)?;
                }
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}

pub fn calc_dep_prog(ast: &ast::Prog) {
    for srv in ast.services.iter() {
        let mut da = DependAnalysis::new(srv);
        da.calc_dep_vars();
        println!("{}", da);
    }

}