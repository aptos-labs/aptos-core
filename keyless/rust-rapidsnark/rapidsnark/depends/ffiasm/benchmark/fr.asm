

        global Fr_copy
        global Fr_copyn
        global Fr_add
        global Fr_sub
        global Fr_neg
        global Fr_mul
        global Fr_square
        global Fr_band
        global Fr_bor
        global Fr_bxor
        global Fr_bnot
        global Fr_eq
        global Fr_neq
        global Fr_lt
        global Fr_gt
        global Fr_leq
        global Fr_geq
        global Fr_land
        global Fr_lor
        global Fr_lnot
        global Fr_toNormal
        global Fr_toLongNormal
        global Fr_toMontgomery
        global Fr_toInt
        global Fr_isTrue
        global Fr_q

        global Fr_rawCopy
        global Fr_rawSwap
        global Fr_rawAdd
        global Fr_rawSub
        global Fr_rawNeg
        global Fr_rawMMul
        global Fr_rawMSquare
        global Fr_rawToMontgomery
        global Fr_rawFromMontgomery

        extern Fr_fail
        DEFAULT REL

        section .text
















;;;;;;;;;;;;;;;;;;;;;;
; copy
;;;;;;;;;;;;;;;;;;;;;;
; Copies
; Params:
;   rsi <= the src
;   rdi <= the dest
;
; Nidified registers:
;   rax
;;;;;;;;;;;;;;;;;;;;;;;
Fr_copy:

        mov     rax, [rsi + 0]
        mov     [rdi + 0], rax

        mov     rax, [rsi + 8]
        mov     [rdi + 8], rax

        mov     rax, [rsi + 16]
        mov     [rdi + 16], rax

        mov     rax, [rsi + 24]
        mov     [rdi + 24], rax

        mov     rax, [rsi + 32]
        mov     [rdi + 32], rax

        ret


;;;;;;;;;;;;;;;;;;;;;;
; rawCopy
;;;;;;;;;;;;;;;;;;;;;;
; Copies
; Params:
;   rsi <= the src
;   rdi <= the dest
;
; Nidified registers:
;   rax
;;;;;;;;;;;;;;;;;;;;;;;
Fr_rawCopy:

        mov     rax, [rsi + 0]
        mov     [rdi + 0], rax

        mov     rax, [rsi + 8]
        mov     [rdi + 8], rax

        mov     rax, [rsi + 16]
        mov     [rdi + 16], rax

        mov     rax, [rsi + 24]
        mov     [rdi + 24], rax

        ret

;;;;;;;;;;;;;;;;;;;;;;
; rawSwap
;;;;;;;;;;;;;;;;;;;;;;
; Copies
; Params:
;   rdi <= a
;   rsi <= p
;
; Nidified registers:
;   rax
;;;;;;;;;;;;;;;;;;;;;;;
Fr_rawSwap:

        mov     rax, [rsi + 0]
        mov     rcx, [rdi + 0]
        mov     [rdi + 0], rax
        mov     [rsi + 0], rbx

        mov     rax, [rsi + 8]
        mov     rcx, [rdi + 8]
        mov     [rdi + 8], rax
        mov     [rsi + 8], rbx

        mov     rax, [rsi + 16]
        mov     rcx, [rdi + 16]
        mov     [rdi + 16], rax
        mov     [rsi + 16], rbx

        mov     rax, [rsi + 24]
        mov     rcx, [rdi + 24]
        mov     [rdi + 24], rax
        mov     [rsi + 24], rbx

        ret

        
;;;;;;;;;;;;;;;;;;;;;;
; copy an array of integers
;;;;;;;;;;;;;;;;;;;;;;
; Copies
; Params:
;   rsi <= the src
;   rdi <= the dest
;   rdx <= number of integers to copy
;
; Nidified registers:
;   rax
;;;;;;;;;;;;;;;;;;;;;;;
Fr_copyn:
Fr_copyn_loop:
        mov     r8, rsi
        mov     r9, rdi
        mov     rax, 5
        mul     rdx
        mov     rcx, rax
        cld
   rep  movsq
        mov     rsi, r8
        mov     rdi, r9
        ret

;;;;;;;;;;;;;;;;;;;;;;
; rawCopyS2L
;;;;;;;;;;;;;;;;;;;;;;
; Convert a 64 bit integer to a long format field element
; Params:
;   rsi <= the integer
;   rdi <= Pointer to the overwritted element
;
; Nidified registers:
;   rax
;;;;;;;;;;;;;;;;;;;;;;;

rawCopyS2L:
        mov     al, 0x80
        shl     rax, 56
        mov     [rdi], rax    ; set the result to LONG normal

        cmp     rsi, 0
        js      u64toLong_adjust_neg

        mov     [rdi + 8], rsi
        xor     rax, rax

        mov     [rdi + 16], rax

        mov     [rdi + 24], rax

        mov     [rdi + 32], rax

        ret

u64toLong_adjust_neg:
        add    rsi, [q]         ; Set the first digit
        mov    [rdi + 8], rsi   ;

        mov    rsi, -1          ; all ones

        mov    rax, rsi                       ; Add to q
        adc    rax, [q + 8 ]
        mov    [rdi + 16], rax

        mov    rax, rsi                       ; Add to q
        adc    rax, [q + 16 ]
        mov    [rdi + 24], rax

        mov    rax, rsi                       ; Add to q
        adc    rax, [q + 24 ]
        mov    [rdi + 32], rax

        ret

;;;;;;;;;;;;;;;;;;;;;;
; toInt
;;;;;;;;;;;;;;;;;;;;;;
; Convert a 64 bit integer to a long format field element
; Params:
;   rsi <= Pointer to the element
; Returs:
;   rax <= The value
;;;;;;;;;;;;;;;;;;;;;;;
Fr_toInt:
        mov     rax, [rdi]
        bt      rax, 63
        jc      Fr_long
        movsx   rax, eax
        ret

Fr_long:
        bt      rax, 62
        jnc     Fr_longNormal
Fr_longMontgomery:
        call    Fr_toLongNormal

Fr_longNormal:
        mov     rax, [rdi + 8]
        mov     rcx, rax
        shr     rcx, 31
        jnz     Fr_longNeg

        mov     rcx, [rdi + 16]
        test    rcx, rcx
        jnz     Fr_longNeg

        mov     rcx, [rdi + 24]
        test    rcx, rcx
        jnz     Fr_longNeg

        mov     rcx, [rdi + 32]
        test    rcx, rcx
        jnz     Fr_longNeg

        ret

Fr_longNeg:
        mov     rax, [rdi + 8]
        sub     rax, [q]
        jnc     Fr_longErr

        mov     rcx, [rdi + 16]
        sbb     rcx, [q + 8]
        jnc     Fr_longErr

        mov     rcx, [rdi + 24]
        sbb     rcx, [q + 16]
        jnc     Fr_longErr

        mov     rcx, [rdi + 32]
        sbb     rcx, [q + 24]
        jnc     Fr_longErr

        mov     rcx, rax
        sar     rcx, 31
        add     rcx, 1
        jnz     Fr_longErr
        ret

Fr_longErr:
        push    rdi
        mov     rdi, 0
        call    Fr_fail
        pop     rdi





Fr_rawMMul:
    push r15
    push r14
    push r13
    push r12
    mov rcx,rdx
    mov r9,[ np ]
    xor r10,r10

; FirstLoop
    mov rdx,[rsi + 0]
    mulx rax,r11,[rcx]
    mulx r8,r12,[rcx +8]
    adcx r12,rax
    mulx rax,r13,[rcx +16]
    adcx r13,r8
    mulx r8,r14,[rcx +24]
    adcx r14,rax
    mov r15,r10
    adcx r15,r8
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

; FirstLoop
    mov rdx,[rsi + 8]
    mov r15,r10
    mulx r8,rax,[rcx +0]
    adcx r11,rax
    adox r12,r8
    mulx r8,rax,[rcx +8]
    adcx r12,rax
    adox r13,r8
    mulx r8,rax,[rcx +16]
    adcx r13,rax
    adox r14,r8
    mulx r8,rax,[rcx +24]
    adcx r14,rax
    adox r15,r8
    adcx r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

; FirstLoop
    mov rdx,[rsi + 16]
    mov r15,r10
    mulx r8,rax,[rcx +0]
    adcx r11,rax
    adox r12,r8
    mulx r8,rax,[rcx +8]
    adcx r12,rax
    adox r13,r8
    mulx r8,rax,[rcx +16]
    adcx r13,rax
    adox r14,r8
    mulx r8,rax,[rcx +24]
    adcx r14,rax
    adox r15,r8
    adcx r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

; FirstLoop
    mov rdx,[rsi + 24]
    mov r15,r10
    mulx r8,rax,[rcx +0]
    adcx r11,rax
    adox r12,r8
    mulx r8,rax,[rcx +8]
    adcx r12,rax
    adox r13,r8
    mulx r8,rax,[rcx +16]
    adcx r13,rax
    adox r14,r8
    mulx r8,rax,[rcx +24]
    adcx r14,rax
    adox r15,r8
    adcx r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

;comparison
    cmp r14,[q + 24]
    jc Fr_rawMMul_done
    jnz Fr_rawMMul_sq
    cmp r13,[q + 16]
    jc Fr_rawMMul_done
    jnz Fr_rawMMul_sq
    cmp r12,[q + 8]
    jc Fr_rawMMul_done
    jnz Fr_rawMMul_sq
    cmp r11,[q + 0]
    jc Fr_rawMMul_done
    jnz Fr_rawMMul_sq
Fr_rawMMul_sq:
    sub r11,[q +0]
    sbb r12,[q +8]
    sbb r13,[q +16]
    sbb r14,[q +24]
Fr_rawMMul_done:
    mov [rdi + 0],r11
    mov [rdi + 8],r12
    mov [rdi + 16],r13
    mov [rdi + 24],r14
    pop r12
    pop r13
    pop r14
    pop r15
    ret
Fr_rawMSquare:
    push r15
    push r14
    push r13
    push r12
    mov rcx,rdx
    mov r9,[ np ]
    xor r10,r10

; FirstLoop
    mov rdx,[rsi + 0]
    mulx rax,r11,rdx
    mulx r8,r12,[rsi +8]
    adcx r12,rax
    mulx rax,r13,[rsi +16]
    adcx r13,r8
    mulx r8,r14,[rsi +24]
    adcx r14,rax
    mov r15,r10
    adcx r15,r8
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

; FirstLoop
    mov rdx,[rsi + 8]
    mov r15,r10
    mulx r8,rax,[rsi +0]
    adcx r11,rax
    adox r12,r8
    mulx r8,rax,[rsi +8]
    adcx r12,rax
    adox r13,r8
    mulx r8,rax,[rsi +16]
    adcx r13,rax
    adox r14,r8
    mulx r8,rax,[rsi +24]
    adcx r14,rax
    adox r15,r8
    adcx r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

; FirstLoop
    mov rdx,[rsi + 16]
    mov r15,r10
    mulx r8,rax,[rsi +0]
    adcx r11,rax
    adox r12,r8
    mulx r8,rax,[rsi +8]
    adcx r12,rax
    adox r13,r8
    mulx r8,rax,[rsi +16]
    adcx r13,rax
    adox r14,r8
    mulx r8,rax,[rsi +24]
    adcx r14,rax
    adox r15,r8
    adcx r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

; FirstLoop
    mov rdx,[rsi + 24]
    mov r15,r10
    mulx r8,rax,[rsi +0]
    adcx r11,rax
    adox r12,r8
    mulx r8,rax,[rsi +8]
    adcx r12,rax
    adox r13,r8
    mulx r8,rax,[rsi +16]
    adcx r13,rax
    adox r14,r8
    mulx r8,rax,[rsi +24]
    adcx r14,rax
    adox r15,r8
    adcx r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

