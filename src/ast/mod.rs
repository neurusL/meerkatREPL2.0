
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

pub struct Assn {
    dest: Expr,
    src: Expr,
}

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
        br1: Box<Expr>,
        br2: Box<Expr>,
    },

    Func {
        pars: Vec<Expr>,
        body: Box<Expr>
    },
    FuncApply {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    /// Action 
    Action {
        stmt: Vec<Assn>
    },
}

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

pub struct Service {
    name: String,
    decls: Vec<Decl>,
}

pub struct Prog {
    services: Vec<Service>,
}

pub enum ReplCmd {
    Do(Expr),
    Decl(Vec<Decl>),
    Exit,

    // service related commands 
    // Service(Service),
    // Open(String),
    // Close,
}
