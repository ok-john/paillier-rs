use crate::{mod_in, Ciphertext, PublicKey};
use serde::{Deserialize, Serialize};
use unknown_order::BigNumber;
use zeroize::Zeroize;

/// A Paillier secret key
#[derive(Clone, Debug, Deserialize, Serialize, Zeroize)]
#[zeroize(drop)]
pub struct SecretKey {
    pub(crate) pk: PublicKey,
    /// lcm(P - 1, Q - 1)
    pub(crate) lambda: BigNumber,
    /// Euler's totient: (P - 1)(Q - 1)
    pub(crate) totient: BigNumber,
    /// L((N + 1)^lambda mod N^2)-1 mod N
    pub(crate) u: BigNumber,
}

impl SecretKey {
    /// Create a new random key
    pub fn random() -> Option<Self> {
        let p = BigNumber::safe_prime(1024);
        let q = BigNumber::safe_prime(1024);
        Self::with_safe_primes(&p, &q)
    }

    /// Create a new key from two safe primes
    pub fn with_safe_primes(p: &BigNumber, q: &BigNumber) -> Option<Self> {
        debug_assert!(p.is_prime());
        debug_assert!(q.is_prime());
        let pm1: BigNumber = p - 1;
        let qm1: BigNumber = q - 1;
        let n = p * q;
        let nn = &n * &n;
        let pk = PublicKey {
            n: n.clone(),
            nn: nn.clone(),
        };
        let lambda = pm1.lcm(&qm1);
        if lambda.is_zero() {
            return None;
        }
        let totient = &pm1 * &qm1;

        // (N+1)^lambda mod N^2
        let t: BigNumber = &n + 1;
        let tt = t.modpow(&lambda, &nn);

        // L((N+1)^lambda mod N^2)^-1 mod N
        let uu = pk.l(&tt).map(|uu| uu.invert(&n));
        match uu {
            None => None,
            Some(u_inv) => match u_inv {
                None => None,
                Some(u) => Some(SecretKey {
                    pk,
                    lambda,
                    totient,
                    u,
                }),
            },
        }
    }

    /// Reverse ciphertext to plaintext
    pub fn decrypt(&self, c: &Ciphertext) -> Option<Vec<u8>> {
        if !mod_in(&c, &self.pk.nn) {
            return None;
        }

        // a = c^\lambda mod n^2
        let a = c.modpow(&self.lambda, &self.pk.nn);
        // ell = L(a, N)
        self.pk.l(&a).map(|l| {
            // m = lu = L(a)*u = L(c^\lamba*)u mod n
            let m = l.modmul(&self.u, &self.pk.n);
            m.to_bytes()
        })
    }

    /// Get this key's byte representation
    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes = SecretKeyBytes {
            n: self.pk.n.to_bytes(),
            lambda: self.lambda.to_bytes(),
            totient: self.totient.to_bytes(),
            u: self.u.to_bytes(),
        };
        serde_bare::to_vec(&bytes).unwrap()
    }

    /// Convert a byte representation to a secret key
    pub fn from_bytes<B: AsRef<[u8]>>(data: B) -> Result<Self, String> {
        let data = data.as_ref();
        let bytes = serde_bare::from_slice::<SecretKeyBytes>(data).map_err(|e| e.to_string())?;
        let pk = PublicKey::from_bytes(bytes.n.as_slice());
        Ok(Self {
            pk,
            lambda: BigNumber::from_slice(bytes.lambda.as_slice()),
            totient: BigNumber::from_slice(bytes.totient.as_slice()),
            u: BigNumber::from_slice(bytes.u.as_slice()),
        })
    }

    /// The Paillier modulus
    pub fn n(&self) -> &BigNumber {
        self.pk.n()
    }

    /// The Paillier `lambda`
    pub fn lambda(&self) -> &BigNumber {
        &self.lambda
    }

    /// The Paillier `totient`
    pub fn totient(&self) -> &BigNumber {
        &self.totient
    }

    /// The Paillier `u`
    pub fn u(&self) -> &BigNumber {
        &self.u
    }
}

#[derive(Serialize, Deserialize)]
struct SecretKeyBytes {
    n: Vec<u8>,
    lambda: Vec<u8>,
    totient: Vec<u8>,
    u: Vec<u8>,
}