;comparison
    cmp r14,[q + 24]
    jc Fr_rawMSquare_done
    jnz Fr_rawMSquare_sq
    cmp r13,[q + 16]
    jc Fr_rawMSquare_done
    jnz Fr_rawMSquare_sq
    cmp r12,[q + 8]
    jc Fr_rawMSquare_done
    jnz Fr_rawMSquare_sq
    cmp r11,[q + 0]
    jc Fr_rawMSquare_done
    jnz Fr_rawMSquare_sq
Fr_rawMSquare_sq:
    sub r11,[q +0]
    sbb r12,[q +8]
    sbb r13,[q +16]
    sbb r14,[q +24]
Fr_rawMSquare_done:
    mov [rdi + 0],r11
    mov [rdi + 8],r12
    mov [rdi + 16],r13
    mov [rdi + 24],r14
    pop r12
    pop r13
    pop r14
    pop r15
    ret
Fr_rawMMul1:
    push r15
    push r14
    push r13
    push r12
    mov rcx,rdx
    mov r9,[ np ]
    xor r10,r10

; FirstLoop
    mov rdx,rcx
    mulx rax,r11,[rsi]
    mulx r8,r12,[rsi +8]
    adcx r12,rax
    mulx rax,r13,[rsi +16]
    adcx r13,r8
    mulx r8,r14,[rsi +24]
    adcx r14,rax
    mov r15,r10
    adcx r15,r8
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

    mov r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

    mov r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

    mov r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

;comparison
    cmp r14,[q + 24]
    jc Fr_rawMMul1_done
    jnz Fr_rawMMul1_sq
    cmp r13,[q + 16]
    jc Fr_rawMMul1_done
    jnz Fr_rawMMul1_sq
    cmp r12,[q + 8]
    jc Fr_rawMMul1_done
    jnz Fr_rawMMul1_sq
    cmp r11,[q + 0]
    jc Fr_rawMMul1_done
    jnz Fr_rawMMul1_sq
Fr_rawMMul1_sq:
    sub r11,[q +0]
    sbb r12,[q +8]
    sbb r13,[q +16]
    sbb r14,[q +24]
Fr_rawMMul1_done:
    mov [rdi + 0],r11
    mov [rdi + 8],r12
    mov [rdi + 16],r13
    mov [rdi + 24],r14
    pop r12
    pop r13
    pop r14
    pop r15
    ret
Fr_rawFromMontgomery:
    push r15
    push r14
    push r13
    push r12
    mov rcx,rdx
    mov r9,[ np ]
    xor r10,r10

; FirstLoop
    mov r11,[rsi +0]
    mov r12,[rsi +8]
    mov r13,[rsi +16]
    mov r14,[rsi +24]
    mov r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

    mov r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

    mov r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

    mov r15,r10
; SecondLoop
    mov rdx,r9
    mulx rax,rdx,r11
    mulx r8,rax,[q]
    adcx rax,r11
    mulx rax,r11,[q +8]
    adcx r11,r8
    adox r11,r12
    mulx r8,r12,[q +16]
    adcx r12,rax
    adox r12,r13
    mulx rax,r13,[q +24]
    adcx r13,r8
    adox r13,r14
    mov r14,r10
    adcx r14,rax
    adox r14,r15

;comparison
    cmp r14,[q + 24]
    jc Fr_rawFromMontgomery_done
    jnz Fr_rawFromMontgomery_sq
    cmp r13,[q + 16]
    jc Fr_rawFromMontgomery_done
    jnz Fr_rawFromMontgomery_sq
    cmp r12,[q + 8]
    jc Fr_rawFromMontgomery_done
    jnz Fr_rawFromMontgomery_sq
    cmp r11,[q + 0]
    jc Fr_rawFromMontgomery_done
    jnz Fr_rawFromMontgomery_sq
Fr_rawFromMontgomery_sq:
    sub r11,[q +0]
    sbb r12,[q +8]
    sbb r13,[q +16]
    sbb r14,[q +24]
Fr_rawFromMontgomery_done:
    mov [rdi + 0],r11
    mov [rdi + 8],r12
    mov [rdi + 16],r13
    mov [rdi + 24],r14
    pop r12
    pop r13
    pop r14
    pop r15
    ret

;;;;;;;;;;;;;;;;;;;;;;
; rawToMontgomery
;;;;;;;;;;;;;;;;;;;;;;
; Convert a number to Montgomery
;   rdi <= Pointer destination element 
;   rsi <= Pointer to src element
;;;;;;;;;;;;;;;;;;;;
Fr_rawToMontgomery:
    push    rsi
    lea     rsi, [R2]
    call    Fr_rawMMul
    pop     rsi
    ret

;;;;;;;;;;;;;;;;;;;;;;
; toMontgomery
;;;;;;;;;;;;;;;;;;;;;;
; Convert a number to Montgomery
;   rdi <= Pointer element to convert
; Modified registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;
Fr_toMontgomery:
    mov     rax, [rdi]
    bt      rax, 62                     ; check if montgomery
    jc      toMontgomery_doNothing
    bt      rax, 63
    jc      toMontgomeryLong

toMontgomeryShort:
    add     rdi, 8
    push    rsi
    push    rdx
    lea     rsi, [R2]
    movsx   rdx, eax
    cmp     rdx, 0
    js      negMontgomeryShort
posMontgomeryShort:
    call    Fr_rawMMul1
    pop     rdx
    pop     rsi
    sub     rdi, 8
            mov r11b, 0x40
        shl r11d, 24
        mov [rdi+4], r11d
    ret

negMontgomeryShort:
    neg     rdx              ; Do the multiplication positive and then negate the result.
    call    Fr_rawMMul1
    mov     rsi, rdi
    call    rawNegL
    pop     rdx
    pop     rsi
    sub     rdi, 8
            mov r11b, 0x40
        shl r11d, 24
        mov [rdi+4], r11d
    ret


toMontgomeryLong:
    mov     [rdi], rax
    add     rdi, 8
    push    rsi
    mov     rdx, rdi
    lea     rsi, [R2]
    call    Fr_rawMMul
    pop     rsi
    sub     rdi, 8
            mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d


toMontgomery_doNothing:
    ret

;;;;;;;;;;;;;;;;;;;;;;
; toNormal
;;;;;;;;;;;;;;;;;;;;;;
; Convert a number from Montgomery
;   rdi <= Pointer element to convert
; Modified registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;
Fr_toNormal:
    mov     rax, [rdi]
    bt      rax, 62                     ; check if montgomery
    jnc     toNormal_doNothing
    bt      rax, 63                     ; if short, it means it's converted
    jnc     toNormal_doNothing

toNormalLong:
    add     rdi, 8
    push    rsi
    mov     rsi, rdi
    call    Fr_rawFromMontgomery
    sub     rdi, 8
    pop     rsi
            mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

toNormal_doNothing:
    ret

;;;;;;;;;;;;;;;;;;;;;;
; toLongNormal
;;;;;;;;;;;;;;;;;;;;;;
; Convert a number to long normal
;   rdi <= Pointer element to convert
; Modified registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;
Fr_toLongNormal:
    mov     rax, [rdi]
    bt      rax, 62                     ; check if montgomery
    jc      toLongNormal_fromMontgomery
    bt      rax, 63                     ; check if long
    jnc     toLongNormal_fromShort
    ret                                 ; It is already long

toLongNormal_fromMontgomery:
    add     rdi, 8
    mov     rsi, rdi
    call    Fr_rawFromMontgomery
    sub     rdi, 8
            mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
    ret

toLongNormal_fromShort:
    mov     r8, rsi                     ; save rsi
    movsx   rsi, eax
    call    rawCopyS2L
    mov     rsi, r8                     ; recover rsi
            mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
    ret
    











;;;;;;;;;;;;;;;;;;;;;;
; add
;;;;;;;;;;;;;;;;;;;;;;
; Adds two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result
; Modified Registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_add:
        mov    rax, [rsi]
        mov    rcx, [rdx]
        bt     rax, 63          ; Check if is short first operand
        jc     add_l1
        bt     rcx, 63          ; Check if is short second operand
        jc     add_s1l2

add_s1s2:                       ; Both operands are short

        xor    rdx, rdx
        mov    edx, eax
        add    edx, ecx
        jo     add_manageOverflow   ; rsi already is the 64bits result

        mov    [rdi], rdx       ; not necessary to adjust so just save and return
        ret

add_manageOverflow:                 ; Do the operation in 64 bits
        push   rsi
        movsx  rsi, eax
        movsx  rdx, ecx
        add    rsi, rdx
        call   rawCopyS2L
        pop    rsi
        ret

add_l1:
        bt     rcx, 63          ; Check if is short second operand
        jc     add_l1l2

;;;;;;;;
add_l1s2:
        bt     rax, 62          ; check if montgomery first
        jc     add_l1ms2
add_l1ns2:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        add rsi, 8
        movsx rdx, ecx
        add rdi, 8
        cmp rdx, 0
        
        jns tmp_1
        neg rdx
        call rawSubLS
        sub rdi, 8
        sub rsi, 8
        ret
tmp_1:
        call rawAddLS
        sub rdi, 8
        sub rsi, 8
        ret



add_l1ms2:
        bt     rcx, 62          ; check if montgomery second
        jc     add_l1ms2m
add_l1ms2n:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rdx
        call Fr_toMontgomery
        mov  rdx, rdi
        pop  rdi

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawAddLL
        sub rdi, 8
        sub rsi, 8
        ret


add_l1ms2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawAddLL
        sub rdi, 8
        sub rsi, 8
        ret



;;;;;;;;
add_s1l2:
        bt     rcx, 62          ; check if montgomery second
        jc     add_s1l2m
add_s1l2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        lea rsi, [rdx + 8]
        movsx rdx, eax
        add rdi, 8
        cmp rdx, 0
        
        jns tmp_2
        neg rdx
        call rawSubLS
        sub rdi, 8
        sub rsi, 8
        ret
tmp_2:
        call rawAddLS
        sub rdi, 8
        sub rsi, 8
        ret


add_s1l2m:
        bt     rax, 62          ; check if montgomery first
        jc     add_s1ml2m
add_s1nl2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toMontgomery
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawAddLL
        sub rdi, 8
        sub rsi, 8
        ret


add_s1ml2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawAddLL
        sub rdi, 8
        sub rsi, 8
        ret


;;;;
add_l1l2:
        bt     rax, 62          ; check if montgomery first
        jc     add_l1ml2
add_l1nl2:
        bt     rcx, 62          ; check if montgomery second
        jc     add_l1nl2m
add_l1nl2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawAddLL
        sub rdi, 8
        sub rsi, 8
        ret


add_l1nl2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toMontgomery
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawAddLL
        sub rdi, 8
        sub rsi, 8
        ret


add_l1ml2:
        bt     rcx, 62          ; check if montgomery seconf
        jc     add_l1ml2m
add_l1ml2n:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rdx
        call Fr_toMontgomery
        mov  rdx, rdi
        pop  rdi

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawAddLL
        sub rdi, 8
        sub rsi, 8
        ret


add_l1ml2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawAddLL
        sub rdi, 8
        sub rsi, 8
        ret




