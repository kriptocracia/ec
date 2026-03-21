use anyhow::{Result, Context};
use blind_rsa_signatures::{KeyPair, Sha384, PSS, Randomized, DefaultRng, SecretKey, PublicKey, Signature, MessageRandomizer};
use base64::Engine;
use rand::RngExt;

pub fn generate_keypair() -> Result<(String, String)> {
    let mut rng = DefaultRng::default();
    let kp = KeyPair::<Sha384, PSS, Randomized>::generate(&mut rng, 2048)?;
    
    let pk_der = kp.pk.to_der()?;
    let sk_der = kp.sk.to_der()?;
    
    Ok((
        base64::engine::general_purpose::STANDARD.encode(pk_der),
        base64::engine::general_purpose::STANDARD.encode(sk_der),
    ))
}

pub fn blind_sign(sk_der_b64: &str, blinded_message: &[u8]) -> Result<Vec<u8>> {
    let sk_der = base64::engine::general_purpose::STANDARD.decode(sk_der_b64)?;
    let sk = SecretKey::<Sha384, PSS, Randomized>::from_der(&sk_der)?;
    
    let blind_sig = sk.blind_sign(blinded_message)?;
    Ok(blind_sig.to_vec())
}

pub fn verify_signature(
    pk_der_b64: &str,
    signature: &[u8],
    msg_randomizer: &[u8],
    message: &[u8]
) -> Result<()> {
    let pk_der = base64::engine::general_purpose::STANDARD.decode(pk_der_b64)?;
    let pk = PublicKey::<Sha384, PSS, Randomized>::from_der(&pk_der)?;
    
    let sig = Signature::from(signature.to_vec());
    let randomizer_bytes: [u8; 32] = msg_randomizer.try_into()
        .context("Invalid message randomizer length (expected 32 bytes)")?;
    let randomizer = MessageRandomizer::from(randomizer_bytes);
    
    // RFC 9474 requires msg_randomizer for verification in Randomized mode
    pk.verify(&sig, Some(randomizer), message)?;
    
    Ok(())
}

pub fn generate_nonce() -> [u8; 32] {
    let mut nonce = [0u8; 32];
    rand::rng().fill(&mut nonce);
    nonce
}
