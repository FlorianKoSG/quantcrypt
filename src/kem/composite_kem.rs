use crate::kem::asn1::asn_util::oid_to_der;
use crate::kem::asn1::composite_kem_primitives::{
    CompositeCiphertextValue, CompositeKEMPrivateKey, CompositeKEMPublicKey,
};
use crate::kem::common::kdf::{Kdf, KdfType};
use crate::kem::common::kem_info::KemInfo;
use crate::kem::common::kem_trait::Kem;
use crate::kem::common::kem_type::KemType;
use crate::kem::kem_factory::KemManager;
use der::{Decode, Encode};
use pkcs8::{AlgorithmIdentifierRef, PrivateKeyInfo};
use rand_core::CryptoRngCore;

use std::error;

use super::kem_factory::KemFactory;
// Change the alias to use `Box<dyn error::Error>`.
type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

/// A KEM manager for the composite KEM method
pub struct CompositeKemManager {
    /// The KEM metadata information
    kem_info: KemInfo,
    /// The traditional KEM manager
    trad_kem: Box<KemManager>,
    /// The post-quantum KEM manager
    pq_kem: Box<KemManager>,
    /// The key derivation function
    kdf: Kdf,
}

impl CompositeKemManager {
    /// See the combiner function in the RFC:
    /// https://lamps-wg.github.io/draft-composite-kem/draft-ietf-lamps-pq-composite-kem.html
    ///
    /// The combiner function is used to combine the shared secrets from the traditional and post-quantum KEMs
    ///
    /// # Arguments
    ///
    /// * `pq_kem_ss` - The shared secret from the post-quantum KEM
    /// * `trad_kem_ss` - The shared secret from the traditional KEM
    /// * `trad_ct` - The traditional ciphertext
    /// * `trad_pk` - The traditional public key (this should exist in the OneAsymmetricKey object)
    ///
    /// # Returns
    ///
    /// The combined shared secret (ss) after applying the KDF
    pub fn combiner(
        &self,
        pq_kem_ss: &[u8],
        trad_kem_ss: &[u8],
        trad_ct: &[u8],
        trad_pk: &[u8],
    ) -> Result<Vec<u8>> {
        let mut combined_ss: Vec<u8> = Vec::new();
        combined_ss.extend_from_slice(pq_kem_ss);
        combined_ss.extend_from_slice(trad_kem_ss);
        combined_ss.extend_from_slice(trad_ct);
        combined_ss.extend_from_slice(trad_pk);

        let dom_sep = oid_to_der(&self.kem_info.oid)?;
        combined_ss.extend_from_slice(&dom_sep);

        let ss = self.kdf.kdf(&combined_ss);

        Ok(ss)
    }
}

impl CompositeKemManager {
    /// Generate a composite KEM keypair from constituent keys
    ///
    /// # Arguments
    ///
    /// * `t_pk` - The traditional public key
    /// * `t_sk` - The traditional secret key
    /// * `pq_pk` - The post-quantum public key
    /// * `pq_sk` - The post-quantum secret key
    ///
    /// # Returns
    ///
    /// A tuple containing the composite public key and secret key. It is CompositeKEMPublicKey, CompositeKEMPrivateKey
    /// objects in ASN.1 format converted to DER
    fn key_gen_composite(
        &self,
        t_pk: &[u8],
        t_sk: &[u8],
        pq_pk: &[u8],
        pq_sk: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        // Create the composite public key
        let c_pk = CompositeKEMPublicKey::new(pq_pk, t_pk);
        let pk = c_pk.to_der()?;

        // Create the OneAsymmetricKey objects for the tradition secret key
        let t_sk_pkcs8 = PrivateKeyInfo {
            algorithm: AlgorithmIdentifierRef {
                oid: self.trad_kem.get_kem_info().oid.parse().unwrap(),
                parameters: None,
            },
            private_key: t_sk,
            // The public key SHOULD be included in the secret key for the traditional KEM
            public_key: Some(t_pk),
        };

        // Create the OneAsymmetricKey objects for the post-quantum secret key
        let pq_sk_pkcs8 = PrivateKeyInfo {
            algorithm: AlgorithmIdentifierRef {
                oid: self.pq_kem.get_kem_info().oid.parse().unwrap(),
                parameters: None,
            },
            private_key: pq_sk,
            public_key: None,
        };

        // Create the composite secret key
        let c_sk: CompositeKEMPrivateKey<'_> = CompositeKEMPrivateKey::new(pq_sk_pkcs8, t_sk_pkcs8);
        let sk = c_sk.to_der()?;

        Ok((pk, sk))
    }
}

