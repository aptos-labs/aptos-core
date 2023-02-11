# Rotate from Ace to multisig, increase threshold, rotate back to Ace.
if test $1 = r; then
    rm -rf tmp # Clear temp dir.
    mkdir tmp # Make temp dir.
    # Generate Ace keyfile.
    python amee.py keyfile generate \
        Ace \
        --vanity-prefix 0xace \
        --outfile tmp/ace.keyfile
    # Generate Bob keyfile.
    python amee.py keyfile generate \
        Bob \
        --vanity-prefix 0xb0b \
        --outfile tmp/bob.keyfile
    python amee.py keyfile fund tmp/ace.keyfile # Fund Ace.
    # Incorporate into 1-of-2 multisig.
    python amee.py metafile incorporate \
        1 \
        Initial \
        --keyfiles \
            tmp/ace.keyfile \
            tmp/bob.keyfile \
        --outfile tmp/initial.multisig
    # Propose rotation challenge for rotating to multisig.
    python amee.py rotate challenge propose \
        tmp/ace.keyfile \
        tmp/initial.multisig \
        2030-01-01 \
        Initial \
        --outfile tmp/initial.challenge_proposal
    # Have Ace sign challenge proposal.
    python amee.py rotate challenge sign \
        tmp/initial.challenge_proposal \
        tmp/ace.keyfile \
        Ace initial \
        --outfile tmp/ace_initial.challenge_signature
    # Have Ace execute rotation from single-signer account.
    python amee.py rotate execute single \
        tmp/ace.keyfile \
        tmp/initial.multisig \
        tmp/ace_initial.challenge_signature
    # Increase metafile threshold to two signatures.
    python amee.py metafile threshold \
        tmp/initial.multisig \
        2 \
        Increased \
        --outfile tmp/increased.multisig
    # Propose rotation challenge for increasing threshold.
    python amee.py rotate challenge propose \
        tmp/initial.multisig \
        tmp/increased.multisig \
        2030-01-01 \
        Increase \
        --outfile tmp/increase.challenge_proposal
    # Have Ace sign challenge proposal.
    python amee.py rotate challenge sign \
        tmp/increase.challenge_proposal \
        tmp/ace.keyfile \
        Ace increase \
        --outfile tmp/ace_increase.challenge_signature
    # Have Bob sign challenge proposal.
    python amee.py rotate challenge sign \
        tmp/increase.challenge_proposal \
        tmp/bob.keyfile \
        Bob increase \
        --outfile tmp/bob_increase.challenge_signature
    # Propose rotation transaction.
    python amee.py rotate transaction propose \
        Increase transaction \
        --from-signatures tmp/ace_increase.challenge_signature \
        --to-signatures tmp/ace_increase.challenge_signature \
            tmp/bob_increase.challenge_signature \
        --outfile tmp/increase.rotation_transaction_proposal
    # Have Bob only sign rotation transaction proposal (1-of-2).
    python amee.py rotate transaction sign \
        tmp/increase.rotation_transaction_proposal \
        tmp/bob.keyfile \
        Bob increase \
        --outfile tmp/bob_increase.rotation_transaction_signature
    # Submit rotation transaction.
    python amee.py rotate execute multisig \
        tmp/initial.multisig \
        --signatures tmp/bob_increase.rotation_transaction_signature \
        --to-metafile tmp/increased.multisig
    # Propose rotation challenge for rotating back to Ace.
    python amee.py rotate challenge propose \
        tmp/increased.multisig \
        tmp/ace.keyfile \
        2030-01-01 \
        Return \
        --outfile tmp/return.challenge_proposal
    # Have Ace sign challenge proposal.
    python amee.py rotate challenge sign \
        tmp/return.challenge_proposal \
        tmp/ace.keyfile \
        Ace return \
        --outfile tmp/ace_return.challenge_signature
    # Have Bob sign challenge proposal.
    python amee.py rotate challenge sign \
        tmp/return.challenge_proposal \
        tmp/bob.keyfile \
        Bob return \
        --outfile tmp/bob_return.challenge_signature
    # Propose rotation transaction.
    python amee.py rotate transaction propose \
        Return \
        --from-signatures \
            tmp/ace_return.challenge_signature \
            tmp/bob_return.challenge_signature \
        --to-signatures tmp/ace_return.challenge_signature \
        --outfile tmp/return.rotation_transaction_proposal
    # Have Ace sign rotation transaction proposal.
    python amee.py rotate transaction sign \
        tmp/return.rotation_transaction_proposal \
        tmp/ace.keyfile \
        Ace return \
        --outfile tmp/ace_return.rotation_transaction_signature
    # Have Bob sign rotation transaction proposal.
    python amee.py rotate transaction sign \
        tmp/return.rotation_transaction_proposal \
        tmp/bob.keyfile \
        Bob return \
        --outfile tmp/bob_return.rotation_transaction_signature
    # Submit rotation transaction.
    python amee.py rotate execute multisig \
        tmp/increased.multisig \
        --signatures \
            tmp/ace_return.rotation_transaction_signature \
            tmp/bob_return.rotation_transaction_signature
    rm -rf tmp # Clear temp dir.

# Mutate metafile.
elif test $1 = m; then
    rm -rf tmp # Clear temp dir.
    mkdir tmp # Make temp dir.
    # Generate Ace keyfile.
    python amee.py keyfile generate \
        Ace \
        --vanity-prefix 0xace \
        --outfile tmp/ace.keyfile
    # Generate Bob keyfile.
    python amee.py keyfile generate \
        Bob \
        --vanity-prefix 0xb0b \
        --outfile tmp/bob.keyfile
    # Incorporate into 1-of-2 multisig.
    python amee.py metafile incorporate \
        1 \
        Initial \
        --keyfiles \
            tmp/ace.keyfile \
            tmp/bob.keyfile \
        --outfile tmp/initial.multisig
    # Generate Ace keyfile.
    python amee.py keyfile generate \
        Cad \
        --vanity-prefix 0xcad \
        --outfile tmp/cad.keyfile
    # Generate Dee keyfile.
    python amee.py keyfile generate \
        Dee \
        --vanity-prefix 0xdee \
        --outfile tmp/dee.keyfile
    # Append Cad and Dee to create 3-of-4 multisig.
    python amee.py metafile append \
        tmp/initial.multisig \
        3 \
        Increased \
        --keyfiles \
            tmp/cad.keyfile \
            tmp/dee.keyfile \
        --outfile tmp/increased.multisig
    # Remove Ace and Dee to create 1-of-2 multisig.
    python amee.py metafile remove \
        tmp/increased.multisig \
        1 \
        Removed \
        --signatories 0 3 \
        --outfile tmp/removed.multisig
    # Change threshold to create 2-of-2 multisig.
    python amee.py metafile threshold \
        tmp/removed.multisig \
        2 \
        Changed \
        --outfile tmp/changed.multisig
    rm -rf tmp # Clear temp dir.

fi