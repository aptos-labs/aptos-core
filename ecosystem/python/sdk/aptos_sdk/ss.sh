# Rotate from Ace to multisig.
if test $1 = r; then
    mkdir tmp # Make temp dir.
    python amee.py k g Ace -v=0xace -k tmp/ace.keyfile # Make Ace signer.
    python amee.py k g Bob -v=0xb0b -k tmp/bob.keyfile # Make Bob signer.
    python amee.py k f tmp/ace.keyfile # Fund Ace.
    # Incorporate into 1-of-2 multisig.
    python amee.py m i Protocol -t 1 -k tmp/ace.keyfile tmp/bob.keyfile \
        -m tmp/protocol.multisig
    # Propose challenge.
    python amee.py r c p tmp/ace.keyfile tmp/protocol.multisig Convert -s \
        -o tmp/convert.challenge_proposal
    # Have Ace sign challenge.
    python amee.py r c s tmp/convert.challenge_proposal tmp/ace.keyfile \
        Ace approval -o tmp/ace_approval.challenge_signature
    # Have Ace execute signle-signer rotation.
    python amee.py r e s tmp/ace.keyfile tmp/protocol.multisig \
        tmp/ace_approval.challenge_signature
    rm -rf tmp # Clear temp dir.

# Mutate metafile.
elif test $1 = m; then
    mkdir tmp # Make temp dir.
    python amee.py k g Ace -v=0xace -k tmp/ace.keyfile # Make Ace signer.
    python amee.py k g Bob -v=0xb0b -k tmp/bob.keyfile # Make Bob signer.
    # Incorporate into 1-of-2 multisig.
    python amee.py m i Protocol -t 1 -k tmp/ace.keyfile tmp/bob.keyfile \
        -m tmp/protocol.multisig
    python amee.py k g Cad -v=0xcad -k tmp/cad.keyfile # Make Cad signer.
    python amee.py k g Dee -v=0xdee -k tmp/dee.keyfile # Make Dee signer.
    # Append both to create 3-of-4 multisig.
    python amee.py m a New Protocol -m tmp/protocol.multisig -t 3 \
        -k tmp/cad.keyfile tmp/dee.keyfile -n tmp/new_protocol.multisig
    # Remove Ace and Dee to create 1-of-2 multisig.
    python amee.py m r Third Protocol -m tmp/new_protocol.multisig -t 1 \
        -s 0 3 -n tmp/third_protocol.multisig
    # Change threshold to create 1-of-2 multisig.
    python amee.py m t Fourth Protocol -m tmp/third_protocol.multisig -t 2 \
        -n tmp/fourth_protocol.multisig
    rm -rf tmp # Clear temp dir.

fi

