# Rotate from Ace to multisig, increase threshold, rotate back to Ace.
if test $1 = r; then
    rm -rf tmp # Clear temp dir.
    mkdir tmp # Make temp dir.
    # Generate Ace keyfile.
    poetry run python amee.py keyfile generate \
        Ace \
        --vanity-prefix 0xace \
        --outfile tmp/ace.keyfile
    # Generate Bee keyfile.
    poetry run python amee.py keyfile generate \
        Bee \
        --vanity-prefix 0xbee \
        --outfile tmp/bee.keyfile
    poetry run python amee.py keyfile fund tmp/ace.keyfile # Fund Ace.
    # Incorporate into 1-of-2 multisig.
    poetry run python amee.py metafile incorporate \
        1 \
        Initial \
        --keyfiles \
            tmp/ace.keyfile \
            tmp/bee.keyfile \
        --outfile tmp/initial.multisig
    # Propose rotation challenge for rotating to multisig.
    poetry run python amee.py rotate challenge propose \
        tmp/ace.keyfile \
        tmp/initial.multisig \
        2030-01-01 \
        Initial \
        --outfile tmp/initial.challenge_proposal
    # Have Ace sign challenge proposal.
    poetry run python amee.py rotate challenge sign \
        tmp/initial.challenge_proposal \
        tmp/ace.keyfile \
        Ace initial \
        --outfile tmp/ace_initial.challenge_signature
    # Have Ace execute rotation from single-signer account.
    poetry run python amee.py rotate execute single \
        tmp/ace.keyfile \
        tmp/initial.multisig \
        tmp/ace_initial.challenge_signature
    # Increase metafile threshold to two signatures.
    poetry run python amee.py metafile threshold \
        tmp/initial.multisig \
        2 \
        Increased \
        --outfile tmp/increased.multisig
    # Propose rotation challenge for increasing threshold.
    poetry run python amee.py rotate challenge propose \
        tmp/initial.multisig \
        tmp/increased.multisig \
        2030-01-01 \
        Increase \
        --outfile tmp/increase.challenge_proposal
    # Have Ace sign challenge proposal.
    poetry run python amee.py rotate challenge sign \
        tmp/increase.challenge_proposal \
        tmp/ace.keyfile \
        Ace increase \
        --outfile tmp/ace_increase.challenge_signature
    # Have Bee sign challenge proposal.
    poetry run python amee.py rotate challenge sign \
        tmp/increase.challenge_proposal \
        tmp/bee.keyfile \
        Bee increase \
        --outfile tmp/bee_increase.challenge_signature
    # Propose rotation transaction.
    poetry run python amee.py rotate transaction propose \
        Increase transaction \
        --from-signatures tmp/ace_increase.challenge_signature \
        --to-signatures tmp/ace_increase.challenge_signature \
            tmp/bee_increase.challenge_signature \
        --outfile tmp/increase.rotation_transaction_proposal
    # Have Bee only sign rotation transaction proposal (1-of-2).
    poetry run python amee.py rotate transaction sign \
        tmp/increase.rotation_transaction_proposal \
        tmp/bee.keyfile \
        Bee increase \
        --outfile tmp/bee_increase.rotation_transaction_signature
    # Submit rotation transaction.
    poetry run python amee.py rotate execute multisig \
        tmp/initial.multisig \
        --signatures tmp/bee_increase.rotation_transaction_signature \
        --to-metafile tmp/increased.multisig
    # Propose rotation challenge for rotating back to Ace.
    poetry run python amee.py rotate challenge propose \
        tmp/increased.multisig \
        tmp/ace.keyfile \
        2030-01-01 \
        Return \
        --outfile tmp/return.challenge_proposal
    # Have Ace sign challenge proposal.
    poetry run python amee.py rotate challenge sign \
        tmp/return.challenge_proposal \
        tmp/ace.keyfile \
        Ace return \
        --outfile tmp/ace_return.challenge_signature
    # Have Bee sign challenge proposal.
    poetry run python amee.py rotate challenge sign \
        tmp/return.challenge_proposal \
        tmp/bee.keyfile \
        Bee return \
        --outfile tmp/bee_return.challenge_signature
    # Propose rotation transaction.
    poetry run python amee.py rotate transaction propose \
        Return \
        --from-signatures \
            tmp/ace_return.challenge_signature \
            tmp/bee_return.challenge_signature \
        --to-signatures tmp/ace_return.challenge_signature \
        --outfile tmp/return.rotation_transaction_proposal
    # Have Ace sign rotation transaction proposal.
    poetry run python amee.py rotate transaction sign \
        tmp/return.rotation_transaction_proposal \
        tmp/ace.keyfile \
        Ace return \
        --outfile tmp/ace_return.rotation_transaction_signature
    # Have Bee sign rotation transaction proposal.
    poetry run python amee.py rotate transaction sign \
        tmp/return.rotation_transaction_proposal \
        tmp/bee.keyfile \
        Bee return \
        --outfile tmp/bee_return.rotation_transaction_signature
    # Submit rotation transaction.
    poetry run python amee.py rotate execute multisig \
        tmp/increased.multisig \
        --signatures \
            tmp/ace_return.rotation_transaction_signature \
            tmp/bee_return.rotation_transaction_signature
    rm -rf tmp # Clear temp dir.