impl Kem for CompositeKemManager {
    /// Create a new KEM instance
    ///
    /// # Arguments
    ///
    /// * `kem_type` - The type of KEM to create
    ///
    /// # Returns
    ///
    /// A new KEM instance
    ///
    /// # Panics
    ///
    /// If the KEM type is not supported (should be a composite KEM)
    fn new(kem_type: KemType) -> Self {
        let kem_info = KemInfo::new(kem_type.clone());
        match kem_type {
            KemType::MlKem768Rsa2048 => Self {
                kem_info,
                trad_kem: Box::new(KemFactory::get_kem(KemType::RsaOAEP2048)),
                pq_kem: Box::new(KemFactory::get_kem(KemType::MlKem768)),
                kdf: Kdf::new(KdfType::HkdfSha256),
            },
            KemType::MlKem768Rsa3072 => Self {
                kem_info,
                trad_kem: Box::new(KemFactory::get_kem(KemType::RsaOAEP3072)),
                pq_kem: Box::new(KemFactory::get_kem(KemType::MlKem768)),
                kdf: Kdf::new(KdfType::HkdfSha256),
            },
            KemType::MlKem768Rsa4096 => Self {
                kem_info,
                trad_kem: Box::new(KemFactory::get_kem(KemType::RsaOAEP4096)),
                pq_kem: Box::new(KemFactory::get_kem(KemType::MlKem768)),
                kdf: Kdf::new(KdfType::HkdfSha256),
            },
            KemType::MlKem768X25519 => Self {
                kem_info,
                trad_kem: Box::new(KemFactory::get_kem(KemType::X25519)),
                pq_kem: Box::new(KemFactory::get_kem(KemType::MlKem768)),
                kdf: Kdf::new(KdfType::Sha3_256),
            },
            KemType::MlKem768P384 => Self {
                kem_info,
                trad_kem: Box::new(KemFactory::get_kem(KemType::P384)),
                pq_kem: Box::new(KemFactory::get_kem(KemType::MlKem768)),
                kdf: Kdf::new(KdfType::HkdfSha384),
            },
            KemType::MlKem768BrainpoolP256r1 => Self {
                kem_info,
                trad_kem: Box::new(KemFactory::get_kem(KemType::BrainpoolP256r1)),
                pq_kem: Box::new(KemFactory::get_kem(KemType::MlKem768)),
                kdf: Kdf::new(KdfType::HkdfSha384),
            },
            KemType::MlKem1024P384 => Self {
                kem_info,
                trad_kem: Box::new(KemFactory::get_kem(KemType::P384)),
                pq_kem: Box::new(KemFactory::get_kem(KemType::MlKem1024)),
                kdf: Kdf::new(KdfType::Sha3_512),
            },
            KemType::MlKem1024BrainpoolP384r1 => Self {
                kem_info,
                trad_kem: Box::new(KemFactory::get_kem(KemType::BrainpoolP384r1)),
                pq_kem: Box::new(KemFactory::get_kem(KemType::MlKem1024)),
                kdf: Kdf::new(KdfType::Sha3_512),
            },
            KemType::MlKem1024X448 => Self {
                kem_info,
                trad_kem: Box::new(KemFactory::get_kem(KemType::X448)),
                pq_kem: Box::new(KemFactory::get_kem(KemType::MlKem1024)),
                kdf: Kdf::new(KdfType::Sha3_512),
            },
            _ => {
                panic!("Not implemented");
            }
        }
    }

    /// Generate a composite KEM keypair using the default RNGs of the
    /// traditional and post-quantum KEMs of the composite KEM
    ///
    /// # Returns
    ///
    /// A tuple containing the composite public key and secret key (pk, sk).
    /// It is CompositeKEMPublicKey, CompositeKEMPrivateKey objects in ASN.1
    /// format converted to DER
    fn key_gen(&mut self) -> Result<(Vec<u8>, Vec<u8>)> {
        // Get the keypair for the traditional KEM
        let (t_pk, t_sk) = self.trad_kem.key_gen()?;

        // Get the keypair for the post-quantum KEM
        let (pq_pk, pq_sk) = self.pq_kem.key_gen()?;

        self.key_gen_composite(&t_pk, &t_sk, &pq_pk, &pq_sk)
    }

    /// Generate a composite KEM keypair
    ///
    /// # Arguments
    ///
    /// * `rng` - The random number generator to use
    ///
    /// # Returns
    ///
    /// A tuple containing the composite public key and secret key (pk, sk).
    /// It is CompositeKEMPublicKey, CompositeKEMPrivateKey objects in ASN.1
    /// format converted to DER
    ///
    /// The keys are composite keys in ASN.1 format:
    /// CompositeKEMPublicKey ::= SEQUENCE SIZE (2) OF BIT STRING
    /// CompositeKEMPrivateKey ::= SEQUENCE SIZE (2) OF OneAsymmetricKey
    ///
    /// OneAsymmetricKey ::= SEQUENCE {
    ///    version                   Version,
    ///    privateKeyAlgorithm       PrivateKeyAlgorithmIdentifier,
    ///    privateKey                PrivateKey,
    ///    attributes            [0] Attributes OPTIONAL,
    ///    ...,
    ///    [[2: publicKey        [1] PublicKey OPTIONAL ]],
    ///    ...
    fn key_gen_with_rng(&mut self, rng: &mut impl CryptoRngCore) -> Result<(Vec<u8>, Vec<u8>)> {
        // Get the keypair for the traditional KEM
        let (t_pk, t_sk) = self.trad_kem.key_gen_with_rng(rng)?;

        // Get the keypair for the post-quantum KEM
        let (pq_pk, pq_sk) = self.pq_kem.key_gen_with_rng(rng)?;

        self.key_gen_composite(&t_pk, &t_sk, &pq_pk, &pq_sk)
    }

