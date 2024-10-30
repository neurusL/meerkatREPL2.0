use std::collections::HashSet;

#[allow(dead_code)]
trait AstNode {}

impl AstNode for ReplInput {}
#[derive(Debug, Clone)]
pub enum ReplInput {
    Service(Service),
    Do(Stmt),
    Decl(Vec<Decl>),
    // Update(Decl),
    Open(String),
    Close,
    Exit,
}

impl AstNode for Program {}
#[derive(Debug, Clone)]
pub enum Program {
    Prog { services: Vec<Service> },
}

impl AstNode for Service {}
#[derive(Debug, Clone)]
pub enum Service {
    Srv { name: String, decls: Vec<Decl> },
}

impl AstNode for Decl {}
#[derive(Debug, Clone)]
pub enum Decl {
    Import {
        srv_name: String,
    },
    VarDecl {
        name: String,
        val: Expr,
    },
    DefDecl {
        name: String,
        val: Expr,
        is_pub: bool,
    },
}

impl AstNode for Stmt {}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stmt {
    Stmt { sgl_stmts: Vec<SglStmt> },
}

impl AstNode for SglStmt {}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SglStmt {
    Do { act: Expr },
    Ass { dst: Expr, src: Expr },
}

impl AstNode for Expr {}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    IdExpr {
        ident: String,
    },
    IntConst {
        val: i32,
    },
    BoolConst {
        val: bool,
    },
    Action {
        stmt: Stmt,
    },
    Member {
        srv_name: String,
        member: Box<Expr>,
    },
    Apply {
        fun: Box<Expr>,
        args: Vec<Expr>,
    },
    BopExpr {
        opd1: Box<Expr>,
        opd2: Box<Expr>,
        bop: Binop,
    },
    UopExpr {
        opd: Box<Expr>,
        uop: Uop,
    },
    IfExpr {
        cond: Box<Expr>,
        then: Box<Expr>,
        elze: Box<Expr>,
    },
    Lambda {
        pars: Vec<Expr>,
        body: Box<Expr>,
    },
}

impl Expr {
    pub fn names_contained(&self) -> HashSet<String> {
        fn compute_names_contained(names: &mut HashSet<String>, expr: &Expr) {
            match expr {
                Expr::IdExpr { ident } => {
                    names.insert(ident.clone());
                }
                Expr::IntConst { val: _ } | Expr::BoolConst { val: _ } => {}
                Expr::Action { stmt } => {
                    let sgls = match stmt {
                        Stmt::Stmt { sgl_stmts } => sgl_stmts,
                    };
                    for sgl in sgls.iter() {
                        match sgl {
                            SglStmt::Do { act } => {
                                compute_names_contained(names, act);
                            }
                            SglStmt::Ass { dst: _, src } => {
                                compute_names_contained(names, src);
                            }
                        }
                    }
                }
                Expr::Member {
                    srv_name: _,
                    member: _,
                } => panic!("not yet support multi service"),
                Expr::Apply { fun, args } => {
                    compute_names_contained(names, fun);
                    for arg_expr in args.iter() {
                        compute_names_contained(names, arg_expr);
                    }
                }
                Expr::BopExpr { opd1, opd2, bop: _ } => {
                    compute_names_contained(names, opd1);
                    compute_names_contained(names, opd2);
                }
                Expr::UopExpr { opd, uop: _ } => {
                    compute_names_contained(names, opd);
                }
                Expr::IfExpr { cond, then, elze } => {
                    compute_names_contained(names, cond);
                    compute_names_contained(names, then);
                    compute_names_contained(names, elze);
                }
                Expr::Lambda { pars, body } => {
                    let mut par_names: HashSet<String> = HashSet::new();
                    for par in pars.iter() {
                        let par_name = match par {
                            Expr::IdExpr { ident } => ident.clone(),
                            _ => panic!(),
                        };
                        par_names.insert(par_name);
                    }
                    compute_names_contained(names, body);
                    *names = names.difference(&par_names).cloned().collect();
                }
            }
        }
        let mut deps: HashSet<String> = HashSet::new();
        compute_names_contained(&mut deps, self);
        deps
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Uop {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Binop {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Gt,
    And,
    Or,
}
