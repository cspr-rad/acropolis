// These constants represent the RISC-V ELF and the image ID generated by risc0-build.
// The ELF is used for proving and the ID is used for verification.
use k256::ecdsa::{signature::Signer, Signature, SigningKey, VerifyingKey};
use methods::ACROPOLIS_ELF;
use risc0_types::CircuitInputs;
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};

pub fn prove(
    choice: &str,
    user_secret_key: &SigningKey,
    government_public_key: &VerifyingKey,
    public_identity: &Signature,
) -> Receipt {
    let user_public_key_serialized: Vec<u8> = user_secret_key
        .verifying_key()
        .clone()
        .to_encoded_point(true)
        .to_bytes()
        .to_vec();
    let government_public_key_serialized: Vec<u8> = government_public_key
        .to_encoded_point(true)
        .to_bytes()
        .to_vec();
    let unique_session_signature: Signature = user_secret_key.sign(&choice.as_bytes().to_vec());
    let circuit_inputs: CircuitInputs = CircuitInputs {
        choice: choice.to_string(),
        user_public_key: user_public_key_serialized,
        session_signature: unique_session_signature,
        government_public_key: government_public_key_serialized,
        public_identity: *public_identity,
    };

    let env = ExecutorEnv::builder()
        .write(&circuit_inputs)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();
    prover.prove(env, ACROPOLIS_ELF).unwrap()
}

#[cfg(feature = "groth16")]
pub fn prove_groth16(receipt: &Receipt) -> Receipt {
    use risc0_groth16::docker::stark_to_snark;
    use risc0_zkvm::{
        get_prover_server, recursion::identity_p254, CompactReceipt, ExecutorEnv, InnerReceipt,
        ProverOpts, Receipt,
    };
    let opts = ProverOpts::default();
    let prover = get_prover_server(&opts).unwrap();
    let claim = receipt.get_claim().unwrap();
    let composite_receipt = receipt.inner.composite().unwrap();
    let succinct_receipt = prover.compress(composite_receipt).unwrap();
    let journal = receipt.journal.bytes.clone();
    let ident_receipt = identity_p254(&succinct_receipt).unwrap();
    let seal_bytes = ident_receipt.get_seal_bytes();
    let seal = stark_to_snark(&seal_bytes).unwrap().to_vec();
    Receipt::new(
        InnerReceipt::Compact(CompactReceipt { seal, claim }),
        journal,
    )
}

#[test]
fn generate_proof() {
    use rand_core::OsRng;
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key_bytes = signing_key
        .verifying_key()
        .to_encoded_point(true)
        .to_bytes()
        .to_vec();
    let government_signing_key: SigningKey = SigningKey::random(&mut OsRng);
    let mut payload: Vec<u8> = verifying_key_bytes.clone();
    payload.append(
        government_signing_key
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .as_mut(),
    );
    let public_identity: Signature = government_signing_key.sign(&payload);
    let choice: String = "42".to_string();
    prove(
        &choice,
        &signing_key,
        government_signing_key.verifying_key(),
        &public_identity,
    );
}

#[cfg(feature = "groth16")]
#[test]
fn optimize_groth16_proof() {
    use rand_core::OsRng;
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key_bytes = signing_key
        .verifying_key()
        .to_encoded_point(true)
        .to_bytes()
        .to_vec();
    let government_signing_key: SigningKey = SigningKey::random(&mut OsRng);
    let mut payload: Vec<u8> = verifying_key_bytes.clone();
    payload.append(
        government_signing_key
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .as_mut(),
    );
    let public_identity: Signature = government_signing_key.sign(&payload);
    let choice: String = "42".to_string();
    let receipt = prove(
        &choice,
        &signing_key,
        government_signing_key.verifying_key(),
        &public_identity,
    );
    let optimized_receipt = prove_groth16(&receipt);
    println!(
        "Optimized Receipt size: {:?}",
        serde_json::to_vec(&optimized_receipt).unwrap().len()
    );
}
