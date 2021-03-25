#[cfg(test)]
pub mod tests;

use algebra::{
    fields::tweedle::Fr,
    curves::tweedle::dum::Projective as Projective,
    curves::tweedle::dee::Affine as Affine,
    Field, PrimeField, ToBits
};
use primitives::{
    signature::schnorr::field_based_schnorr::{
        FieldBasedSchnorrSignature, FieldBasedSchnorrPk,
    },
    crh::TweedleFrPoseidonHash as PoseidonHash,
};
use r1cs_crypto::{
    signature::{
        schnorr::field_based_schnorr::{FieldBasedSchnorrSigGadget, FieldBasedSchnorrSigVerificationGadget},
        FieldBasedSigGadget,
    },
    crh::{TweedleFrPoseidonHashGadget as PoseidonHashGadget, FieldBasedHashGadget},
};

use r1cs_std::{instantiated::tweedle::TweedleDumGadget as CurveGadget, fields::{
    fp::FpGadget, FieldGadget,
}, alloc::AllocGadget, bits::{
    boolean::Boolean, FromBitsGadget,
}, eq::EqGadget, Assignment};

use r1cs_core::{ConstraintSystem, ConstraintSynthesizer, SynthesisError};

use crate::constants::NaiveThresholdSigParams;

use std::marker::PhantomData;
use rand::rngs::OsRng;
use lazy_static::*;

lazy_static! {
    pub static ref NULL_CONST: NaiveThresholdSigParams = NaiveThresholdSigParams::new();
}

//Sig types
type SchnorrSigGadget = FieldBasedSchnorrSigGadget<Fr, Projective>;
type SchnorrVrfySigGadget = FieldBasedSchnorrSigVerificationGadget<
    Fr, Projective, CurveGadget, PoseidonHash, PoseidonHashGadget
>;
type SchnorrPk = FieldBasedSchnorrPk<Projective>;
type SchnorrPkGadget = FieldBasedSchnorrPkGadget<Fr, Projective, CurveGadget>;

//Field types
type FrGadget = FpGadget<Fr>;

pub struct NaiveTresholdSignature<F: PrimeField>{

    //Witnesses
    pks:                      Vec<Option<SchnorrPk>>, //pk_n = g^sk_n
    sigs:                     Vec<Option<FieldBasedSchnorrSignature<Fr, Projective>>>, //sig_n = sign(sk_n, H(MR(BT), BH(Bi-1), BH(Bi)))
    threshold:                Option<Fr>,
    b:                        Vec<Option<bool>>,
    end_epoch_mc_b_hash:      Option<Fr>,
    prev_end_epoch_mc_b_hash: Option<Fr>,
    mr_bt:                    Option<Fr>,

    //Other
    max_pks:                  usize,
    _field:                   PhantomData<F>,
}

impl<F: PrimeField>NaiveTresholdSignature<F> {
    pub fn new(
        pks:                      Vec<SchnorrPk>,
        sigs:                     Vec<Option<FieldBasedSchnorrSignature<Fr, Projective>>>,
        threshold:                Fr,
        b:                        Fr,
        end_epoch_mc_b_hash:      Fr,
        prev_end_epoch_mc_b_hash: Fr,
        mr_bt:                    Fr,
        max_pks:                  usize,
    ) -> Self {

        //Convert b to the needed bool vector
        let b_bool = {
            let log_max_pks = (max_pks.next_power_of_two() as u64).trailing_zeros() as usize;
            let b_bits = b.write_bits();
            let to_skip = Fr::size_in_bits() - (log_max_pks + 1);
            b_bits[to_skip..].to_vec().iter().map(|&b| Some(b)).collect::<Vec<_>>()
        };
        Self{
            pks: pks.iter().map(|&pk| Some(pk)).collect::<Vec<_>>(),
            sigs,
            threshold: Some(threshold),
            b: b_bool,
            end_epoch_mc_b_hash:      Some(end_epoch_mc_b_hash),
            prev_end_epoch_mc_b_hash: Some(prev_end_epoch_mc_b_hash),
            mr_bt:                    Some(mr_bt),
            max_pks,
            _field: PhantomData
        }
    }
}

