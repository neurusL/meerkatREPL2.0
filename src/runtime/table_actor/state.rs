use crate::{ ast::{ Expr, Insert, Record } };
use log::info;

#[derive(Debug, Clone)]
pub enum TableValueState {
    Val(Expr),
}

impl TableValueState {
    pub fn new(val: Expr) -> TableValueState {
        TableValueState::Val(val)
    }

    pub fn update(&mut self, insert: &Insert) -> Expr {
        // will only occur for insert queries
        let mut found_records = Vec::new();
        if let Expr::Vector { val } = &insert.row {
            for keyval in val {
                if let Expr::KeyVal { key, value } = keyval {
                    found_records.push((**value).clone());
                } else{
                    println!("Expected keyval, got {:?}", keyval);
                }
            }
        } else {
            println!("Expected rows, got {:?}", &insert.row);
        }
        if let TableValueState::Val(Expr::Table { schema: _, records }) = self {
            records.push(Expr::Vector{val: found_records.clone()});
            info!("Updated table state!");
        } else {
          panic!("TableValueState is not a Table variant");
        }
        Expr::Vector{val: found_records}            // return new record when updating
        
      
    }
}

impl Into<Expr> for TableValueState {
    fn into(self) -> Expr {
        match self {
            TableValueState::Val(val) => val
        }
    }
}