;;;;;;;;;;;;;;;;;;;;;;
; rawAddLL
;;;;;;;;;;;;;;;;;;;;;;
; Adds two elements of type long
; Params:
;   rsi <= Pointer to the long data of element 1
;   rdx <= Pointer to the long data of element 2
;   rdi <= Pointer to the long data of result
; Modified Registers:
;    rax
;;;;;;;;;;;;;;;;;;;;;;
rawAddLL:
Fr_rawAdd:
        ; Add component by component with carry

        mov rax, [rsi + 0]
        add rax, [rdx + 0]
        mov [rdi + 0], rax

        mov rax, [rsi + 8]
        adc rax, [rdx + 8]
        mov [rdi + 8], rax

        mov rax, [rsi + 16]
        adc rax, [rdx + 16]
        mov [rdi + 16], rax

        mov rax, [rsi + 24]
        adc rax, [rdx + 24]
        mov [rdi + 24], rax

        jc rawAddLL_sq   ; if overflow, substract q

        ; Compare with q


        cmp rax, [q + 24]
        jc rawAddLL_done        ; q is bigget so done.
        jnz rawAddLL_sq         ; q is lower


        mov rax, [rdi + 16]

        cmp rax, [q + 16]
        jc rawAddLL_done        ; q is bigget so done.
        jnz rawAddLL_sq         ; q is lower


        mov rax, [rdi + 8]

        cmp rax, [q + 8]
        jc rawAddLL_done        ; q is bigget so done.
        jnz rawAddLL_sq         ; q is lower


        mov rax, [rdi + 0]

        cmp rax, [q + 0]
        jc rawAddLL_done        ; q is bigget so done.
        jnz rawAddLL_sq         ; q is lower

        ; If equal substract q
rawAddLL_sq:

        mov rax, [q + 0]
        sub [rdi + 0], rax

        mov rax, [q + 8]
        sbb [rdi + 8], rax

        mov rax, [q + 16]
        sbb [rdi + 16], rax

        mov rax, [q + 24]
        sbb [rdi + 24], rax

rawAddLL_done:
        ret


;;;;;;;;;;;;;;;;;;;;;;
; rawAddLS
;;;;;;;;;;;;;;;;;;;;;;
; Adds two elements of type long
; Params:
;   rdi <= Pointer to the long data of result
;   rsi <= Pointer to the long data of element 1
;   rdx <= Value to be added
;;;;;;;;;;;;;;;;;;;;;;
rawAddLS:
        ; Add component by component with carry

        add rdx, [rsi]
        mov [rdi] ,rdx

        mov rdx, 0
        adc rdx, [rsi + 8]
        mov [rdi + 8], rdx

        mov rdx, 0
        adc rdx, [rsi + 16]
        mov [rdi + 16], rdx

        mov rdx, 0
        adc rdx, [rsi + 24]
        mov [rdi + 24], rdx

        jc rawAddLS_sq   ; if overflow, substract q

        ; Compare with q

        mov rax, [rdi + 24]
        cmp rax, [q + 24]
        jc rawAddLS_done        ; q is bigget so done.
        jnz rawAddLS_sq         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 16]
        jc rawAddLS_done        ; q is bigget so done.
        jnz rawAddLS_sq         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 8]
        jc rawAddLS_done        ; q is bigget so done.
        jnz rawAddLS_sq         ; q is lower

        mov rax, [rdi + 0]
        cmp rax, [q + 0]
        jc rawAddLS_done        ; q is bigget so done.
        jnz rawAddLS_sq         ; q is lower

        ; If equal substract q
rawAddLS_sq:

        mov rax, [q + 0]
        sub [rdi + 0], rax

        mov rax, [q + 8]
        sbb [rdi + 8], rax

        mov rax, [q + 16]
        sbb [rdi + 16], rax

        mov rax, [q + 24]
        sbb [rdi + 24], rax

rawAddLS_done:
        ret















;;;;;;;;;;;;;;;;;;;;;;
; sub
;;;;;;;;;;;;;;;;;;;;;;
; Substracts two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result
; Modified Registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_sub:
        mov    rax, [rsi]
        mov    rcx, [rdx]
        bt     rax, 63          ; Check if is long first operand
        jc     sub_l1
        bt     rcx, 63          ; Check if is long second operand
        jc     sub_s1l2

sub_s1s2:                       ; Both operands are short

        xor    rdx, rdx
        mov    edx, eax
        sub    edx, ecx
        jo     sub_manageOverflow   ; rsi already is the 64bits result

        mov    [rdi], rdx       ; not necessary to adjust so just save and return
        ret

sub_manageOverflow:                 ; Do the operation in 64 bits
        push   rsi
        movsx  rsi, eax
        movsx  rdx, ecx
        sub    rsi, rdx
        call   rawCopyS2L
        pop    rsi
        ret

sub_l1:
        bt     rcx, 63          ; Check if is short second operand
        jc     sub_l1l2

;;;;;;;;
sub_l1s2:
        bt     rax, 62          ; check if montgomery first
        jc     sub_l1ms2
sub_l1ns2:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        add rsi, 8
        movsx rdx, ecx
        add rdi, 8
        cmp rdx, 0
        
        jns tmp_3
        neg rdx
        call rawAddLS
        sub rdi, 8
        sub rsi, 8
        ret
tmp_3:
        call rawSubLS
        sub rdi, 8
        sub rsi, 8
        ret


sub_l1ms2:
        bt     rcx, 62          ; check if montgomery second
        jc     sub_l1ms2m
sub_l1ms2n:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rdx
        call Fr_toMontgomery
        mov  rdx, rdi
        pop  rdi

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawSubLL
        sub rdi, 8
        sub rsi, 8
        ret


sub_l1ms2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawSubLL
        sub rdi, 8
        sub rsi, 8
        ret



;;;;;;;;
sub_s1l2:
        bt     rcx, 62          ; check if montgomery first
        jc     sub_s1l2m
sub_s1l2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        cmp eax, 0
        
        js tmp_4

        ; First Operand is positive
        push rsi
        add rdi, 8
        movsx rsi, eax
        add rdx, 8
        call rawSubSL
        sub rdi, 8
        pop rsi
        ret

tmp_4:   ; First operand is negative
        push rsi
        lea rsi, [rdx + 8]
        movsx rdx, eax
        add rdi, 8
        neg rdx
        call rawNegLS
        sub rdi, 8
        pop rsi
        ret


sub_s1l2m:
        bt     rax, 62          ; check if montgomery second
        jc     sub_s1ml2m
sub_s1nl2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toMontgomery
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawSubLL
        sub rdi, 8
        sub rsi, 8
        ret


sub_s1ml2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawSubLL
        sub rdi, 8
        sub rsi, 8
        ret


;;;;
sub_l1l2:
        bt     rax, 62          ; check if montgomery first
        jc     sub_l1ml2
sub_l1nl2:
        bt     rcx, 62          ; check if montgomery second
        jc     sub_l1nl2m
sub_l1nl2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawSubLL
        sub rdi, 8
        sub rsi, 8
        ret


sub_l1nl2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toMontgomery
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawSubLL
        sub rdi, 8
        sub rsi, 8
        ret


sub_l1ml2:
        bt     rcx, 62          ; check if montgomery seconf
        jc     sub_l1ml2m
sub_l1ml2n:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rdx
        call Fr_toMontgomery
        mov  rdx, rdi
        pop  rdi

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawSubLL
        sub rdi, 8
        sub rsi, 8
        ret


sub_l1ml2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call rawSubLL
        sub rdi, 8
        sub rsi, 8
        ret



;;;;;;;;;;;;;;;;;;;;;;
; rawSubLS
;;;;;;;;;;;;;;;;;;;;;;
; Substracts a short element from the long element
; Params:
;   rdi <= Pointer to the long data of result
;   rsi <= Pointer to the long data of element 1 where will be substracted
;   rdx <= Value to be substracted
;   [rdi] = [rsi] - rdx
; Modified Registers:
;    rax
;;;;;;;;;;;;;;;;;;;;;;
rawSubLS:
        ; Substract first digit

        mov rax, [rsi]
        sub rax, rdx
        mov [rdi] ,rax
        mov rdx, 0

        mov rax, [rsi + 8]
        sbb rax, rdx
        mov [rdi + 8], rax

        mov rax, [rsi + 16]
        sbb rax, rdx
        mov [rdi + 16], rax

        mov rax, [rsi + 24]
        sbb rax, rdx
        mov [rdi + 24], rax

        jnc rawSubLS_done   ; if overflow, add q

        ; Add q
rawSubLS_aq:

        mov rax, [q + 0]
        add [rdi + 0], rax

        mov rax, [q + 8]
        adc [rdi + 8], rax

        mov rax, [q + 16]
        adc [rdi + 16], rax

        mov rax, [q + 24]
        adc [rdi + 24], rax

rawSubLS_done:
        ret


;;;;;;;;;;;;;;;;;;;;;;
; rawSubSL
;;;;;;;;;;;;;;;;;;;;;;
; Substracts a long element from a short element
; Params:
;   rdi <= Pointer to the long data of result
;   rsi <= Value from where will bo substracted
;   rdx <= Pointer to long of the value to be substracted
;
;   [rdi] = rsi - [rdx]
; Modified Registers:
;    rax
;;;;;;;;;;;;;;;;;;;;;;
rawSubSL:
        ; Substract first digit
        sub rsi, [rdx]
        mov [rdi] ,rsi


        mov rax, 0
        sbb rax, [rdx + 8]
        mov [rdi + 8], rax

        mov rax, 0
        sbb rax, [rdx + 16]
        mov [rdi + 16], rax

        mov rax, 0
        sbb rax, [rdx + 24]
        mov [rdi + 24], rax

        jnc rawSubSL_done   ; if overflow, add q

        ; Add q
rawSubSL_aq:

        mov rax, [q + 0]
        add [rdi + 0], rax

        mov rax, [q + 8]
        adc [rdi + 8], rax

        mov rax, [q + 16]
        adc [rdi + 16], rax

        mov rax, [q + 24]
        adc [rdi + 24], rax

rawSubSL_done:
        ret

;;;;;;;;;;;;;;;;;;;;;;
; rawSubLL
;;;;;;;;;;;;;;;;;;;;;;
; Substracts a long element from a short element
; Params:
;   rdi <= Pointer to the long data of result
;   rsi <= Pointer to long from where substracted
;   rdx <= Pointer to long of the value to be substracted
;
;   [rdi] = [rsi] - [rdx]
; Modified Registers:
;    rax
;;;;;;;;;;;;;;;;;;;;;;
rawSubLL:
Fr_rawSub:
        ; Substract first digit

        mov rax, [rsi + 0]
        sub rax, [rdx + 0]
        mov [rdi + 0], rax

        mov rax, [rsi + 8]
        sbb rax, [rdx + 8]
        mov [rdi + 8], rax

        mov rax, [rsi + 16]
        sbb rax, [rdx + 16]
        mov [rdi + 16], rax

        mov rax, [rsi + 24]
        sbb rax, [rdx + 24]
        mov [rdi + 24], rax

        jnc rawSubLL_done   ; if overflow, add q

        ; Add q
rawSubLL_aq:

        mov rax, [q + 0]
        add [rdi + 0], rax

        mov rax, [q + 8]
        adc [rdi + 8], rax

        mov rax, [q + 16]
        adc [rdi + 16], rax

        mov rax, [q + 24]
        adc [rdi + 24], rax

rawSubLL_done:
        ret

