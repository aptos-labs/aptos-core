use const_oid::ObjectIdentifier;

const fn oid(s: &str) -> ObjectIdentifier {
    ObjectIdentifier::new_unwrap(s)
}

pub const SGX_EXTENSION: ObjectIdentifier = oid("1.2.840.113741.1.13.1");
pub const PPID: ObjectIdentifier = oid("1.2.840.113741.1.13.1.1");
pub const TCB: ObjectIdentifier = oid("1.2.840.113741.1.13.1.2");
pub const PCEID: ObjectIdentifier = oid("1.2.840.113741.1.13.1.3");
pub const FMSPC: ObjectIdentifier = oid("1.2.840.113741.1.13.1.4");
pub const SGX_TYPE: ObjectIdentifier = oid("1.2.840.113741.1.13.1.5");
pub const PLATFORM_INSTANCE_ID: ObjectIdentifier = oid("1.2.840.113741.1.13.1.6");
/// Configuration sequence (Platform CA certs only)
pub const CONFIGURATION: ObjectIdentifier = oid("1.2.840.113741.1.13.1.7");
/// Whether platform can be extended with additional packages
pub const DYNAMIC_PLATFORM: ObjectIdentifier = oid("1.2.840.113741.1.13.1.7.1");
/// Whether platform root keys are cached by SGX Registration Backend
pub const CACHED_KEYS: ObjectIdentifier = oid("1.2.840.113741.1.13.1.7.2");
/// Whether platform has SMT (simultaneous multithreading) enabled
pub const SMT_ENABLED: ObjectIdentifier = oid("1.2.840.113741.1.13.1.7.3");
pub const PCESVN: ObjectIdentifier = oid("1.2.840.113741.1.13.1.2.17");
pub const CPUSVN: ObjectIdentifier = oid("1.2.840.113741.1.13.1.2.18");