# Mutate metafile.
elif test $1 = m; then
    rm -rf tmp # Clear temp dir.
    mkdir tmp # Make temp dir.
    # Generate Ace keyfile.
    poetry run python amee.py keyfile generate \
        Ace \
        --vanity-prefix 0xace \
        --outfile tmp/ace.keyfile
    # Generate Bee keyfile.
    poetry run python amee.py keyfile generate \
        Bee \
        --vanity-prefix 0xbee \
        --outfile tmp/bee.keyfile
    # Incorporate into 1-of-2 multisig.
    poetry run python amee.py metafile incorporate \
        1 \
        Initial \
        --keyfiles \
            tmp/ace.keyfile \
            tmp/bee.keyfile \
        --outfile tmp/initial.multisig
    # Generate Ace keyfile.
    poetry run python amee.py keyfile generate \
        Cad \
        --vanity-prefix 0xcad \
        --outfile tmp/cad.keyfile
    # Generate Dee keyfile.
    poetry run python amee.py keyfile generate \
        Dee \
        --vanity-prefix 0xdee \
        --outfile tmp/dee.keyfile
    # Append Cad and Dee to create 3-of-4 multisig.
    poetry run python amee.py metafile append \
        tmp/initial.multisig \
        3 \
        Increased \
        --keyfiles \
            tmp/cad.keyfile \
            tmp/dee.keyfile \
        --outfile tmp/increased.multisig
    # Remove Ace and Dee to create 1-of-2 multisig.
    poetry run python amee.py metafile remove \
        tmp/increased.multisig \
        1 \
        Removed \
        --signatories 0 3 \
        --outfile tmp/removed.multisig
    # Change threshold to create 2-of-2 multisig.
    poetry run python amee.py metafile threshold \
        tmp/removed.multisig \
        2 \
        Changed \
        --outfile tmp/changed.multisig
    rm -rf tmp # Clear temp dir.

