#![feature(try_trait_v2)]

use ledger::{DefaultPool, Ledger};
use types::{Block, BlockValidation, Requirements, ValidationRule};

mod rules;
mod ledger;
mod types;

struct NoRequirementRule {}

impl ValidationRule for NoRequirementRule {
    fn prepare_for(&self, _block: &Block) -> Option<Requirements> {
        None        
    }
    fn validate(&self, _ctx: &types::Context, _block: &Block) -> BlockValidation {
        BlockValidation::Valid(None)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup various bits. Not part of block processing hotpath.

    let ledger = Ledger::new(Box::new(DefaultPool::new()));
    let rules: Vec<Box<dyn ValidationRule>> = vec![Box::new(NoRequirementRule {})];

    let block: Block = ();

    // QQ can a rule output data required by another rule?
    // QQ: termination condition: every rules complete for every blocks ?
    let results = ledger.apply(rules, &block).await;
    // TODO Same for transaction validation

    // Then computes the deltas that will be applied to the state
    // Some bits must be done before moving to next block (QQ: could we start some bits of another block processing in avance?)
    // Some bits are long running and must be finished before/after epoch transition

    let epoch = 1;
    ledger.before_epoch_transition(epoch).await?;

    ledger.after_epoch_transition(epoch).await?;

    Ok(())
}