;;;;;;;;;;;;;;;;;;;;;;
; rawNegLS
;;;;;;;;;;;;;;;;;;;;;;
; Substracts a long element and a short element form 0
; Params:
;   rdi <= Pointer to the long data of result
;   rsi <= Pointer to long from where substracted
;   rdx <= short value to be substracted too
;
;   [rdi] = -[rsi] - rdx
; Modified Registers:
;    rax
;;;;;;;;;;;;;;;;;;;;;;
rawNegLS:
        mov rax, [q]
        sub rax, rdx
        mov [rdi], rax

        mov rax, [q + 8 ]
        sbb rax, 0
        mov [rdi + 8], rax

        mov rax, [q + 16 ]
        sbb rax, 0
        mov [rdi + 16], rax

        mov rax, [q + 24 ]
        sbb rax, 0
        mov [rdi + 24], rax

        setc dl


        mov rax, [rdi + 0 ]
        sub rax, [rsi + 0]
        mov [rdi + 0], rax

        mov rax, [rdi + 8 ]
        sbb rax, [rsi + 8]
        mov [rdi + 8], rax

        mov rax, [rdi + 16 ]
        sbb rax, [rsi + 16]
        mov [rdi + 16], rax

        mov rax, [rdi + 24 ]
        sbb rax, [rsi + 24]
        mov [rdi + 24], rax


        setc dh
        or dl, dh
        jz rawNegSL_done

        ; it is a negative value, so add q

        mov rax, [q + 0]
        add [rdi + 0], rax

        mov rax, [q + 8]
        adc [rdi + 8], rax

        mov rax, [q + 16]
        adc [rdi + 16], rax

        mov rax, [q + 24]
        adc [rdi + 24], rax


rawNegSL_done:
        ret







;;;;;;;;;;;;;;;;;;;;;;
; neg
;;;;;;;;;;;;;;;;;;;;;;
; Adds two elements of any kind
; Params:
;   rsi <= Pointer to element to be negated
;   rdi <= Pointer to result
;   [rdi] = -[rsi]
;;;;;;;;;;;;;;;;;;;;;;
Fr_neg:
        mov    rax, [rsi]
        bt     rax, 63          ; Check if is short first operand
        jc     neg_l

neg_s:                          ; Operand is short

        neg    eax
        jo     neg_manageOverflow   ; Check if overflow. (0x80000000 is the only case)

        mov    [rdi], rax           ; not necessary to adjust so just save and return
        ret

neg_manageOverflow:                 ; Do the operation in 64 bits
        push   rsi
        movsx  rsi, eax
        neg    rsi
        call   rawCopyS2L
        pop    rsi
        ret



neg_l:
        mov [rdi], rax          ; Copy the type

        add rdi, 8
        add rsi, 8
        call rawNegL
        sub rdi, 8
        sub rsi, 8
        ret



;;;;;;;;;;;;;;;;;;;;;;
; rawNeg
;;;;;;;;;;;;;;;;;;;;;;
; Negates a value
; Params:
;   rdi <= Pointer to the long data of result
;   rsi <= Pointer to the long data of element 1
;
;   [rdi] = - [rsi]
;;;;;;;;;;;;;;;;;;;;;;
rawNegL:
Fr_rawNeg:
        ; Compare is zero

        xor rax, rax

        cmp [rsi + 0], rax
        jnz doNegate

        cmp [rsi + 8], rax
        jnz doNegate

        cmp [rsi + 16], rax
        jnz doNegate

        cmp [rsi + 24], rax
        jnz doNegate

        ; it's zero so just set to zero

        mov [rdi + 0], rax

        mov [rdi + 8], rax

        mov [rdi + 16], rax

        mov [rdi + 24], rax

        ret
doNegate:

        mov rax, [q + 0]
        sub rax, [rsi + 0]
        mov [rdi + 0], rax

        mov rax, [q + 8]
        sbb rax, [rsi + 8]
        mov [rdi + 8], rax

        mov rax, [q + 16]
        sbb rax, [rsi + 16]
        mov [rdi + 16], rax

        mov rax, [q + 24]
        sbb rax, [rsi + 24]
        mov [rdi + 24], rax

        ret



















;;;;;;;;;;;;;;;;;;;;;;
; square
;;;;;;;;;;;;;;;;;;;;;;
; Squares a field element
; Params:
;   rsi <= Pointer to element 1
;   rdi <= Pointer to result
;   [rdi] = [rsi] * [rsi]
; Modified Registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_square:
        mov    r8, [rsi]
        bt     r8, 63          ; Check if is short first operand
        jc     square_l1

square_s1:                       ; Both operands are short

        xor    rax, rax
        mov    eax, r8d
        imul   eax
        jo     square_manageOverflow   ; rsi already is the 64bits result

        mov    [rdi], rax       ; not necessary to adjust so just save and return

square_manageOverflow:                 ; Do the operation in 64 bits
        push   rsi
        movsx  rax, r8d
        imul   rax
        mov    rsi, rax
        call   rawCopyS2L
        pop    rsi

        ret

square_l1:
        bt     r8, 62          ; check if montgomery first
        jc     square_l1m
square_l1n:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        call Fr_rawMSquare
        sub rdi, 8
        sub rsi, 8


        push rsi
        add rdi, 8
        mov rsi, rdi
        lea rdx, [R3]
        call Fr_rawMMul
        sub rdi, 8
        pop rsi

        ret

square_l1m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        call Fr_rawMSquare
        sub rdi, 8
        sub rsi, 8

        ret



;;;;;;;;;;;;;;;;;;;;;;
; mul
;;;;;;;;;;;;;;;;;;;;;;
; Multiplies two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result
;   [rdi] = [rsi] * [rdi]
; Modified Registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_mul:
        mov    r8, [rsi]
        mov    r9, [rdx]
        bt     r8, 63          ; Check if is short first operand
        jc     mul_l1
        bt     r9, 63          ; Check if is short second operand
        jc     mul_s1l2

mul_s1s2:                       ; Both operands are short

        xor    rax, rax
        mov    eax, r8d
        imul   r9d
        jo     mul_manageOverflow   ; rsi already is the 64bits result

        mov    [rdi], rax       ; not necessary to adjust so just save and return

mul_manageOverflow:                 ; Do the operation in 64 bits
        push   rsi
        movsx  rax, r8d
        movsx  rcx, r9d
        imul   rcx
        mov    rsi, rax
        call   rawCopyS2L
        pop    rsi

        ret

mul_l1:
        bt     r9, 63          ; Check if is short second operand
        jc     mul_l1l2

;;;;;;;;
mul_l1s2:
        bt     r8, 62          ; check if montgomery first
        jc     mul_l1ms2
mul_l1ns2:
        bt     r9, 62          ; check if montgomery first
        jc     mul_l1ns2m
mul_l1ns2n:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        push rsi
        add rsi, 8
        movsx rdx, r9d
        add rdi, 8
        cmp rdx, 0
        
        jns tmp_5
        neg rdx
        call Fr_rawMMul1
        mov rsi, rdi
        call rawNegL
        sub rdi, 8
        pop rsi
        
        jmp tmp_6
tmp_5:
        call Fr_rawMMul1
        sub rdi, 8
        pop rsi
tmp_6:



        push rsi
        add rdi, 8
        mov rsi, rdi
        lea rdx, [R3]
        call Fr_rawMMul
        sub rdi, 8
        pop rsi

        ret


mul_l1ns2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call Fr_rawMMul
        sub rdi, 8
        sub rsi, 8

        ret


mul_l1ms2:
        bt     r9, 62          ; check if montgomery second
        jc     mul_l1ms2m
mul_l1ms2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        push rsi
        add rsi, 8
        movsx rdx, r9d
        add rdi, 8
        cmp rdx, 0
        
        jns tmp_7
        neg rdx
        call Fr_rawMMul1
        mov rsi, rdi
        call rawNegL
        sub rdi, 8
        pop rsi
        
        jmp tmp_8
tmp_7:
        call Fr_rawMMul1
        sub rdi, 8
        pop rsi
tmp_8:


        ret

mul_l1ms2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call Fr_rawMMul
        sub rdi, 8
        sub rsi, 8

        ret


;;;;;;;;
mul_s1l2:
        bt     r8, 62          ; check if montgomery first
        jc     mul_s1ml2
mul_s1nl2:
        bt     r9, 62          ; check if montgomery first
        jc     mul_s1nl2m
mul_s1nl2n:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        push rsi
        lea rsi, [rdx + 8]
        movsx rdx, r8d
        add rdi, 8
        cmp rdx, 0
        
        jns tmp_9
        neg rdx
        call Fr_rawMMul1
        mov rsi, rdi
        call rawNegL
        sub rdi, 8
        pop rsi
        
        jmp tmp_10
tmp_9:
        call Fr_rawMMul1
        sub rdi, 8
        pop rsi
tmp_10:



        push rsi
        add rdi, 8
        mov rsi, rdi
        lea rdx, [R3]
        call Fr_rawMMul
        sub rdi, 8
        pop rsi

        ret

mul_s1nl2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        push rsi
        lea rsi, [rdx + 8]
        movsx rdx, r8d
        add rdi, 8
        cmp rdx, 0
        
        jns tmp_11
        neg rdx
        call Fr_rawMMul1
        mov rsi, rdi
        call rawNegL
        sub rdi, 8
        pop rsi
        
        jmp tmp_12
tmp_11:
        call Fr_rawMMul1
        sub rdi, 8
        pop rsi
tmp_12:


        ret

mul_s1ml2:
        bt     r9, 62          ; check if montgomery first
        jc     mul_s1ml2m
mul_s1ml2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call Fr_rawMMul
        sub rdi, 8
        sub rsi, 8

        ret

mul_s1ml2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call Fr_rawMMul
        sub rdi, 8
        sub rsi, 8

        ret

;;;;
mul_l1l2:
        bt     r8, 62          ; check if montgomery first
        jc     mul_l1ml2
mul_l1nl2:
        bt     r9, 62          ; check if montgomery second
        jc     mul_l1nl2m
mul_l1nl2n:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call Fr_rawMMul
        sub rdi, 8
        sub rsi, 8


        push rsi
        add rdi, 8
        mov rsi, rdi
        lea rdx, [R3]
        call Fr_rawMMul
        sub rdi, 8
        pop rsi

        ret

mul_l1nl2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call Fr_rawMMul
        sub rdi, 8
        sub rsi, 8

        ret

mul_l1ml2:
        bt     r9, 62          ; check if montgomery seconf
        jc     mul_l1ml2m
mul_l1ml2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call Fr_rawMMul
        sub rdi, 8
        sub rsi, 8

        ret

mul_l1ml2m:
        mov r11b, 0xC0
        shl r11d, 24
        mov [rdi+4], r11d

        add rdi, 8
        add rsi, 8
        add rdx, 8
        call Fr_rawMMul
        sub rdi, 8
        sub rsi, 8

        ret


















;;;;;;;;;;;;;;;;;;;;;;
; band
;;;;;;;;;;;;;;;;;;;;;;
; Adds two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result
; Modified Registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_band:
        mov    r8, [rsi]
        mov    r9, [rdx]
        bt     r8, 63          ; Check if is short first operand
        jc     and_l1
        bt     r9, 63          ; Check if is short second operand
        jc     and_s1l2

and_s1s2:

        cmp    r8d, 0
        
        js     tmp_13

        cmp    r9d, 0
        js     tmp_13
        xor    rdx, rdx         ; both ops are positive so do the op and return
        mov    edx, r8d
        and    edx, r9d
        mov    [rdi], rdx       ; not necessary to adjust so just save and return
        ret

tmp_13:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi


        mov rax, [rsi + 8]
        and rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_15        ; q is bigget so done.
        jnz tmp_14         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_15        ; q is bigget so done.
        jnz tmp_14         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_15        ; q is bigget so done.
        jnz tmp_14         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_15        ; q is bigget so done.
        jnz tmp_14         ; q is lower

        ; If equal substract q
