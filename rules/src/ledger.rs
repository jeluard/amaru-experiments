use std::collections::BTreeMap;

use crate::types::{Block, BlockValidation, ValidationRule};

pub struct Ledger {
    pool: Box<dyn Pool>,
}

impl Ledger {

    pub fn new(pool: Box<dyn Pool>) -> Self {
        Ledger {
            pool,
        }
    }

    pub async fn apply(&self, rules: Vec<Box<dyn ValidationRule>>, block: &Block) -> BTreeMap<Box<dyn ValidationRule>, BlockValidation> {
        for rule in rules {
            let requirements = rule.prepare_for(&block);
        }
        BTreeMap::new()
    }

    pub async fn before_epoch_transition(&self, _epoch: u64) -> Result<(), String> {
        Ok(())
    }

    pub async fn after_epoch_transition(&self, _epoch: u64) -> Result<(), String> {
        Ok(())
    }

}

pub trait Pool {

}

pub struct DefaultPool {}

impl DefaultPool {
    pub fn new() -> Self {
        DefaultPool {}
    }
}

impl Pool for DefaultPool {

}