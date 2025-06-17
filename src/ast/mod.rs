use std::fmt::Display;

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
    Number {
        val: i32,
    },
    Bool {
        val: bool,
    },
    Variable {
        ident: String,
    },

    Unop {
        op: UnOp,
        expr: Box<Expr>,
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
        body: Box<Expr>,
    },
    FuncApply {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    /// Action
    Action {
        assns: Vec<Assn>,
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
    TableDecl {
        name: String,
        records: Vec<Record>,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Record {
    pub name: String,
    pub type_: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    String,
    Number,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Service {
    pub name: String,
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Test {
    pub name: String,
    pub commands: Vec<ReplCmd>, // commands here refer to dos and asserts
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Prog {
    pub services: Vec<Service>,
    pub tests: Vec<Test>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReplCmd {
    Do(Expr),
    Assert(Expr),
    // service related commands
    // Service(Service),
    // Open(String),
    // Close,
}

impl Default for Expr {
    fn default() -> Self {
        Expr::Number { val: 0 }
    }
}

impl Display for UnOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnOp::Neg => write!(f, "-"),
            UnOp::Not => write!(f, "!"),
        }
    }
}

impl Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::And => write!(f, "&&"),
            BinOp::Or => write!(f, "||"),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Number { val } => write!(f, "{}", val),
            Expr::Bool { val } => write!(f, "{}", val),
            Expr::Variable { ident } => write!(f, "{}", ident),
            Expr::Unop { op, expr } => write!(f, "{}{}", op, expr),
            Expr::Binop { op, expr1, expr2 } => write!(f, "{} {} {}", expr1, op, expr2),
            Expr::If { cond, expr1, expr2 } => {
                write!(f, "if {} then {} else {}", cond, expr1, expr2)
            }
            Expr::Func { params, body } => write!(f, "fn({})[{}]", params.join(","), body),
            Expr::FuncApply { func, args } => write!(
                f,
                "{}({})",
                func,
                args.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expr::Action { assns } => write!(
                f,
                "Action({:?})",
                assns
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl Display for Assn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.dest, self.src)
    }
}

impl Display for Decl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decl::Import { srv_name } => todo!(),
            Decl::VarDecl { name, val } => {
                write!(f, "var {} = {}", name, val)
            }
            Decl::DefDecl { name, val, is_pub } => {
                if *is_pub {
                    write!(f, "pub def {} = {}", name, val)
                } else {
                    write!(f, "def {} = {}", name, val)
                }
            }
            Decl::TableDecl { name, records } => {
                write!(f, "table {} created", name)
            }
        }
    }
}
