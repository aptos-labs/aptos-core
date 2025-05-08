macro_rules! define_u256 {
    ($name: ident) => {
        #[derive(Clone, serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq, Copy)]
        pub struct $name(pub [u8; 32]);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", hex::encode(&self.0))
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", hex::encode(&self.0))
            }
        }

        impl $name {
            pub fn from_bytes(bytes: &[u8]) -> Self {
                let mut b = [0u8; 32];
                b.copy_from_slice(bytes);
                Self(b)
            }

            pub fn random() -> Self {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let random_bytes: [u8; 32] = rng.gen();
                Self(random_bytes)
            }

            pub fn new(res: [u8; 32]) -> Self {
                Self(res)
            }
        
            pub fn bytes(&self) -> [u8; 32] {
                self.0.clone()
            }

            pub fn as_bytes(&self) -> &[u8] {
                self.0.as_ref()
            }
        }
    };
}

define_u256!(TxnHash);
define_u256!(BlockId);
define_u256!(AccountAddress);
define_u256!(HashValue);
define_u256!(Random);