    /// Encapsulate a public key
    ///
    /// # Arguments
    ///
    /// * `pk` - The composite public key to encapsulate
    ///
    /// # Returns
    ///
    /// A tuple containing the shared secret and ciphertext (ss, ct).
    /// The shared secret is the result of the combiner function, and the
    /// ciphertext is the CompositeCiphertextValue in ASN.1 format converted to DER
    fn encap(&mut self, pk: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        // Deserialize the composite public key
        let c_pk = CompositeKEMPublicKey::from_der(pk)?;

        // Encapsulate the public key for the traditional KEM
        let (t_ss, t_ct) = self.trad_kem.encap(&c_pk.get_trad_pk())?;

        // Encapsulate the public key for the post-quantum KEM
        let (pq_ss, pq_ct) = self.pq_kem.encap(&c_pk.get_pq_pk())?;

        // Create the composite ciphertext
        let ct = CompositeCiphertextValue::new(&pq_ct, &t_ct);
        let ct = ct.to_der()?;

        // Get the shared secret using the combiner
        let ss = self.combiner(&pq_ss, &t_ss, &t_ct, &c_pk.get_trad_pk())?;

        Ok((ss, ct))
    }

    /// Decapsulate a ciphertext
    ///
    /// # Arguments
    ///
    /// * `sk` - The composite secret key to decapsulate - CompositeKEMPrivateKey in ASN.1 format converted to DER
    /// * `ct` - The composite ciphertext to decapsulate - CompositeCiphertextValue in ASN.1 format converted to DER
    ///
    /// # Returns
    ///
    /// The shared secret after applying the combiner function
    fn decap(&self, sk: &[u8], ct: &[u8]) -> Result<Vec<u8>> {
        // Deserialize the composite secret key
        let c_sk = CompositeKEMPrivateKey::from_der(sk).unwrap();

        // Deserialize the composite ciphertext
        let c_ct = CompositeCiphertextValue::from_der(ct).unwrap();

        // Decapsulate the ciphertext for the traditional KEM
        let t_ss = self
            .trad_kem
            .decap(c_sk.get_trad_sk().private_key, &c_ct.get_trad_ct())
            .unwrap();

        // Decapsulate the ciphertext for the post-quantum KEM
        let pq_ss = self
            .pq_kem
            .decap(c_sk.get_pq_sk().private_key, &c_ct.get_pq_ct())
            .unwrap();

        // Get the trad PK
        let t_pk = c_sk.get_trad_sk().public_key.unwrap();

        // Get the shared secret using the combiner
        let ss = self
            .combiner(&pq_ss, &t_ss, &c_ct.get_trad_ct(), t_pk)
            .unwrap();

        Ok(ss)
    }

    /// Get KEM metadata information such as the key lengths,
    ///
    /// These values are also used to test the correctness of the KEM
    ///
    /// # Returns
    ///
    /// A structure containing metadata about the KEM
    fn get_kem_info(&self) -> KemInfo {
        self.kem_info.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kem::common::macros::test_kem;

    #[test]
    fn test_mlkem_768_rsa2048() {
        let mut kem = CompositeKemManager::new(KemType::MlKem768Rsa2048);
        test_kem!(&mut kem);
    }

    #[test]
    fn test_mlkem_768_rsa3072() {
        let mut kem = CompositeKemManager::new(KemType::MlKem768Rsa3072);
        test_kem!(&mut kem);
    }

    #[test]
    fn test_mlkem_768_rsa4096() {
        let mut kem = CompositeKemManager::new(KemType::MlKem768Rsa4096);
        test_kem!(&mut kem);
    }

    #[test]
    fn test_mlkem_768_x25519() {
        let mut kem = CompositeKemManager::new(KemType::MlKem768X25519);
        test_kem!(&mut kem);
    }

    #[test]
    fn test_mlkem_768_p384() {
        let mut kem = CompositeKemManager::new(KemType::MlKem768P384);
        test_kem!(&mut kem);
    }

    #[test]
    fn test_mlkem_768_brainpool_p256r1() {
        let mut kem = CompositeKemManager::new(KemType::MlKem768BrainpoolP256r1);
        test_kem!(&mut kem);
    }

    #[test]
    fn test_mlkem_1024_p384() {
        let mut kem = CompositeKemManager::new(KemType::MlKem1024P384);
        test_kem!(&mut kem);
    }

    #[test]
    fn test_mlkem_1024_brainpool_p384r1() {
        let mut kem = CompositeKemManager::new(KemType::MlKem1024BrainpoolP384r1);
        test_kem!(&mut kem);
    }

    #[test]
    fn test_mlkem_1024_x448() {
        let mut kem = CompositeKemManager::new(KemType::MlKem1024X448);
        test_kem!(&mut kem);
    }
}