tmp_14:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_15:

        ret






and_l1:
        bt     r9, 63          ; Check if is short second operand
        jc     and_l1l2


and_l1s2:
        bt     r8, 62          ; check if montgomery first
        jc     and_l1ms2
and_l1ns2:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        cmp    r9d, 0
        
        js     tmp_16
        movsx  rax, r9d
        and rax, [rsi +8]
        mov    [rdi+8], rax

        xor    rax, rax
        and rax, [rsi + 16];

        mov    [rdi + 16 ], rax;

        xor    rax, rax
        and rax, [rsi + 24];

        mov    [rdi + 24 ], rax;

        xor    rax, rax
        and rax, [rsi + 32];

        and    rax, [lboMask] ;

        mov    [rdi + 32 ], rax;


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_18        ; q is bigget so done.
        jnz tmp_17         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_18        ; q is bigget so done.
        jnz tmp_17         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_18        ; q is bigget so done.
        jnz tmp_17         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_18        ; q is bigget so done.
        jnz tmp_17         ; q is lower

        ; If equal substract q
tmp_17:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_18:

        ret

tmp_16:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        and rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_20        ; q is bigget so done.
        jnz tmp_19         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_20        ; q is bigget so done.
        jnz tmp_19         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_20        ; q is bigget so done.
        jnz tmp_19         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_20        ; q is bigget so done.
        jnz tmp_19         ; q is lower

        ; If equal substract q
tmp_19:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_20:

        ret




and_l1ms2:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push   r9              ; r9 is used in montgomery so we need to save it
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        pop    r9

        cmp    r9d, 0
        
        js     tmp_21
        movsx  rax, r9d
        and rax, [rsi +8]
        mov    [rdi+8], rax

        xor    rax, rax
        and rax, [rsi + 16];

        mov    [rdi + 16 ], rax;

        xor    rax, rax
        and rax, [rsi + 24];

        mov    [rdi + 24 ], rax;

        xor    rax, rax
        and rax, [rsi + 32];

        and    rax, [lboMask] ;

        mov    [rdi + 32 ], rax;


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_23        ; q is bigget so done.
        jnz tmp_22         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_23        ; q is bigget so done.
        jnz tmp_22         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_23        ; q is bigget so done.
        jnz tmp_22         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_23        ; q is bigget so done.
        jnz tmp_22         ; q is lower

        ; If equal substract q
tmp_22:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_23:

        ret

tmp_21:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        and rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_25        ; q is bigget so done.
        jnz tmp_24         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_25        ; q is bigget so done.
        jnz tmp_24         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_25        ; q is bigget so done.
        jnz tmp_24         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_25        ; q is bigget so done.
        jnz tmp_24         ; q is lower

        ; If equal substract q
tmp_24:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_25:

        ret





and_s1l2:
        bt     r9, 62          ; check if montgomery first
        jc     and_s1l2m
and_s1l2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        cmp    r8d, 0
        
        js     tmp_26
        movsx  rax, r8d
        and rax, [rdx +8]
        mov    [rdi+8], rax

        xor    rax, rax
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        xor    rax, rax
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        xor    rax, rax
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_28        ; q is bigget so done.
        jnz tmp_27         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_28        ; q is bigget so done.
        jnz tmp_27         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_28        ; q is bigget so done.
        jnz tmp_27         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_28        ; q is bigget so done.
        jnz tmp_27         ; q is lower

        ; If equal substract q
tmp_27:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_28:

        ret

tmp_26:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        and rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_30        ; q is bigget so done.
        jnz tmp_29         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_30        ; q is bigget so done.
        jnz tmp_29         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_30        ; q is bigget so done.
        jnz tmp_29         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_30        ; q is bigget so done.
        jnz tmp_29         ; q is lower

        ; If equal substract q
tmp_29:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_30:

        ret




and_s1l2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push   r8             ; r8 is used in montgomery so we need to save it
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi
        pop    r8

        cmp    r8d, 0
        
        js     tmp_31
        movsx  rax, r8d
        and rax, [rdx +8]
        mov    [rdi+8], rax

        xor    rax, rax
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        xor    rax, rax
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        xor    rax, rax
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_33        ; q is bigget so done.
        jnz tmp_32         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_33        ; q is bigget so done.
        jnz tmp_32         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_33        ; q is bigget so done.
        jnz tmp_32         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_33        ; q is bigget so done.
        jnz tmp_32         ; q is lower

        ; If equal substract q
tmp_32:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_33:

        ret

tmp_31:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        and rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_35        ; q is bigget so done.
        jnz tmp_34         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_35        ; q is bigget so done.
        jnz tmp_34         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_35        ; q is bigget so done.
        jnz tmp_34         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_35        ; q is bigget so done.
        jnz tmp_34         ; q is lower

        ; If equal substract q
tmp_34:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_35:

        ret





and_l1l2:
        bt     r8, 62          ; check if montgomery first
        jc     and_l1ml2
        bt     r9, 62          ; check if montgomery first
        jc     and_l1nl2m
and_l1nl2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        and rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_37        ; q is bigget so done.
        jnz tmp_36         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_37        ; q is bigget so done.
        jnz tmp_36         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_37        ; q is bigget so done.
        jnz tmp_36         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_37        ; q is bigget so done.
        jnz tmp_36         ; q is lower

        ; If equal substract q
tmp_36:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_37:

        ret


and_l1nl2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi


        mov rax, [rsi + 8]
        and rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_39        ; q is bigget so done.
        jnz tmp_38         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_39        ; q is bigget so done.
        jnz tmp_38         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_39        ; q is bigget so done.
        jnz tmp_38         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_39        ; q is bigget so done.
        jnz tmp_38         ; q is lower

        ; If equal substract q
tmp_38:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_39:

        ret


and_l1ml2:
        bt     r9, 62          ; check if montgomery first
        jc     and_l1ml2m
and_l1ml2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi


        mov rax, [rsi + 8]
        and rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_41        ; q is bigget so done.
        jnz tmp_40         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_41        ; q is bigget so done.
        jnz tmp_40         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_41        ; q is bigget so done.
        jnz tmp_40         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_41        ; q is bigget so done.
        jnz tmp_40         ; q is lower

        ; If equal substract q
tmp_40:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_41:

        ret


and_l1ml2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi


        mov rax, [rsi + 8]
        and rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        and rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        and rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        and rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_43        ; q is bigget so done.
        jnz tmp_42         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_43        ; q is bigget so done.
        jnz tmp_42         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_43        ; q is bigget so done.
        jnz tmp_42         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_43        ; q is bigget so done.
        jnz tmp_42         ; q is lower

        ; If equal substract q
tmp_42:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_43:

        ret



;;;;;;;;;;;;;;;;;;;;;;
; bor
;;;;;;;;;;;;;;;;;;;;;;
; Adds two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result
; Modified Registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_bor:
        mov    r8, [rsi]
        mov    r9, [rdx]
        bt     r8, 63          ; Check if is short first operand
        jc     or_l1
        bt     r9, 63          ; Check if is short second operand
        jc     or_s1l2

or_s1s2:

        cmp    r8d, 0
        
        js     tmp_44

        cmp    r9d, 0
        js     tmp_44
        xor    rdx, rdx         ; both ops are positive so do the op and return
        mov    edx, r8d
        or    edx, r9d
        mov    [rdi], rdx       ; not necessary to adjust so just save and return
        ret

tmp_44:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi


        mov rax, [rsi + 8]
        or rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_46        ; q is bigget so done.
        jnz tmp_45         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_46        ; q is bigget so done.
        jnz tmp_45         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_46        ; q is bigget so done.
        jnz tmp_45         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_46        ; q is bigget so done.
        jnz tmp_45         ; q is lower

        ; If equal substract q
tmp_45:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_46:

        ret






or_l1:
        bt     r9, 63          ; Check if is short second operand
        jc     or_l1l2


or_l1s2:
        bt     r8, 62          ; check if montgomery first
        jc     or_l1ms2
or_l1ns2:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        cmp    r9d, 0
        
        js     tmp_47
        movsx  rax, r9d
        or rax, [rsi +8]
        mov    [rdi+8], rax

        xor    rax, rax
        or rax, [rsi + 16];

        mov    [rdi + 16 ], rax;

        xor    rax, rax
        or rax, [rsi + 24];

        mov    [rdi + 24 ], rax;

        xor    rax, rax
        or rax, [rsi + 32];

        and    rax, [lboMask] ;

        mov    [rdi + 32 ], rax;


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_49        ; q is bigget so done.
        jnz tmp_48         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_49        ; q is bigget so done.
        jnz tmp_48         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_49        ; q is bigget so done.
        jnz tmp_48         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_49        ; q is bigget so done.
        jnz tmp_48         ; q is lower

        ; If equal substract q
tmp_48:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_49:

        ret

tmp_47:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        or rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_51        ; q is bigget so done.
        jnz tmp_50         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_51        ; q is bigget so done.
        jnz tmp_50         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_51        ; q is bigget so done.
        jnz tmp_50         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_51        ; q is bigget so done.
        jnz tmp_50         ; q is lower

        ; If equal substract q
tmp_50:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_51:

        ret




or_l1ms2:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push   r9              ; r9 is used in montgomery so we need to save it
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        pop    r9

        cmp    r9d, 0
        
        js     tmp_52
        movsx  rax, r9d
        or rax, [rsi +8]
        mov    [rdi+8], rax

        xor    rax, rax
        or rax, [rsi + 16];

        mov    [rdi + 16 ], rax;

        xor    rax, rax
        or rax, [rsi + 24];

        mov    [rdi + 24 ], rax;

        xor    rax, rax
        or rax, [rsi + 32];

        and    rax, [lboMask] ;

        mov    [rdi + 32 ], rax;


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_54        ; q is bigget so done.
        jnz tmp_53         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_54        ; q is bigget so done.
        jnz tmp_53         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_54        ; q is bigget so done.
        jnz tmp_53         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_54        ; q is bigget so done.
        jnz tmp_53         ; q is lower

        ; If equal substract q
tmp_53:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_54:

        ret

tmp_52:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        or rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_56        ; q is bigget so done.
        jnz tmp_55         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_56        ; q is bigget so done.
        jnz tmp_55         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_56        ; q is bigget so done.
        jnz tmp_55         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_56        ; q is bigget so done.
        jnz tmp_55         ; q is lower

        ; If equal substract q
tmp_55:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_56:

        ret





or_s1l2:
        bt     r9, 62          ; check if montgomery first
        jc     or_s1l2m
or_s1l2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        cmp    r8d, 0
        
        js     tmp_57
        movsx  rax, r8d
        or rax, [rdx +8]
        mov    [rdi+8], rax

        xor    rax, rax
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        xor    rax, rax
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        xor    rax, rax
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_59        ; q is bigget so done.
        jnz tmp_58         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_59        ; q is bigget so done.
        jnz tmp_58         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_59        ; q is bigget so done.
        jnz tmp_58         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_59        ; q is bigget so done.
        jnz tmp_58         ; q is lower

        ; If equal substract q
tmp_58:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_59:

        ret

tmp_57:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        or rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_61        ; q is bigget so done.
        jnz tmp_60         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_61        ; q is bigget so done.
        jnz tmp_60         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_61        ; q is bigget so done.
        jnz tmp_60         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_61        ; q is bigget so done.
        jnz tmp_60         ; q is lower

        ; If equal substract q
tmp_60:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_61:

        ret




