use std::collections::{HashMap};

use crate::runtime::message::Val;
// TODO: now we only assume def f := f1 + f2 + ... + f_n
pub fn compute_val(replica: &HashMap<String, Option<Val>>) -> Option<Val> {
    let mut sum = 0;
    for (k, value) in replica.iter() {
        match value {
            Some(Val::Int(v)) => sum += v,
            Some(_) => todo!(),
            None => return None,
        }
    }
    return Some(Val::Int(sum));
}
