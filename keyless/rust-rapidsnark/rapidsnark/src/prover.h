#ifndef PROVER_HPP
#define PROVER_HPP

#ifdef __cplusplus
extern "C" {
#endif

//Error codes returned by the functions.
#define PROVER_OK                     0x0
#define PROVER_ERROR                  0x1
#define PROVER_ERROR_SHORT_BUFFER     0x2
#define PROVER_INVALID_WITNESS_LENGTH 0x3

/**
 * Calculates buffer size to output public signals as json string
 * @returns buffer size in bytes or 0 in case of an error
 */
unsigned long CalcPublicBufferSize(const void *zkey_buffer, unsigned long zkey_size);

/**
 * groth16_prover
 * @return error code:
 *         PROVER_OK - in case of success
 *         PPOVER_ERROR - in case of an error
 *         PROVER_ERROR_SHORT_BUFFER - in case of a short buffer error, also updates proof_size and public_size with actual proof and public sizess
 */
int
groth16_prover(const void *zkey_buffer,   unsigned long  zkey_size,
               const void *wtns_buffer,   unsigned long  wtns_size,
               char       *proof_buffer,  unsigned long *proof_size,
               char       *public_buffer, unsigned long *public_size,
               char       *error_msg,     unsigned long  error_msg_maxsize);

#ifdef __cplusplus
}
#endif


#endif // PROVER_HPP