or_s1l2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push   r8             ; r8 is used in montgomery so we need to save it
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi
        pop    r8

        cmp    r8d, 0
        
        js     tmp_62
        movsx  rax, r8d
        or rax, [rdx +8]
        mov    [rdi+8], rax

        xor    rax, rax
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        xor    rax, rax
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        xor    rax, rax
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_64        ; q is bigget so done.
        jnz tmp_63         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_64        ; q is bigget so done.
        jnz tmp_63         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_64        ; q is bigget so done.
        jnz tmp_63         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_64        ; q is bigget so done.
        jnz tmp_63         ; q is lower

        ; If equal substract q
tmp_63:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_64:

        ret

tmp_62:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        or rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_66        ; q is bigget so done.
        jnz tmp_65         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_66        ; q is bigget so done.
        jnz tmp_65         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_66        ; q is bigget so done.
        jnz tmp_65         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_66        ; q is bigget so done.
        jnz tmp_65         ; q is lower

        ; If equal substract q
tmp_65:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_66:

        ret





or_l1l2:
        bt     r8, 62          ; check if montgomery first
        jc     or_l1ml2
        bt     r9, 62          ; check if montgomery first
        jc     or_l1nl2m
or_l1nl2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        or rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_68        ; q is bigget so done.
        jnz tmp_67         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_68        ; q is bigget so done.
        jnz tmp_67         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_68        ; q is bigget so done.
        jnz tmp_67         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_68        ; q is bigget so done.
        jnz tmp_67         ; q is lower

        ; If equal substract q
tmp_67:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_68:

        ret


or_l1nl2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi


        mov rax, [rsi + 8]
        or rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_70        ; q is bigget so done.
        jnz tmp_69         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_70        ; q is bigget so done.
        jnz tmp_69         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_70        ; q is bigget so done.
        jnz tmp_69         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_70        ; q is bigget so done.
        jnz tmp_69         ; q is lower

        ; If equal substract q
tmp_69:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_70:

        ret


or_l1ml2:
        bt     r9, 62          ; check if montgomery first
        jc     or_l1ml2m
or_l1ml2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi


        mov rax, [rsi + 8]
        or rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_72        ; q is bigget so done.
        jnz tmp_71         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_72        ; q is bigget so done.
        jnz tmp_71         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_72        ; q is bigget so done.
        jnz tmp_71         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_72        ; q is bigget so done.
        jnz tmp_71         ; q is lower

        ; If equal substract q
tmp_71:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_72:

        ret


or_l1ml2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi


        mov rax, [rsi + 8]
        or rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        or rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        or rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        or rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_74        ; q is bigget so done.
        jnz tmp_73         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_74        ; q is bigget so done.
        jnz tmp_73         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_74        ; q is bigget so done.
        jnz tmp_73         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_74        ; q is bigget so done.
        jnz tmp_73         ; q is lower

        ; If equal substract q
tmp_73:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_74:

        ret



;;;;;;;;;;;;;;;;;;;;;;
; bxor
;;;;;;;;;;;;;;;;;;;;;;
; Adds two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result
; Modified Registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_bxor:
        mov    r8, [rsi]
        mov    r9, [rdx]
        bt     r8, 63          ; Check if is short first operand
        jc     xor_l1
        bt     r9, 63          ; Check if is short second operand
        jc     xor_s1l2

xor_s1s2:

        cmp    r8d, 0
        
        js     tmp_75

        cmp    r9d, 0
        js     tmp_75
        xor    rdx, rdx         ; both ops are positive so do the op and return
        mov    edx, r8d
        xor    edx, r9d
        mov    [rdi], rdx       ; not necessary to adjust so just save and return
        ret

tmp_75:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi


        mov rax, [rsi + 8]
        xor rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_77        ; q is bigget so done.
        jnz tmp_76         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_77        ; q is bigget so done.
        jnz tmp_76         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_77        ; q is bigget so done.
        jnz tmp_76         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_77        ; q is bigget so done.
        jnz tmp_76         ; q is lower

        ; If equal substract q
tmp_76:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_77:

        ret






xor_l1:
        bt     r9, 63          ; Check if is short second operand
        jc     xor_l1l2


xor_l1s2:
        bt     r8, 62          ; check if montgomery first
        jc     xor_l1ms2
xor_l1ns2:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        cmp    r9d, 0
        
        js     tmp_78
        movsx  rax, r9d
        xor rax, [rsi +8]
        mov    [rdi+8], rax

        xor    rax, rax
        xor rax, [rsi + 16];

        mov    [rdi + 16 ], rax;

        xor    rax, rax
        xor rax, [rsi + 24];

        mov    [rdi + 24 ], rax;

        xor    rax, rax
        xor rax, [rsi + 32];

        and    rax, [lboMask] ;

        mov    [rdi + 32 ], rax;


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_80        ; q is bigget so done.
        jnz tmp_79         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_80        ; q is bigget so done.
        jnz tmp_79         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_80        ; q is bigget so done.
        jnz tmp_79         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_80        ; q is bigget so done.
        jnz tmp_79         ; q is lower

        ; If equal substract q
tmp_79:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_80:

        ret

tmp_78:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        xor rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_82        ; q is bigget so done.
        jnz tmp_81         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_82        ; q is bigget so done.
        jnz tmp_81         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_82        ; q is bigget so done.
        jnz tmp_81         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_82        ; q is bigget so done.
        jnz tmp_81         ; q is lower

        ; If equal substract q
tmp_81:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_82:

        ret




xor_l1ms2:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push   r9              ; r9 is used in montgomery so we need to save it
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        pop    r9

        cmp    r9d, 0
        
        js     tmp_83
        movsx  rax, r9d
        xor rax, [rsi +8]
        mov    [rdi+8], rax

        xor    rax, rax
        xor rax, [rsi + 16];

        mov    [rdi + 16 ], rax;

        xor    rax, rax
        xor rax, [rsi + 24];

        mov    [rdi + 24 ], rax;

        xor    rax, rax
        xor rax, [rsi + 32];

        and    rax, [lboMask] ;

        mov    [rdi + 32 ], rax;


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_85        ; q is bigget so done.
        jnz tmp_84         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_85        ; q is bigget so done.
        jnz tmp_84         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_85        ; q is bigget so done.
        jnz tmp_84         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_85        ; q is bigget so done.
        jnz tmp_84         ; q is lower

        ; If equal substract q
tmp_84:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_85:

        ret

tmp_83:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        xor rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_87        ; q is bigget so done.
        jnz tmp_86         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_87        ; q is bigget so done.
        jnz tmp_86         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_87        ; q is bigget so done.
        jnz tmp_86         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_87        ; q is bigget so done.
        jnz tmp_86         ; q is lower

        ; If equal substract q
tmp_86:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_87:

        ret





xor_s1l2:
        bt     r9, 62          ; check if montgomery first
        jc     xor_s1l2m
xor_s1l2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        cmp    r8d, 0
        
        js     tmp_88
        movsx  rax, r8d
        xor rax, [rdx +8]
        mov    [rdi+8], rax

        xor    rax, rax
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        xor    rax, rax
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        xor    rax, rax
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_90        ; q is bigget so done.
        jnz tmp_89         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_90        ; q is bigget so done.
        jnz tmp_89         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_90        ; q is bigget so done.
        jnz tmp_89         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_90        ; q is bigget so done.
        jnz tmp_89         ; q is lower

        ; If equal substract q
tmp_89:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_90:

        ret

tmp_88:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        xor rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_92        ; q is bigget so done.
        jnz tmp_91         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_92        ; q is bigget so done.
        jnz tmp_91         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_92        ; q is bigget so done.
        jnz tmp_91         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_92        ; q is bigget so done.
        jnz tmp_91         ; q is lower

        ; If equal substract q
tmp_91:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_92:

        ret




xor_s1l2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push   r8             ; r8 is used in montgomery so we need to save it
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi
        pop    r8

        cmp    r8d, 0
        
        js     tmp_93
        movsx  rax, r8d
        xor rax, [rdx +8]
        mov    [rdi+8], rax

        xor    rax, rax
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        xor    rax, rax
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        xor    rax, rax
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_95        ; q is bigget so done.
        jnz tmp_94         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_95        ; q is bigget so done.
        jnz tmp_94         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_95        ; q is bigget so done.
        jnz tmp_94         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_95        ; q is bigget so done.
        jnz tmp_94         ; q is lower

        ; If equal substract q
tmp_94:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_95:

        ret

tmp_93:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        xor rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_97        ; q is bigget so done.
        jnz tmp_96         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_97        ; q is bigget so done.
        jnz tmp_96         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_97        ; q is bigget so done.
        jnz tmp_96         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_97        ; q is bigget so done.
        jnz tmp_96         ; q is lower

        ; If equal substract q
tmp_96:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_97:

        ret





xor_l1l2:
        bt     r8, 62          ; check if montgomery first
        jc     xor_l1ml2
        bt     r9, 62          ; check if montgomery first
        jc     xor_l1nl2m
xor_l1nl2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d


        mov rax, [rsi + 8]
        xor rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_99        ; q is bigget so done.
        jnz tmp_98         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_99        ; q is bigget so done.
        jnz tmp_98         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_99        ; q is bigget so done.
        jnz tmp_98         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_99        ; q is bigget so done.
        jnz tmp_98         ; q is lower

        ; If equal substract q
tmp_98:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_99:

        ret


xor_l1nl2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi


        mov rax, [rsi + 8]
        xor rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_101        ; q is bigget so done.
        jnz tmp_100         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_101        ; q is bigget so done.
        jnz tmp_100         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_101        ; q is bigget so done.
        jnz tmp_100         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_101        ; q is bigget so done.
        jnz tmp_100         ; q is lower

        ; If equal substract q
tmp_100:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_101:

        ret


xor_l1ml2:
        bt     r9, 62          ; check if montgomery first
        jc     xor_l1ml2m
xor_l1ml2n:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi


        mov rax, [rsi + 8]
        xor rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_103        ; q is bigget so done.
        jnz tmp_102         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_103        ; q is bigget so done.
        jnz tmp_102         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_103        ; q is bigget so done.
        jnz tmp_102         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_103        ; q is bigget so done.
        jnz tmp_102         ; q is lower

        ; If equal substract q
tmp_102:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_103:

        ret


xor_l1ml2m:
        mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi


        mov rax, [rsi + 8]
        xor rax, [rdx + 8]

        mov    [rdi + 8 ], rax

        mov rax, [rsi + 16]
        xor rax, [rdx + 16]

        mov    [rdi + 16 ], rax

        mov rax, [rsi + 24]
        xor rax, [rdx + 24]

        mov    [rdi + 24 ], rax

        mov rax, [rsi + 32]
        xor rax, [rdx + 32]

        and    rax, [lboMask]

        mov    [rdi + 32 ], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_105        ; q is bigget so done.
        jnz tmp_104         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_105        ; q is bigget so done.
        jnz tmp_104         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_105        ; q is bigget so done.
        jnz tmp_104         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_105        ; q is bigget so done.
        jnz tmp_104         ; q is lower

        ; If equal substract q
tmp_104:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_105:

        ret




;;;;;;;;;;;;;;;;;;;;;;
; bnot
;;;;;;;;;;;;;;;;;;;;;;
; Adds two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdi <= Pointer to result
; Modified Registers:
;    r8, r9, 10, r11, rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_bnot:
                mov r11b, 0x80
        shl r11d, 24
        mov [rdi+4], r11d

        mov     r8, [rsi]
        bt      r8, 63          ; Check if is long operand
        jc      bnot_l1
bnot_s:
                push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        jmp     bnot_l1n

bnot_l1:
        bt     r8, 62          ; check if montgomery first
        jnc    bnot_l1n

bnot_l1m:
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi

