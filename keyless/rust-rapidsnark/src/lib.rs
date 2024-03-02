
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]


mod cpp {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
    unsafe impl Send for FullProverImpl {}
    unsafe impl Send for FullProver {}
}

use std::ffi::{CString, CStr};
use thiserror::Error;


pub type ProverResponse<'a> = Result<&'a str, ProverError>;


#[derive(Debug, Error)]
pub enum ProverInitError {
    #[error("Problem loading the prover key")]
    ZKeyFileLoadError,
    #[error("Prover key is using an unsupported curve")]
    UnsupportedZKeyCurve,
    #[error("Unknown error")]
    Unknown
}

#[derive(Debug, Error)]
pub enum ProverError {
    #[error("Prover is not ready")]
    ProverNotReady,
    #[error("Invalid input")]
    InvalidInput,
    #[error("There was a problem with the witness generation binary")]
    WitnessGenerationBinaryProblem,
    #[error("Witness generation outputted with an invalid curve")]
    WitnessGenerationInvalidCurve,
    #[error("Unknown error: {0}")]
    Unknown(&'static str)
}


pub struct FullProver {
    _full_prover : cpp::FullProver
}

impl FullProver {
    pub fn new(zkey_path : &str, witness_gen_binary_folder_path : &str) -> Result<FullProver, ProverInitError> {
        let zkey_path_cstr = CString::new(zkey_path).expect("CString::new failed");
        let wgbfp_cstr = CString::new(witness_gen_binary_folder_path).expect("CString::new failed");
        let full_prover = unsafe { 
            FullProver { 
                _full_prover: cpp::FullProver::new(
                                  zkey_path_cstr.as_ptr(),
                                  wgbfp_cstr.as_ptr()
                                  )
            }
        };
        match full_prover._full_prover.state {
            cpp::FullProverState_OK => Ok(full_prover),
            cpp::FullProverState_ZKEY_FILE_LOAD_ERROR => Err(ProverInitError::ZKeyFileLoadError),
            cpp::FullProverState_UNSUPPORTED_ZKEY_CURVE => Err(ProverInitError::UnsupportedZKeyCurve),
            _ => Err(ProverInitError::Unknown)
        }
    }

    pub fn prove(&mut self, input: &str) -> Result<(&str, cpp::ProverResponseMetrics), ProverError> {
        let input_cstr = CString::new(input).expect("CString::new failed");
        let response = unsafe {
            self._full_prover.prove(input_cstr.as_ptr())
        };
        match response.type_ {
            cpp::ProverResponseType_SUCCESS => unsafe { Ok((CStr::from_ptr(response.raw_json).to_str().expect("CStr::to_str failed"), response.metrics)) },
            cpp::ProverResponseType_ERROR => match response.error {
                cpp::ProverError_NONE => Err(ProverError::Unknown("c++ rapidsnark prover returned \"error\" response type but error is \"none\"")),
                cpp::ProverError_PROVER_NOT_READY => Err(ProverError::ProverNotReady),
                cpp::ProverError_INVALID_INPUT => Err(ProverError::InvalidInput),
                cpp::ProverError_WITNESS_GENERATION_BINARY_PROBLEM => Err(ProverError::WitnessGenerationBinaryProblem),
                cpp::ProverError_WITNESS_GENERATION_INVALID_CURVE => Err(ProverError::WitnessGenerationInvalidCurve),
                _ => Err(ProverError::Unknown("c++ rapidsnark prover returned an unknown error code"))
            },
            _ =>  Err(ProverError::Unknown("c++ rapidsnark prover returned an unknown error code"))
        }
    }
}

impl Drop for FullProver {
    fn drop(&mut self) {
        unsafe {
            self._full_prover.destruct();
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;


    #[test]
    fn sanity_test() {
        let arg1 = "/home/rex_fernando_aptoslabs_com/main_c.zkey";
        let arg2 = "/home/rex_fernando_aptoslabs_com/main_c_cpp";
        let input = fs::read_to_string("/home/rex_fernando_aptoslabs_com/input.json").expect("reading input.json failed");
        unsafe { 
            println!("starting test");
            println!("calling constructor");
            let mut full_prover = FullProver::new(arg1, arg2).expect("full_prover::new failed");
            println!("calling prove method");
            let response = full_prover.prove(input.as_str());
            println!("response: {:?}", response);
        }
        
    }
}
