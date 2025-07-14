use crate::{ ast::{ Expr, Insert, Record } };

#[derive(Debug, Clone)]
pub enum TableValueState {
    Val(Expr),
}

impl TableValueState {
    pub fn new(val: Expr) -> TableValueState {
        TableValueState::Val(val)
    }

    pub fn update(&mut self, insert: &Insert) {
        // will only occur for insert queries
        let record = Record {
            val: insert.row.val
                .iter()
                .map(|entry| entry.val.clone())
                .collect(),
        };
        if let TableValueState::Val(Expr::Table { name: _, schema: _, records }) = self {
            records.push(record);
        } else {
          panic!("TableValueState is not a Table variant");
        }
        
      
    }
}

impl Into<Expr> for TableValueState {
    fn into(self) -> Expr {
        match self {
            TableValueState::Val(val) => val
        }
    }
}
