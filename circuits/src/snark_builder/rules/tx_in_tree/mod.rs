use algebra::PrimeField;
use primitives::{FieldBasedHash, FieldBasedMerkleTreeParameters, FieldBasedBinaryMHTPath};
use r1cs_crypto::{FieldBasedHashGadget, FieldBasedMerkleTreePathGadget, FieldHasherGadget};
use r1cs_core::{ConstraintSystem, SynthesisError};
use r1cs_std::fields::fp::FpGadget;

pub mod core;

/// Generic trait holding data and SNARK logic required to enforce belonging of a Transaction
/// to a Merkle Tree
pub trait TxInTreeRule<
    ConstraintF: PrimeField,
    H: FieldBasedHash<Data = ConstraintF>,
    HG: FieldBasedHashGadget<H, ConstraintF>,
    P: FieldBasedMerkleTreeParameters<Data = ConstraintF, H = H>,
>: Sized
{
    type MerklePathGadget:     FieldBasedMerkleTreePathGadget<FieldBasedBinaryMHTPath<P>, H, HG, ConstraintF>;
    type TransactionGadget:    FieldHasherGadget<H, ConstraintF, HG>;

    fn enforce_rule<CS: ConstraintSystem<ConstraintF>>(
        &self,
        cs:             CS,
        tx_g:           &Self::TransactionGadget,
        tx_path_g:      Self::MerklePathGadget,
        prev_root_g:    FpGadget<ConstraintF>,
        next_root_g:    FpGadget<ConstraintF>,
    ) -> Result<(), SynthesisError>;
}