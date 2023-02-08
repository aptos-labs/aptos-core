# Rotate convert from Ace to multisig.
if test $1 = t; then
    mkdir tmp # Make temp dir.
    python amee.py k g Ace -v=0xa -k tmp/ace.keyfile # Make Ace signer.
    python amee.py k g Bob -v=0xb -k tmp/bob.keyfile # Make Bob signer.
    python amee.py k f tmp/ace.keyfile # Fund Ace.
    # Incorporate into 1-of-2 multisig.
    python amee.py m i Protocol -t 1 -k tmp/ace.keyfile tmp/bob.keyfile -m tmp/protocol.multisig
    # Propose challenge.
    python amee.py r c p tmp/ace.keyfile tmp/protocol.multisig Convert -s -o tmp/convert.challenge_proposal
    # Have Ace sign challenge.
    python amee.py r c s tmp/convert.challenge_proposal tmp/ace.keyfile Ace approval -o tmp/ace_approval.challenge_signature
    # Have Ace execute rotation conversion.
    python amee.py r e c tmp/ace.keyfile tmp/protocol.multisig tmp/ace_approval.challenge_signature
    rm -rf tmp # Clear temp dir.

fi