bnot_l1n:

        mov    rax, [rsi + 8]
        not    rax

        mov    [rdi + 8], rax

        mov    rax, [rsi + 16]
        not    rax

        mov    [rdi + 16], rax

        mov    rax, [rsi + 24]
        not    rax

        mov    [rdi + 24], rax

        mov    rax, [rsi + 32]
        not    rax

        and    rax, [lboMask]

        mov    [rdi + 32], rax


        
        

        ; Compare with q

        mov rax, [rdi + 32]
        cmp rax, [q + 24]
        jc tmp_107        ; q is bigget so done.
        jnz tmp_106         ; q is lower

        mov rax, [rdi + 24]
        cmp rax, [q + 16]
        jc tmp_107        ; q is bigget so done.
        jnz tmp_106         ; q is lower

        mov rax, [rdi + 16]
        cmp rax, [q + 8]
        jc tmp_107        ; q is bigget so done.
        jnz tmp_106         ; q is lower

        mov rax, [rdi + 8]
        cmp rax, [q + 0]
        jc tmp_107        ; q is bigget so done.
        jnz tmp_106         ; q is lower

        ; If equal substract q
tmp_106:

        mov rax, [q + 0]
        sub [rdi + 8], rax

        mov rax, [q + 8]
        sbb [rdi + 16], rax

        mov rax, [q + 16]
        sbb [rdi + 24], rax

        mov rax, [q + 24]
        sbb [rdi + 32], rax

tmp_107:

        ret






;;;;;;;;;;;;;;;;;;;;;;
; rgt - Raw Greater Than
;;;;;;;;;;;;;;;;;;;;;;
; returns in ax 1 id *rsi > *rdx
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rax <= Return 1 or 0
; Modified Registers:
;    r8, r9, rax
;;;;;;;;;;;;;;;;;;;;;;
Fr_rgt:
        mov    r8, [rsi]
        mov    r9, [rdx]
        bt     r8, 63          ; Check if is short first operand
        jc     rgt_l1
        bt     r9, 63          ; Check if is short second operand
        jc     rgt_s1l2

rgt_s1s2:                       ; Both operands are short
        cmp    r8d, r9d
        jg     rgt_ret1
        jmp    rgt_ret0


rgt_l1:
        bt     r9, 63          ; Check if is short second operand
        jc     rgt_l1l2

;;;;;;;;
rgt_l1s2:
        bt     r8, 62          ; check if montgomery first
        jc     rgt_l1ms2
rgt_l1ns2:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        jmp rgtL1L2

rgt_l1ms2:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        jmp rgtL1L2


;;;;;;;;
rgt_s1l2:
        bt     r9, 62          ; check if montgomery second
        jc     rgt_s1l2m
rgt_s1l2n:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        jmp rgtL1L2

rgt_s1l2m:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi
        jmp rgtL1L2

;;;;
rgt_l1l2:
        bt     r8, 62          ; check if montgomery first
        jc     rgt_l1ml2
rgt_l1nl2:
        bt     r9, 62          ; check if montgomery second
        jc     rgt_l1nl2m
rgt_l1nl2n:
        jmp rgtL1L2

rgt_l1nl2m:
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi
        jmp rgtL1L2

rgt_l1ml2:
        bt     r9, 62          ; check if montgomery second
        jc     rgt_l1ml2m
rgt_l1ml2n:
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        jmp rgtL1L2

rgt_l1ml2m:
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi
        jmp rgtL1L2


;;;;;;
; rgtL1L2
;;;;;;

rgtL1L2:


        mov     rax, [rsi + 32]
        cmp     [half + 24], rax     ; comare with (q-1)/2
        jc rgtl1l2_n1                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rgtl1l2_p1                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rsi + 24]
        cmp     [half + 16], rax     ; comare with (q-1)/2
        jc rgtl1l2_n1                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rgtl1l2_p1                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rsi + 16]
        cmp     [half + 8], rax     ; comare with (q-1)/2
        jc rgtl1l2_n1                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rgtl1l2_p1                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rsi + 8]
        cmp     [half + 0], rax     ; comare with (q-1)/2
        jc rgtl1l2_n1                           ; half<rax => e1-e2 is neg => e1 < e2

        jmp rgtl1l2_p1



rgtl1l2_p1:


        mov     rax, [rdx + 32]
        cmp     [half + 24], rax     ; comare with (q-1)/2
        jc rgt_ret1                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rgtRawL1L2                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 24]
        cmp     [half + 16], rax     ; comare with (q-1)/2
        jc rgt_ret1                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rgtRawL1L2                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 16]
        cmp     [half + 8], rax     ; comare with (q-1)/2
        jc rgt_ret1                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rgtRawL1L2                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 8]
        cmp     [half + 0], rax     ; comare with (q-1)/2
        jc rgt_ret1                           ; half<rax => e1-e2 is neg => e1 < e2

        jmp rgtRawL1L2




rgtl1l2_n1:


        mov     rax, [rdx + 32]
        cmp     [half + 24], rax     ; comare with (q-1)/2
        jc rgtRawL1L2                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rgt_ret0                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 24]
        cmp     [half + 16], rax     ; comare with (q-1)/2
        jc rgtRawL1L2                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rgt_ret0                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 16]
        cmp     [half + 8], rax     ; comare with (q-1)/2
        jc rgtRawL1L2                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rgt_ret0                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 8]
        cmp     [half + 0], rax     ; comare with (q-1)/2
        jc rgtRawL1L2                           ; half<rax => e1-e2 is neg => e1 < e2

        jmp rgt_ret0





rgtRawL1L2:

        mov     rax, [rsi + 32]
        cmp     [rdx + 32], rax     ; comare with (q-1)/2
        jc rgt_ret1                      ; rsi<rdx => 1st > 2nd

        jnz rgt_ret0


        mov     rax, [rsi + 24]
        cmp     [rdx + 24], rax     ; comare with (q-1)/2
        jc rgt_ret1                      ; rsi<rdx => 1st > 2nd

        jnz rgt_ret0


        mov     rax, [rsi + 16]
        cmp     [rdx + 16], rax     ; comare with (q-1)/2
        jc rgt_ret1                      ; rsi<rdx => 1st > 2nd

        jnz rgt_ret0


        mov     rax, [rsi + 8]
        cmp     [rdx + 8], rax     ; comare with (q-1)/2
        jc rgt_ret1                      ; rsi<rdx => 1st > 2nd



rgt_ret0:
        xor    rax, rax
        ret
rgt_ret1:
        mov    rax, 1
        ret



;;;;;;;;;;;;;;;;;;;;;;
; rlt - Raw Less Than
;;;;;;;;;;;;;;;;;;;;;;
; returns in ax 1 id *rsi > *rdx
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rax <= Return 1 or 0
; Modified Registers:
;    r8, r9, rax
;;;;;;;;;;;;;;;;;;;;;;
Fr_rlt:
        mov    r8, [rsi]
        mov    r9, [rdx]
        bt     r8, 63          ; Check if is short first operand
        jc     rlt_l1
        bt     r9, 63          ; Check if is short second operand
        jc     rlt_s1l2

rlt_s1s2:                       ; Both operands are short
        cmp    r8d, r9d
        jl     rlt_ret1
        jmp    rlt_ret0


rlt_l1:
        bt     r9, 63          ; Check if is short second operand
        jc     rlt_l1l2

;;;;;;;;
rlt_l1s2:
        bt     r8, 62          ; check if montgomery first
        jc     rlt_l1ms2
rlt_l1ns2:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        jmp rltL1L2

rlt_l1ms2:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        jmp rltL1L2


;;;;;;;;
rlt_s1l2:
        bt     r9, 62          ; check if montgomery second
        jc     rlt_s1l2m
rlt_s1l2n:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        jmp rltL1L2

rlt_s1l2m:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi
        jmp rltL1L2

;;;;
rlt_l1l2:
        bt     r8, 62          ; check if montgomery first
        jc     rlt_l1ml2
rlt_l1nl2:
        bt     r9, 62          ; check if montgomery second
        jc     rlt_l1nl2m
rlt_l1nl2n:
        jmp rltL1L2

rlt_l1nl2m:
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi
        jmp rltL1L2

rlt_l1ml2:
        bt     r9, 62          ; check if montgomery second
        jc     rlt_l1ml2m
rlt_l1ml2n:
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        jmp rltL1L2

rlt_l1ml2m:
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toNormal
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        push rdi
        mov  rdi, rdx
        call Fr_toNormal
        mov  rdx, rdi
        pop  rdi
        jmp rltL1L2


;;;;;;
; rltL1L2
;;;;;;

rltL1L2:


        mov     rax, [rsi + 32]
        cmp     [half + 24], rax     ; comare with (q-1)/2
        jc rltl1l2_n1                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rltl1l2_p1                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rsi + 24]
        cmp     [half + 16], rax     ; comare with (q-1)/2
        jc rltl1l2_n1                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rltl1l2_p1                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rsi + 16]
        cmp     [half + 8], rax     ; comare with (q-1)/2
        jc rltl1l2_n1                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rltl1l2_p1                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rsi + 8]
        cmp     [half + 0], rax     ; comare with (q-1)/2
        jc rltl1l2_n1                           ; half<rax => e1-e2 is neg => e1 < e2

        jmp rltl1l2_p1



rltl1l2_p1:


        mov     rax, [rdx + 32]
        cmp     [half + 24], rax     ; comare with (q-1)/2
        jc rlt_ret0                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rltRawL1L2                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 24]
        cmp     [half + 16], rax     ; comare with (q-1)/2
        jc rlt_ret0                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rltRawL1L2                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 16]
        cmp     [half + 8], rax     ; comare with (q-1)/2
        jc rlt_ret0                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rltRawL1L2                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 8]
        cmp     [half + 0], rax     ; comare with (q-1)/2
        jc rlt_ret0                           ; half<rax => e1-e2 is neg => e1 < e2

        jmp rltRawL1L2




rltl1l2_n1:


        mov     rax, [rdx + 32]
        cmp     [half + 24], rax     ; comare with (q-1)/2
        jc rltRawL1L2                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rlt_ret1                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 24]
        cmp     [half + 16], rax     ; comare with (q-1)/2
        jc rltRawL1L2                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rlt_ret1                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 16]
        cmp     [half + 8], rax     ; comare with (q-1)/2
        jc rltRawL1L2                           ; half<rax => e1-e2 is neg => e1 < e2

        jnz rlt_ret1                       ; half>rax => e1 -e2 is pos => e1 > e2


        mov     rax, [rdx + 8]
        cmp     [half + 0], rax     ; comare with (q-1)/2
        jc rltRawL1L2                           ; half<rax => e1-e2 is neg => e1 < e2

        jmp rlt_ret1





rltRawL1L2:

        mov     rax, [rsi + 32]
        cmp     [rdx + 32], rax     ; comare with (q-1)/2
        jc rlt_ret0                      ; rsi<rdx => 1st > 2nd
        jnz rlt_ret1

        mov     rax, [rsi + 24]
        cmp     [rdx + 24], rax     ; comare with (q-1)/2
        jc rlt_ret0                      ; rsi<rdx => 1st > 2nd
        jnz rlt_ret1

        mov     rax, [rsi + 16]
        cmp     [rdx + 16], rax     ; comare with (q-1)/2
        jc rlt_ret0                      ; rsi<rdx => 1st > 2nd
        jnz rlt_ret1

        mov     rax, [rsi + 8]
        cmp     [rdx + 8], rax     ; comare with (q-1)/2
        jc rlt_ret0                      ; rsi<rdx => 1st > 2nd
        jnz rlt_ret1


rlt_ret0:
        xor    rax, rax
        ret
