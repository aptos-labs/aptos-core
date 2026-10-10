// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* The Lean half of the mono-move-lean-link boundary: wraps the adapter's
 * plain C ABI into a Lean `IO ByteArray` call. Only owned bytes cross the
 * boundary; the response buffer is copied into a Lean `ByteArray` and
 * returned to the adapter's deallocator before returning. The file avoids
 * libc headers beyond what `lean/lean.h` already pulls in, because `leanc`
 * runs its compiler without the system header search paths. */

#include <lean/lean.h>

typedef struct {
    uint8_t *data;
    size_t len;
} leaner_buffer;

extern uint32_t leaner_monovm_abi_version(void);
extern int32_t leaner_monovm_run(const uint8_t *request, size_t request_len,
                                 leaner_buffer *response);
extern void leaner_monovm_buffer_free(leaner_buffer response);

static lean_obj_res transport_error(void) {
    return lean_io_result_mk_error(
        lean_mk_io_user_error(lean_mk_string(
            "the mono-move adapter could not produce a response; rebuild it "
            "with the lake native target")));
}

/* The `@[extern "leaner_monovm_run_bytes"]` implementation for
 * `runBytes : (@& ByteArray) → IO ByteArray`. */
lean_obj_res leaner_monovm_run_bytes(b_lean_obj_arg request, b_lean_obj_arg w) {
    (void)w;
    const uint8_t *request_bytes = lean_sarray_cptr(request);
    size_t request_len = lean_sarray_size(request);
    leaner_buffer response;
    int32_t status = leaner_monovm_run(request_bytes, request_len, &response);
    if (status != 0 || response.data == NULL) {
        if (response.data != NULL) {
            leaner_monovm_buffer_free(response);
        }
        return transport_error();
    }
    lean_object *result = lean_alloc_sarray(1, response.len, response.len);
    uint8_t *dst = lean_sarray_cptr(result);
    for (size_t i = 0; i < response.len; i++) {
        dst[i] = response.data[i];
    }
    leaner_monovm_buffer_free(response);
    return lean_io_result_mk_ok(result);
}
