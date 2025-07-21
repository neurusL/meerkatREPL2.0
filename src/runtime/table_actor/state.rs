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
        let mut found_records = Vec::new();
        if let Expr::Rows { val: rows } = &insert.row {
            for row in rows {
                for entry in &row.val {
                    found_records.push(entry.val.clone());
                }
            }
        }
        let record = Record {val: found_records};
        if let TableValueState::Val(Expr::Table { schema: _, records }) = self {
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