rlt_ret1:
        mov    rax, 1
        ret



;;;;;;;;;;;;;;;;;;;;;;
; req - Raw Eq
;;;;;;;;;;;;;;;;;;;;;;
; returns in ax 1 id *rsi == *rdx
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rax <= Return 1 or 0
; Modified Registers:
;    r8, r9, rax
;;;;;;;;;;;;;;;;;;;;;;
Fr_req:
        mov    r8, [rsi]
        mov    r9, [rdx]
        bt     r8, 63          ; Check if is short first operand
        jc     req_l1
        bt     r9, 63          ; Check if is short second operand
        jc     req_s1l2

req_s1s2:                       ; Both operands are short
        cmp    r8d, r9d
        je     req_ret1
        jmp    req_ret0


req_l1:
        bt     r9, 63          ; Check if is short second operand
        jc     req_l1l2

;;;;;;;;
req_l1s2:
        bt     r8, 62          ; check if montgomery first
        jc     req_l1ms2
req_l1ns2:
        push  rdi
        push   rsi
        mov   rdi, rdx
        movsx rsi, r9d
        call  rawCopyS2L
        mov   rdx, rdi
        pop   rsi
        pop   rdi
        jmp reqL1L2

req_l1ms2:
        push rdi
        mov  rdi, rdx
        call Fr_toMontgomery
        mov  rdx, rdi
        pop  rdi
        jmp reqL1L2


;;;;;;;;
req_s1l2:
        bt     r9, 62          ; check if montgomery second
        jc     req_s1l2m
req_s1l2n:
        push  rdi
        push  rdx
        mov   rdi, rsi
        movsx rsi, r8d
        call  rawCopyS2L
        mov   rsi, rdi
        pop   rdx
        pop   rdi
        jmp reqL1L2

req_s1l2m:
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toMontgomery
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        jmp reqL1L2

;;;;
req_l1l2:
        bt     r8, 62          ; check if montgomery first
        jc     req_l1ml2
req_l1nl2:
        bt     r9, 62          ; check if montgomery second
        jc     req_l1nl2m
req_l1nl2n:
        jmp reqL1L2

req_l1nl2m:
        push rdi
        mov  rdi, rsi
        mov  rsi, rdx
        call Fr_toMontgomery
        mov  rdx, rsi
        mov  rsi, rdi
        pop  rdi
        jmp reqL1L2

req_l1ml2:
        bt     r9, 62          ; check if montgomery second
        jc     req_l1ml2m
req_l1ml2n:
        push rdi
        mov  rdi, rdx
        call Fr_toMontgomery
        mov  rdx, rdi
        pop  rdi
        jmp reqL1L2

req_l1ml2m:
        jmp reqL1L2


;;;;;;
; eqL1L2
;;;;;;

reqL1L2:

        mov     rax, [rsi + 8]
        cmp     [rdx + 8], rax
        jne     req_ret0                      ; rsi<rdi => 1st > 2nd

        mov     rax, [rsi + 16]
        cmp     [rdx + 16], rax
        jne     req_ret0                      ; rsi<rdi => 1st > 2nd

        mov     rax, [rsi + 24]
        cmp     [rdx + 24], rax
        jne     req_ret0                      ; rsi<rdi => 1st > 2nd

        mov     rax, [rsi + 32]
        cmp     [rdx + 32], rax
        jne     req_ret0                      ; rsi<rdi => 1st > 2nd


req_ret1:
        mov    rax, 1
        ret

req_ret0:
        xor    rax, rax
        ret


;;;;;;;;;;;;;;;;;;;;;;
; gt
;;;;;;;;;;;;;;;;;;;;;;
; Compares two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result can be zero or one.
; Modified Registers:
;    rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_gt:
        call Fr_rgt
        mov [rdi], rax
        ret

;;;;;;;;;;;;;;;;;;;;;;
; lt
;;;;;;;;;;;;;;;;;;;;;;
; Compares two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result can be zero or one.
; Modified Registers:
;    rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_lt:
        call Fr_rlt
        mov [rdi], rax
        ret

;;;;;;;;;;;;;;;;;;;;;;
; eq
;;;;;;;;;;;;;;;;;;;;;;
; Compares two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result can be zero or one.
; Modified Registers:
;    rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_eq:
        call Fr_req
        mov [rdi], rax
        ret

;;;;;;;;;;;;;;;;;;;;;;
; neq
;;;;;;;;;;;;;;;;;;;;;;
; Compares two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result can be zero or one.
; Modified Registers:
;    rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_neq:
        call Fr_req
        xor rax, 1
        mov [rdi], rax
        ret

;;;;;;;;;;;;;;;;;;;;;;
; geq
;;;;;;;;;;;;;;;;;;;;;;
; Compares two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result can be zero or one.
; Modified Registers:
;    rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_geq:
        call Fr_rlt
        xor rax, 1
        mov [rdi], rax
        ret

;;;;;;;;;;;;;;;;;;;;;;
; leq
;;;;;;;;;;;;;;;;;;;;;;
; Compares two elements of any kind
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result can be zero or one.
; Modified Registers:
;    rax, rcx
;;;;;;;;;;;;;;;;;;;;;;
Fr_leq:
        call Fr_rgt
        xor rax, 1
        mov [rdi], rax
        ret











;;;;;;;;;;;;;;;;;;;;;;
; land
;;;;;;;;;;;;;;;;;;;;;;
; Logical and between two elements
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result zero or one
; Modified Registers:
;    rax, rcx, r8
;;;;;;;;;;;;;;;;;;;;;;
Fr_land:






    mov     rax, [rsi]
    bt      rax, 63
    jc      tmp_108

    test    eax, eax
    jz      retZero_110
    jmp     retOne_109

tmp_108:

    mov     rax, [rsi + 8]
    test    rax, rax
    jnz     retOne_109

    mov     rax, [rsi + 16]
    test    rax, rax
    jnz     retOne_109

    mov     rax, [rsi + 24]
    test    rax, rax
    jnz     retOne_109

    mov     rax, [rsi + 32]
    test    rax, rax
    jnz     retOne_109


retZero_110:
    mov     qword r8, 0
    jmp     done_111

retOne_109:
    mov     qword r8, 1

done_111:







    mov     rax, [rdx]
    bt      rax, 63
    jc      tmp_112

    test    eax, eax
    jz      retZero_114
    jmp     retOne_113

tmp_112:

    mov     rax, [rdx + 8]
    test    rax, rax
    jnz     retOne_113

    mov     rax, [rdx + 16]
    test    rax, rax
    jnz     retOne_113

    mov     rax, [rdx + 24]
    test    rax, rax
    jnz     retOne_113

    mov     rax, [rdx + 32]
    test    rax, rax
    jnz     retOne_113


retZero_114:
    mov     qword rcx, 0
    jmp     done_115

retOne_113:
    mov     qword rcx, 1

done_115:

        and rcx, r8
        mov [rdi], rcx
        ret


;;;;;;;;;;;;;;;;;;;;;;
; lor
;;;;;;;;;;;;;;;;;;;;;;
; Logical or between two elements
; Params:
;   rsi <= Pointer to element 1
;   rdx <= Pointer to element 2
;   rdi <= Pointer to result zero or one
; Modified Registers:
;    rax, rcx, r8
;;;;;;;;;;;;;;;;;;;;;;
Fr_lor:






    mov     rax, [rsi]
    bt      rax, 63
    jc      tmp_116

    test    eax, eax
    jz      retZero_118
    jmp     retOne_117

tmp_116:

    mov     rax, [rsi + 8]
    test    rax, rax
    jnz     retOne_117

    mov     rax, [rsi + 16]
    test    rax, rax
    jnz     retOne_117

    mov     rax, [rsi + 24]
    test    rax, rax
    jnz     retOne_117

    mov     rax, [rsi + 32]
    test    rax, rax
    jnz     retOne_117


retZero_118:
    mov     qword r8, 0
    jmp     done_119

retOne_117:
    mov     qword r8, 1

done_119:







    mov     rax, [rdx]
    bt      rax, 63
    jc      tmp_120

    test    eax, eax
    jz      retZero_122
    jmp     retOne_121

tmp_120:

    mov     rax, [rdx + 8]
    test    rax, rax
    jnz     retOne_121

    mov     rax, [rdx + 16]
    test    rax, rax
    jnz     retOne_121

    mov     rax, [rdx + 24]
    test    rax, rax
    jnz     retOne_121

    mov     rax, [rdx + 32]
    test    rax, rax
    jnz     retOne_121


retZero_122:
    mov     qword rcx, 0
    jmp     done_123

retOne_121:
    mov     qword rcx, 1

done_123:

        or rcx, r8
        mov [rdi], rcx
        ret


;;;;;;;;;;;;;;;;;;;;;;
; lnot
;;;;;;;;;;;;;;;;;;;;;;
; Do the logical not of an element
; Params:
;   rsi <= Pointer to element to be tested
;   rdi <= Pointer to result one if element1 is zero and zero otherwise
; Modified Registers:
;    rax, rax, r8
;;;;;;;;;;;;;;;;;;;;;;
Fr_lnot:






    mov     rax, [rsi]
    bt      rax, 63
    jc      tmp_124

    test    eax, eax
    jz      retZero_126
    jmp     retOne_125

tmp_124:

    mov     rax, [rsi + 8]
    test    rax, rax
    jnz     retOne_125

    mov     rax, [rsi + 16]
    test    rax, rax
    jnz     retOne_125

    mov     rax, [rsi + 24]
    test    rax, rax
    jnz     retOne_125

    mov     rax, [rsi + 32]
    test    rax, rax
    jnz     retOne_125


retZero_126:
    mov     qword rcx, 0
    jmp     done_127

retOne_125:
    mov     qword rcx, 1

done_127:

        test rcx, rcx

        jz lnot_retOne
lnot_retZero:
        mov qword [rdi], 0
        ret
lnot_retOne:
        mov qword [rdi], 1
        ret


;;;;;;;;;;;;;;;;;;;;;;
; isTrue
;;;;;;;;;;;;;;;;;;;;;;
; Convert a 64 bit integer to a long format field element
; Params:
;   rsi <= Pointer to the element
; Returs:
;   rax <= 1 if true 0 if false
;;;;;;;;;;;;;;;;;;;;;;;
Fr_isTrue:
        





    mov     rax, [rdi]
    bt      rax, 63
    jc      tmp_128

    test    eax, eax
    jz      retZero_130
    jmp     retOne_129

tmp_128:

    mov     rax, [rdi + 8]
    test    rax, rax
    jnz     retOne_129

    mov     rax, [rdi + 16]
    test    rax, rax
    jnz     retOne_129

    mov     rax, [rdi + 24]
    test    rax, rax
    jnz     retOne_129

    mov     rax, [rdi + 32]
    test    rax, rax
    jnz     retOne_129


retZero_130:
    mov     qword rax, 0
    jmp     done_131

retOne_129:
    mov     qword rax, 1

done_131:

        ret





        section .data
Fr_q:
        dd      0
        dd      0x80000000
q       dq      0x3c208c16d87cfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029
half    dq      0x9e10460b6c3e7ea3,0xcbc0b548b438e546,0xdc2822db40c0ac2e,0x183227397098d014
R2      dq      0xf32cfc5b538afa89,0xb5e71911d44501fb,0x47ab1eff0a417ff6,0x06d89f71cab8351f
R3      dq      0xb1cd6dafda1530df,0x62f210e6a7283db6,0xef7f0b0c0ada0afb,0x20fd6e902d592544
lboMask dq      0x3fffffffffffffff
np      dq      0x87d20782e4866389

