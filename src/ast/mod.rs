
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
    // Action {
    //     stmt: Stmt
    // }
}