# Publish and upgrade.
elif test $1 = p; then
    rm -rf tmp # Clear temp dir.
    mkdir tmp # Make temp dir.
    # Generate Ace keyfile.
    poetry run python amee.py keyfile generate \
        Ace \
        --vanity-prefix 0xace \
        --outfile tmp/ace.keyfile
    # Generate Bee keyfile.
    poetry run python amee.py keyfile generate \
        Bee \
        --vanity-prefix 0xbee \
        --outfile tmp/bee.keyfile
    # Incorporate into 1-of-2 multisig.
    poetry run python amee.py metafile incorporate \
        1 \
        Protocol \
        --keyfiles \
            tmp/ace.keyfile \
            tmp/bee.keyfile \
        --outfile tmp/protocol.multisig
    # Fund multisig.
    poetry run python amee.py metafile fund tmp/protocol.multisig
    # Propose publication.
    poetry run python amee.py publish propose \
        tmp/protocol.multisig \
        alnoki \
        aptos-core \
        1c26076f5f \
        aptos-move/move-examples/upgrade_and_govern/v1_0_0/Move.toml \
        upgrade_and_govern \
        2030-12-31 \
        Genesis \
        --outfile tmp/genesis.publication_proposal
    # Sign publication.
    poetry run python amee.py publish sign \
        tmp/genesis.publication_proposal \
        tmp/ace.keyfile \
        Genesis \
        --outfile tmp/genesis.publication_signature
    # Execute publication.
    poetry run python amee.py publish execute tmp/genesis.publication_signature
    # Propose upgrade.
    poetry run python amee.py publish propose \
        tmp/protocol.multisig \
        alnoki \
        aptos-core \
        1c26076f5f \
        aptos-move/move-examples/upgrade_and_govern/v1_1_0/Move.toml \
        upgrade_and_govern \
        2030-12-31 \
        Upgrade \
        --outfile tmp/upgrade.publication_proposal
    # Sign upgrade publication.
    poetry run python amee.py publish sign \
        tmp/upgrade.publication_proposal \
        tmp/ace.keyfile \
        Genesis \
        --outfile tmp/upgrade.publication_signature
    # Execute upgrade publication.
    poetry run python amee.py publish execute tmp/upgrade.publication_signature
    rm -rf tmp # Clear temp dir.

# Invoke script.
elif test $1 = s; then
    rm -rf tmp # Clear temp dir.
    mkdir tmp # Make temp dir.
    # Generate Ace keyfile.
    poetry run python amee.py keyfile generate \
        Ace \
        --vanity-prefix 0xace \
        --outfile tmp/ace.keyfile
    # Generate Bee keyfile.
    poetry run python amee.py keyfile generate \
        Bee \
        --vanity-prefix 0xbee \
        --outfile tmp/bee.keyfile
    # Incorporate into 1-of-2 multisig.
    poetry run python amee.py metafile incorporate \
        1 \
        Protocol \
        --keyfiles \
            tmp/ace.keyfile \
            tmp/bee.keyfile \
        --outfile tmp/protocol.multisig
    # Fund multisig.
    poetry run python amee.py metafile fund tmp/protocol.multisig
    # Propose publication.
    poetry run python amee.py publish propose \
        tmp/protocol.multisig \
        alnoki \
        aptos-core \
        1c26076f5f \
        aptos-move/move-examples/upgrade_and_govern/v1_1_0/Move.toml \
        upgrade_and_govern \
        2030-12-31 \
        Genesis \
        --outfile tmp/genesis.publication_proposal
    # Sign publication.
    poetry run python amee.py publish sign \
        tmp/genesis.publication_proposal \
        tmp/ace.keyfile \
        Genesis \
        --outfile tmp/genesis.publication_signature
    # Execute publication.
    poetry run python amee.py publish execute tmp/genesis.publication_signature
    # Propose script invocation.
    poetry run python amee.py script propose \
        tmp/protocol.multisig \
        alnoki \
        aptos-core \
        1c26076f5f \
        aptos-move/move-examples/upgrade_and_govern/v1_1_0/Move.toml \
        upgrade_and_govern \
        set_only \
        2030-12-31 \
        Invoke \
        --outfile tmp/invoke.script_proposal
    # Sign invocation proposal.
    poetry run python amee.py script sign \
        tmp/invoke.script_proposal \
        tmp/ace.keyfile \
        Invoke \
        --outfile tmp/invoke.script_signature
    # Execute invocation.
    poetry run python amee.py script execute tmp/invoke.script_signature
    rm -rf tmp # Clear temp dir.

fi