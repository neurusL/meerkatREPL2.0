use std::fmt::Display;

mod utils;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Neg, // negate
    Not, // logical not
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Assn {
    pub dest: String,
    pub src: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    /// Basic Lambda Core expressions
    Number { val: i32 },
    Bool { val: bool},
    Variable { ident: String },

    Unop {
        op: UnOp,
        expr: Box<Expr>
    },
    Binop {
        op: BinOp,
        expr1: Box<Expr>,
        expr2: Box<Expr>,
    },

    If {
        cond: Box<Expr>,
        expr1: Box<Expr>,
        expr2: Box<Expr>,
    },

    Func {
        params: Vec<String>,
        body: Box<Expr>
    },
    FuncApply {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    /// Action 
    Action {
        assns: Vec<Assn>
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Service {
    pub name: String,
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Prog {
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReplCmd {
    Do(Expr),
    Decl(Vec<Decl>),
    Exit,

    // service related commands 
    // Service(Service),
    // Open(String),
    // Close,
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Number { val } => write!(f, "Int({})", val),
            Expr::Bool { val } => write!(f, "Bool({})", val),
            Expr::Variable { ident } => write!(f, "Var({})", ident),
            Expr::Unop { op, expr } => 
                write!(f, "{:?}{}", op, expr),
            Expr::Binop { op, expr1, expr2 } => 
                write!(f, "{} {:?} {}", expr1, op, expr2),
            Expr::If { cond, expr1, expr2 } => 
                write!(f, "if {} then {} else {}", cond, expr1, expr2),
            Expr::Func { params, body } => 
                write!(f, "Func({:?})[{}]", params, body),
            Expr::FuncApply { func, args } => 
                write!(f, "{}({:?})", func, args),
            Expr::Action { assns } => 
                write!(f, "Action({:?})", assns),
        }
    }
}