impl<F: PrimeField> ConstraintSynthesizer<Fr> for NaiveTresholdSignature<F> {
    fn generate_constraints<CS: ConstraintSystem<Fr>>(self, cs: &mut CS) -> Result<(), SynthesisError> {

        //Internal checks
        let log_max_pks = (self.max_pks.next_power_of_two() as u64).trailing_zeros() as usize;
        assert_eq!(self.max_pks, self.pks.len());
        assert_eq!(self.max_pks, self.sigs.len());
        assert_eq!(log_max_pks + 1, self.b.len());

        //Check pks are consistent with self.hash_commitment

        //Allocate public keys as witnesses
        let mut pks_g = Vec::with_capacity(self.max_pks);

        for (i, pk) in self.pks.iter().enumerate() {
            // It's safe to not perform any check when allocating the pks,
            // considering that the pks are hashed, so they should be public
            // at some point, therefore verifiable by everyone.
            let pk_g = SchnorrPkGadget::alloc_without_check(
                cs.ns(|| format!("alloc_pk_{}", i)),
                || pk.ok_or(SynthesisError::AssignmentMissing)
            )?;
            pks_g.push(pk_g);
        }

        //Enforce pks_threshold_hash
        let mut pks_threshold_hash_g = PoseidonHashGadget::check_evaluation_gadget(
            cs.ns(|| "hash public keys"),
            pks_g.iter().map(|pk| pk.pk.x.clone()).collect::<Vec<_>>().as_slice(),
        )?;

        //Allocate threshold as witness
        let t_g = FrGadget::alloc(
            cs.ns(|| "alloc threshold"),
            || self.threshold.ok_or(SynthesisError::AssignmentMissing)
        )?;

        pks_threshold_hash_g = PoseidonHashGadget::check_evaluation_gadget(
            cs.ns(|| "H(H(pks), threshold)"),
            &[pks_threshold_hash_g, t_g.clone()],
        )?;

        //Check signatures

        //Reconstruct message as H(MR(BT), BH(Bi-1), BH(Bi))
        let mr_bt_g = FrGadget::alloc(
            cs.ns(|| "alloc mr_bt"),
            || self.mr_bt.ok_or(SynthesisError::AssignmentMissing)
        )?;

        let prev_end_epoch_mc_block_hash_g = FrGadget::alloc(
            cs.ns(|| "alloc prev_end_epoch_mc_block_hash"),
            || self.prev_end_epoch_mc_b_hash.ok_or(SynthesisError::AssignmentMissing)
        )?;

        let end_epoch_mc_block_hash_g = FrGadget::alloc(
            cs.ns(|| "alloc end_epoch_mc_block_hash"),
            || self.end_epoch_mc_b_hash.ok_or(SynthesisError::AssignmentMissing)
        )?;

        let message_g = PoseidonHashGadget::check_evaluation_gadget(
            cs.ns(|| "H(MR(BT), BH(i-1), BH(i))"),
            &[mr_bt_g.clone(), prev_end_epoch_mc_block_hash_g.clone(), end_epoch_mc_block_hash_g.clone()],
        )?;

        let mut sigs_g = Vec::with_capacity(self.max_pks);

        //Allocate signatures as witnesses
        for (i, sig) in self.sigs.iter().enumerate() {
            let sig_g = SchnorrSigGadget::alloc(
                cs.ns(|| format!("alloc_sig_{}", i)),
                || sig.ok_or(SynthesisError::AssignmentMissing)
            )?;
            sigs_g.push(sig_g);
        }

        let mut verdicts = Vec::with_capacity(self.max_pks);

        //Check signatures verification verdict on message
        for (i ,(pk_g, sig_g))
            in pks_g.iter().zip(sigs_g.iter()).enumerate() {

            let v = SchnorrVrfySigGadget::enforce_signature_verdict(
                cs.ns(|| format!("check_sig_verdict_{}", i)),
                pk_g,
                sig_g,
                &[message_g.clone()],
            )?;
            verdicts.push(v);
        }

        //Count valid signatures
        let mut valid_signatures = FrGadget::zero(cs.ns(|| "alloc valid signatures count"))?;
        for (i, v) in verdicts.iter().enumerate() {
            valid_signatures = valid_signatures.conditionally_add_constant(
                cs.ns(|| format!("add_verdict_{}", i)),
                v,
                Fr::one(),
            )?;
        }

        //Enforce wcert_sysdata_hash
        let wcert_sysdata_hash_g = PoseidonHashGadget::check_evaluation_gadget(
            cs.ns(|| "H(valid_signatures, MR(BT), BH(i-1), BH(i))"),
            &[valid_signatures.clone(), mr_bt_g, prev_end_epoch_mc_block_hash_g, end_epoch_mc_block_hash_g]
        )?;

        //Check pks_threshold_hash and wcert_sysdata_hash

        let actual_aggregated_input = PoseidonHashGadget::check_evaluation_gadget(
            cs.ns(|| "H(pks_threshold_hash, wcert_sysdata_hash)"),
            &[pks_threshold_hash_g, wcert_sysdata_hash_g]
        )?;

        let expected_aggregated_input = FrGadget::alloc_input(
            cs.ns(|| "alloc aggregated input"),
            || {
                let aggregated_input_val = actual_aggregated_input.get_value().get()?;
                Ok(aggregated_input_val)
            }
        )?;

        expected_aggregated_input.enforce_equal(
            cs.ns(|| "check aggregated input"),
            &actual_aggregated_input
        )?;

        //Alloc the b's as witnesses
        let mut bs_g = Vec::with_capacity(log_max_pks + 1);
        for (i, b) in self.b.iter().enumerate(){
            let b_g = Boolean::alloc(
                cs.ns(|| format!("alloc b_{}", i)),
                || b.ok_or(SynthesisError::AssignmentMissing)
            )?;
            bs_g.push(b_g);
        }

        //Pack the b's into a field element
        let b_field = FrGadget::from_bits(
            cs.ns(|| "pack the b's into a field element"),
            bs_g.as_slice(),
        )?;

        //Enforce threshold
        valid_signatures
            .sub(cs.ns(|| "valid_signatures - threshold"), &t_g)?
            .enforce_equal(cs.ns(|| "threshold check"), &b_field)?;

        Ok(())
    }
}

