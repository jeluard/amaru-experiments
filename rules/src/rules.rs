use crate::types::{Block, BlockValidation, Context, Requirements, ValidationRule};

pub fn merge_requirements(requirements: impl IntoIterator<Item = Requirements>) -> Requirements {
    let mut result = Requirements::new();
    for requirement in requirements {
        result.utxos.extend(requirement.utxos);
    }
    result
}