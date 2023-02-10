# Rotate from Ace to multisig, then increase threshold.
if test $1 = r; then
    mkdir tmp # Make temp dir.
    # Generate Ace keyfile.
    python amee.py k g Ace -v=0xace -k tmp/ace.keyfile
    # Generate Bob keyfile.
    python amee.py k g Bob -v=0xb0b -k tmp/bob.keyfile
    python amee.py k f tmp/ace.keyfile # Fund Ace.
    # Incorporate into 1-of-2 multisig.
    python amee.py m i Protocol -t 1 -k tmp/ace.keyfile tmp/bob.keyfile \
        -m tmp/protocol.multisig
    # Propose challenge to convert to a multisig.
    python amee.py r c p tmp/ace.keyfile tmp/protocol.multisig Rotate -f \
        -o tmp/convert.challenge_proposal
    # Have Ace sign challenge.
    python amee.py r c s tmp/convert.challenge_proposal tmp/ace.keyfile \
        Ace convert -o tmp/ace_convert.challenge_signature
    # Have Ace execute single-signer rotation.
    python amee.py r e s tmp/ace.keyfile tmp/protocol.multisig \
        tmp/ace_convert.challenge_signature
    # Increase threshold to two signatures.
    python amee.py m t Increased -m tmp/protocol.multisig -t 2 \
        -n tmp/increased.multisig
    # Propose challenge.
    python amee.py r c p tmp/protocol.multisig tmp/increased.multisig \
        Increase threshold challenge -o tmp/increase.challenge_proposal
    # Have Ace sign challenge.
    python amee.py r c s tmp/increase.challenge_proposal tmp/ace.keyfile \
        Ace increase -o tmp/ace_increase.challenge_signature
    # Have Bob sign challenge.
    python amee.py r c s tmp/increase.challenge_proposal tmp/bob.keyfile \
        Bob increase -o tmp/bob_increase.challenge_signature
    # Propose rotation transaction.
    python amee.py r t p Increase transaction \
        -f tmp/ace_increase.challenge_signature \
        -t tmp/ace_increase.challenge_signature \
            tmp/bob_increase.challenge_signature \
        -o tmp/increase_threshold.rotation_transaction_proposal


    rm -rf tmp # Clear temp dir.

# Mutate metafile.
elif test $1 = m; then
    mkdir tmp # Make temp dir.
    # Generate Ace keyfile.
    python amee.py k g Ace -v=0xace -k tmp/ace.keyfile
    # Generate Bob keyfile.
    python amee.py k g Bob -v=0xb0b -k tmp/bob.keyfile
    # Incorporate into 1-of-2 multisig.
    python amee.py m i Protocol -t 1 -k tmp/ace.keyfile tmp/bob.keyfile \
        -m tmp/protocol.multisig
    # Generate Cad keyfile.
    python amee.py k g Cad -v=0xcad -k tmp/cad.keyfile
    # Make Dee signer.
    python amee.py k g Dee -v=0xdee -k tmp/dee.keyfile
    # Append Cad and Dee to create 3-of-4 multisig.
    python amee.py m a New Protocol -m tmp/protocol.multisig -t 3 \
        -k tmp/cad.keyfile tmp/dee.keyfile -n tmp/new_protocol.multisig
    # Remove Ace and Dee to create 1-of-2 multisig.
    python amee.py m r Third Protocol -m tmp/new_protocol.multisig -t 1 \
        -s 0 3 -n tmp/third_protocol.multisig
    # Change threshold to create 2-of-2 multisig.
    python amee.py m t Fourth Protocol -m tmp/third_protocol.multisig -t 2 \
        -n tmp/fourth_protocol.multisig
    rm -rf tmp # Clear temp dir.

fi