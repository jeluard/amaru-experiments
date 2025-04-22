use std::ops::{ControlFlow, FromResidual, Try};

pub type Block = ();

#[derive(Debug)]
pub enum InvalidBlockDetails {
    UncategorizedError(String),
}

pub enum StateDiff {
    UtxoProduced
}

pub enum BlockValidation {
    Valid(Option<Vec<StateDiff>>),
    Invalid(InvalidBlockDetails),
}

impl Try for BlockValidation {
    type Output = Option<Vec<StateDiff>>;
    type Residual = InvalidBlockDetails;

    fn from_output(res: Self::Output) -> Self {
        BlockValidation::Valid(res)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            BlockValidation::Valid(res) => ControlFlow::Continue(res),
            BlockValidation::Invalid(e) => ControlFlow::Break(e),
        }
    }
}

impl FromResidual for BlockValidation {
    fn from_residual(residual: InvalidBlockDetails) -> Self {
        BlockValidation::Invalid(residual)
    }
}

pub type UTXO = ();

pub struct Context {
}

pub struct Requirements {
    pub utxos: Vec<UTXO>,
}

impl Requirements {
    pub fn new() -> Self {
        Requirements { utxos: vec![] }
    }
}

pub trait ValidationRule {

    fn prepare_for(&self, block: &Block) -> Option<Requirements>;

    fn validate(&self, ctx: &Context, block: &Block) -> BlockValidation;
}