use r1cs_crypto::signature::schnorr::field_based_schnorr::FieldBasedSchnorrPkGadget;

use marlin::*;
use blake2::Blake2s;
use poly_commit::ipa_pc::InnerProductArgPC;

// #[derive(Clone)]
// struct MarlinNoLCNoZk;

// impl MarlinConfig for MarlinNoLCNoZk {
//     const LC_OPT: bool = false;
//     const ZK: bool = false;
// }

type IPAPC = InnerProductArgPC<Affine, Blake2s>;
// type MarlinInst = Marlin<Fr, IPAPC, Blake2s, MarlinNoLCNoZk>;
type MarlinInst = Marlin<Fr, IPAPC, Blake2s>;

#[allow(dead_code)]
pub fn generate_parameters(max_pks: usize) -> Result<(IndexProverKey<Fr, IPAPC>, IndexVerifierKey<Fr, IPAPC>), SynthesisError> {

    //Istantiating rng
    let mut rng = OsRng::default();

    //Istantiating supported number of pks and sigs
    let log_max_pks = (max_pks.next_power_of_two() as u64).trailing_zeros() as usize;

    // Create parameters for our circuit
    let c = NaiveTresholdSignature::<Fr> {
        pks:                      vec![None; max_pks],
        sigs:                     vec![None; max_pks],
        threshold:                None,
        b:                        vec![None; log_max_pks + 1],
        end_epoch_mc_b_hash:      None,
        prev_end_epoch_mc_b_hash: None,
        mr_bt:                    None,
        max_pks,
        _field:                   PhantomData
    };

    let universal_srs = MarlinInst::universal_setup(2usize.pow(20), 2usize.pow(20), 2usize.pow(20), &mut rng).unwrap();

    Ok(MarlinInst::index(
        &universal_srs,
        c,
    ).unwrap())
}

#[cfg(test)]
mod test {
    use super::*;
    use algebra::{BigInteger256, ProjectiveCurve};
    use primitives::{
        crh::FieldBasedHash,
        signature::{
            FieldBasedSignatureScheme, schnorr::field_based_schnorr::FieldBasedSchnorrSignatureScheme,
        },
    };
    use poly_commit::Error as PCError;
    use marlin::{Proof, Error as MarlinError};
    use rand::{
        Rng, rngs::OsRng, thread_rng
    };

    type SchnorrSig = FieldBasedSchnorrSignatureScheme<Fr, Projective, PoseidonHash>;

    fn generate_test_proof(
        max_pks:                  usize,
        valid_sigs:               usize,
        threshold:                usize,
        wrong_pks_threshold_hash: bool,
        wrong_wcert_sysdata_hash: bool,
        params:                   (IndexProverKey<Fr, IPAPC>, IndexVerifierKey<Fr, IPAPC>),
    ) -> Result<(Proof<Fr, IPAPC>, Vec<Fr>), MarlinError<PCError>> {

        //Istantiate rng
        let mut rng = OsRng::default();
        let mut h = PoseidonHash::init(None);

        //Generate message to sign
        let mr_bt: Fr = rng.gen();
        let prev_end_epoch_mc_b_hash: Fr = rng.gen();
        let end_epoch_mc_b_hash: Fr = rng.gen();
        let message = h
            .update(mr_bt)
            .update(prev_end_epoch_mc_b_hash)
            .update(end_epoch_mc_b_hash)
            .finalize();

        //Generate another random message used to simulate a non-valid signature
        let invalid_message: Fr = rng.gen();

        let mut pks = vec![];
        let mut sigs = vec![];

        for _ in 0..valid_sigs {
            let (pk, sk) = SchnorrSig::keygen(&mut rng);
            let sig = SchnorrSig::sign(&mut rng, &pk, &sk, &[message]).unwrap();
            pks.push(pk);
            sigs.push(Some(sig));
        }

        for _ in 0..(max_pks-valid_sigs){
            //Sample a random boolean and decide if generating a non valid signature or a null one
            let generate_null: bool = rng.gen();
            let (pk, sig) = if generate_null {
                (NULL_CONST.null_pk, NULL_CONST.null_sig)
            } else {

                let (pk, sk) = SchnorrSig::keygen(&mut rng);
                let sig = SchnorrSig::sign(&mut rng, &pk, &sk, &[invalid_message]).unwrap();
                (pk, sig)
            };
            pks.push(pk);
            sigs.push(Some(sig));
        }

        //Generate b
        let t_field = Fr::from_repr(BigInteger256::from(threshold as u64));
        let valid_field = Fr::from_repr(BigInteger256::from(valid_sigs as u64));
        let b_field = valid_field - &t_field;

        //Compute pks_threshold_hash
        h.reset(None);
        pks.iter().for_each(|pk| { h.update(pk.0.into_affine().x); });
        let pks_hash = h.finalize();
        let pks_threshold_hash = if !wrong_pks_threshold_hash {
            h
                .reset(None)
                .update(pks_hash)
                .update(t_field)
                .finalize()
        } else {
            rng.gen()
        };

        //Compute wcert_sysdata_hash
        let wcert_sysdata_hash = if !wrong_wcert_sysdata_hash {
            h
                .reset(None)
                .update(valid_field)
                .update(mr_bt)
                .update(prev_end_epoch_mc_b_hash)
                .update(end_epoch_mc_b_hash)
                .finalize()
        } else {
            rng.gen()
        };

        // Compute aggregated input
        let aggregated_input = h
            .reset(None)
            .update(pks_threshold_hash)
            .update(wcert_sysdata_hash)
            .finalize();

        //Create proof for our circuit
        let c = NaiveTresholdSignature::<Fr>::new(
            pks, sigs, t_field, b_field, end_epoch_mc_b_hash,
            prev_end_epoch_mc_b_hash, mr_bt, max_pks,
        );

        //Return proof and public inputs if success
        let start = std::time::Instant::now();

        let proof = match MarlinInst::prove::<_, OsRng>(&params.0, c, &mut rng) {
            Ok(proof) => {
                let public_inputs = vec![aggregated_input];
                Ok((proof, public_inputs))
            }
            Err(e) => Err(e)
        };
        println!("Proof creation time: {:?}", start.elapsed());
        proof
    }

    #[test]
    #[should_panic]
    fn test_naive_threshold_circuit() {
        let rng = &mut thread_rng();
        let n = 6;

        let params = generate_parameters(n).unwrap();

        //Generate proof with correct witnesses and v > t
        let (proof, public_inputs) =
            generate_test_proof(n, 5, 4, false, false, params.clone()).unwrap();
        assert!(MarlinInst::verify(&params.1, public_inputs.as_slice(), &proof, rng).unwrap());

        //Generate proof with insufficient valid signatures
        let (proof, public_inputs) =
            generate_test_proof(n, 4, 5, false, false, params.clone()).unwrap();
        assert!(MarlinInst::verify(&params.1, public_inputs.as_slice(), &proof, rng).unwrap());

        //Generate proof with bad pks_threshold_hash
        let (proof, public_inputs) =
            generate_test_proof(n, 5, 4, true, false, params.clone()).unwrap();
        assert!(MarlinInst::verify(&params.1, public_inputs.as_slice(), &proof, rng).unwrap());

        //Generate proof with bad wcert_sysdata_hash
        let (proof, public_inputs) =
            generate_test_proof(n, 5, 4, false, true, params.clone()).unwrap();
        assert!(MarlinInst::verify(&params.1, public_inputs.as_slice(), &proof, rng).unwrap());
    }
}