# This file contains an assortment of shell scripts for to the AMEE
# tutorial (see "Your First Multisig").
#
# All scripts are intended to be run from the `aptos_sdk` Python package
# directory (where amee.py is located), and use a single argument. For
# example, to display AMEE's subcommand menus, run the following:
#
# sh ../examples/multisig.sh menus

# Helper functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

# Print n lines of whitespace between commands, where n is argument.
print_lines() {
    for ((i=0;i<$1;i++)); do
        echo
    done
}

# Print a whitespace break between calls.
print_break() {
    print_lines 5
}

# Print a separator message using all arguments taken as a string.
heading() {
    print_lines 2
    echo === $@ ===
    print_lines 2
}

# Wait for user to press Enter.
wait() {
    print_lines 2
    read -p "Press Enter to continue..."
}


# Helper functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

# Demo scripts >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

# Return if no arguments passed
if test "$#" = 0; then echo No subscript specified

# Display menus.
elif test $1 = menus; then

    # :!:>help
    # Print top-level menu.
    poetry run python amee.py -h # <:!:help

        # Print nested subcommand help menus with whitespace in between.
        print_break
        poetry run python amee.py keyfile -h
            print_break
            poetry run python amee.py keyfile change-password -h
            print_break
            poetry run python amee.py keyfile extract -h
            print_break
            poetry run python amee.py keyfile fund -h
            print_break
            poetry run python amee.py keyfile generate -h
            print_break
            poetry run python amee.py keyfile verify -h
        print_break
        poetry run python amee.py metafile -h
            print_break
            poetry run python amee.py metafile append -h
            print_break
            poetry run python amee.py metafile fund -h
            print_break
            poetry run python amee.py metafile incorporate -h
            print_break
            poetry run python amee.py metafile remove -h
            print_break
            poetry run python amee.py metafile threshold -h
        print_break
        poetry run python amee.py publish -h
            print_break
            poetry run python amee.py publish execute -h
            print_break
            poetry run python amee.py publish propose -h
            print_break
            poetry run python amee.py publish sign -h
        print_break
        poetry run python amee.py rotate -h
            print_break
            poetry run python amee.py rotate challenge -h
                print_break
                poetry run python amee.py rotate challenge propose -h
                print_break
                poetry run python amee.py rotate challenge sign -h
            print_break
            poetry run python amee.py rotate execute -h
                print_break
                poetry run python amee.py rotate execute single -h
                print_break
                poetry run python amee.py rotate execute multisig -h
            print_break
            poetry run python amee.py rotate transaction -h
                print_break
                poetry run python amee.py rotate transaction propose -h
                print_break
                poetry run python amee.py rotate transaction sign -h
        print_break
        poetry run python amee.py script -h
            print_break
            poetry run python amee.py script execute -h
            print_break
            poetry run python amee.py script propose -h
            print_break
            poetry run python amee.py script sign -h

# Demo keyfile operations.
elif test $1 = keyfiles; then

    rm -f *.keyfile
    rm -f *.account_store

    heading Generate
    # :!:>generate_keyfile
    poetry run python amee.py keyfile generate \
        The Aptos Foundation # <:!:generate_keyfile

    wait

    heading Extract
    # :!:>extract_keyfile
    poetry run python amee.py k extract \
        the_aptos_foundation.keyfile \
        the_aptos_foundation.account_store # <:!:extract_keyfile

    wait

    heading Generate from store
    # :!:>generate_from_store
    poetry run python amee.py keyfile g \
        The Aptos Foundation \
        --account-store the_aptos_foundation.account_store \
        --outfile from_store.keyfile # <:!:generate_from_store

    wait

    heading Change password
    # :!:>change_password
    poetry run python amee.py keyfile change-password \
        from_store.keyfile # <:!:change_password

    wait

    heading Verify
    # :!:>verify_password
    poetry run python amee.py keyfile verify \
        from_store.keyfile # <:!:verify_password

    wait

    heading Deleting keyfiles and account store

    rm -f *.keyfile
    rm -f *.account_store


# Demo metafile operations.
elif test $1 = metafiles; then

    rm -f *.keyfile
    rm -f *.multisig

    # :!:>metafiles_ace_bee
    heading Generate vanity account for Ace

    poetry run python amee.py keyfile generate \
        Ace \
        --vanity-prefix 0xace \
        --use-test-password

    heading Generate vanity account for Bee

    poetry run python amee.py keyfile generate \
        Bee \
        --vanity-prefix 0xbee \
        --use-test-password # <:!:metafiles_ace_bee

    wait

    # :!:>metafiles_incorporate
    heading Incorporate Ace and Bee into 1-of-2 multisig

    poetry run python amee.py metafile incorporate \
        1 \
        Ace and Bee \
        --keyfiles \
            ace.keyfile \
            bee.keyfile # <:!:metafiles_incorporate

    wait

    # :!:>metafiles_threshold
    heading Increase threshold to two signatures

    poetry run python amee.py metafile threshold \
        ace_and_bee.multisig \
        2 \
        Ace and Bee increased # <:!:metafiles_threshold

    wait

    # :!:>metafiles_cad_dee
    heading Generate vanity account for Cad

    poetry run python amee.py keyfile generate \
        Cad \
        --vanity-prefix 0xcad \
        --use-test-password

    heading Generate vanity account for Dee

    poetry run python amee.py keyfile generate \
        Dee \
        --vanity-prefix 0xdee \
        --use-test-password # <:!:metafiles_cad_dee

    wait

    # :!:>metafiles_append
    heading Append Cad and Dee to 3-of-4 multisig

    poetry run python amee.py metafile append \
        ace_and_bee.multisig \
        3 \
        Cad and Dee added \
        --keyfiles \
            cad.keyfile \
            dee.keyfile # <:!:metafiles_append

    wait

    # :!:>metafiles_remove
    heading Remove Ace and Dee for 1-of-2 multisig

    poetry run python amee.py metafile remove \
        cad_and_dee_added.multisig \
        1 \
        Ace and Dee removed \
        --signatories 0 3 # <:!:metafiles_remove

    wait

    heading Deleting keyfiles and metafiles

    rm -f *.keyfile
    rm -f *.multisig

# Demo authentication key rotation operations.
elif test $1 = rotate; then

    rm -f *.keyfile
    rm -f *.multisig
    rm -f *.challenge_proposal
    rm -f *.challenge_signature
    rm -f *.rotation_transaction_proposal
    rm -f *.rotation_transaction_signature

    # :!:>rotate_prep_accounts
    heading Generate vanity account for Ace

    poetry run python amee.py keyfile generate \
        Ace \
        --vanity-prefix 0xace \
        --use-test-password

    heading Generate vanity account for Bee

    poetry run python amee.py keyfile generate \
        Bee \
        --vanity-prefix 0xbee \
        --use-test-password

    heading Fund Ace on devnet

    poetry run python amee.py keyfile fund \
        ace.keyfile # <:!:rotate_prep_accounts

    wait

    # :!:>rotate_convert_multisig
    heading Incorporate to 1-of-2 multisig

    poetry run python amee.py metafile incorporate \
        1 \
        Initial \
        --keyfiles \
            ace.keyfile \
            bee.keyfile

    heading  Propose rotation challenge for rotating to multisig

    poetry run python amee.py rotate challenge propose \
        ace.keyfile \
        initial.multisig \
        2030-01-01 \
        Initial \
        --network devnet

    heading  Have Ace sign challenge proposal

    poetry run python amee.py rotate challenge sign \
        initial.challenge_proposal \
        ace.keyfile \
        Ace initial \
        --use-test-password

    heading Have Ace execute rotation from single-signer account

    poetry run python amee.py rotate execute single \
        ace.keyfile \
        initial.multisig \
        ace_initial.challenge_signature \
        --use-test-password \
        --network devnet # <:!:rotate_convert_multisig

    wait

    # :!:>rotate_increase_propose
    heading Increase metafile threshold to two signatures

    poetry run python amee.py metafile threshold \
        initial.multisig \
        2 \
        Increased

    heading Propose rotation challenge for increasing threshold

    poetry run python amee.py rotate challenge propose \
        initial.multisig \
        increased.multisig \
        2030-01-01 \
        Increase \
        --network devnet

    heading Have Ace sign challenge proposal

    poetry run python amee.py rotate challenge sign \
        increase.challenge_proposal \
        ace.keyfile \
        Ace increase \
        --use-test-password

    heading Have Bee sign challenge proposal

    poetry run python amee.py rotate challenge sign \
        increase.challenge_proposal \
        bee.keyfile \
        Bee increase \
        --use-test-password # <:!:rotate_increase_propose

    wait

    # :!:>rotate_increase_execute
    heading Propose rotation transaction

    poetry run python amee.py rotate transaction propose \
        Increase \
        --from-signatures \
            ace_increase.challenge_signature \
        --to-signatures \
            ace_increase.challenge_signature \
            bee_increase.challenge_signature \

    heading Have Bee only sign rotation transaction proposal

    poetry run python amee.py rotate transaction sign \
        increase.rotation_transaction_proposal \
        bee.keyfile \
        Bee increase \
        --use-test-password

    heading Submit rotation transaction

    poetry run python amee.py rotate execute multisig \
        initial.multisig \
        --signatures \
            bee_increase.rotation_transaction_signature \
        --to-metafile \
            increased.multisig # <:!:rotate_increase_execute

    wait

    # :!:>rotate_convert_single_propose
    heading Propose rotation challenge for rotating back to Ace

    poetry run python amee.py rotate challenge propose \
        increased.multisig \
        ace.keyfile \
        2030-01-01 \
        Return \
        --network devnet

    heading Have Ace sign challenge proposal

    poetry run python amee.py rotate challenge sign \
        return.challenge_proposal \
        ace.keyfile \
        Ace return \
        --use-test-password

    heading Have Bee sign challenge proposal

    poetry run python amee.py rotate challenge sign \
        return.challenge_proposal \
        bee.keyfile \
        Bee return \
        --use-test-password # <:!:rotate_convert_single_propose

    wait

    # :!:>rotate_convert_single_execute
    heading Propose rotation transaction

    poetry run python amee.py rotate transaction propose \
        Return \
        --from-signatures \
            ace_return.challenge_signature \
            bee_return.challenge_signature \
        --to-signatures \
            ace_return.challenge_signature \

    heading Have Ace sign rotation transaction proposal

    poetry run python amee.py rotate transaction sign \
        return.rotation_transaction_proposal \
        ace.keyfile \
        Ace return \
        --use-test-password

    heading Have Bee sign rotation transaction proposal

    poetry run python amee.py rotate transaction sign \
        return.rotation_transaction_proposal \
        bee.keyfile \
        Bee return \
        --use-test-password

    heading Submit rotation transaction

    poetry run python amee.py rotate execute multisig \
        increased.multisig \
        --signatures \
            ace_return.rotation_transaction_signature \
            bee_return.rotation_transaction_signature \
        --network devnet # <:!:rotate_convert_single_execute

    wait

    heading Deleting JSON files

    rm -f *.keyfile
    rm -f *.multisig
    rm -f *.challenge_proposal
    rm -f *.challenge_signature
    rm -f *.rotation_transaction_proposal
    rm -f *.rotation_transaction_signature


# Demo governance operations.
elif test $1 = govern; then

    rm -f *.keyfile
    rm -f *.multisig
    rm -f *.publication_proposal
    rm -f *.publication_signature
    rm -f *.script_proposal
    rm -f *.script_signature

    # :!:>govern_prep_accounts
    heading Generate vanity account for Ace

    poetry run python amee.py keyfile generate \
        Ace \
        --vanity-prefix 0xace \
        --use-test-password

    heading Generate vanity account for Bee

    poetry run python amee.py keyfile generate \
        Bee \
        --vanity-prefix 0xbee \
        --use-test-password # <:!:govern_prep_accounts

    wait

    # :!:>govern_prep_multisig
    heading Incorporate to 1-of-2 multisig

    poetry run python amee.py metafile incorporate \
        1 \
        Protocol \
        --keyfiles \
            ace.keyfile \
            bee.keyfile

    heading Fund multisig

    poetry run python amee.py metafile fund \
        protocol.multisig # <:!:govern_prep_multisig

    wait

    # :!:>govern_publish
    heading Propose publication

    poetry run python amee.py publish propose \
        protocol.multisig \
        alnoki \
        aptos-core \
        1c26076f5f \
        aptos-move/move-examples/upgrade_and_govern/v1_0_0/Move.toml \
        upgrade_and_govern \
        2030-12-31 \
        Genesis \
        --network devnet

    heading Sign publication proposal

    poetry run python amee.py publish sign \
        genesis.publication_proposal \
        ace.keyfile \
        Genesis \
        --use-test-password

    heading Execute publication

    poetry run python amee.py publish execute \
        genesis.publication_signature \
        --network devnet # <:!:govern_publish

    wait

    # :!:>govern_upgrade
    heading Propose upgrade

    poetry run python amee.py publish propose \
        protocol.multisig \
        alnoki \
        aptos-core \
        1c26076f5f \
        aptos-move/move-examples/upgrade_and_govern/v1_1_0/Move.toml \
        upgrade_and_govern \
        2030-12-31 \
        Upgrade \
        --network devnet

    heading Sign upgrade proposal

    poetry run python amee.py publish sign \
        upgrade.publication_proposal \
        ace.keyfile \
        Upgrade \
        --use-test-password

    heading Execute upgrade

    poetry run python amee.py publish execute \
        upgrade.publication_signature \
        --network devnet # <:!:govern_upgrade

    wait

    # :!:>govern_script
    heading Propose script invocation

    poetry run python amee.py script propose \
        protocol.multisig \
        alnoki \
        aptos-core \
        1c26076f5f \
        aptos-move/move-examples/upgrade_and_govern/v1_1_0/Move.toml \
        upgrade_and_govern \
        set_only \
        2030-12-31 \
        Invoke \
        --network devnet

    heading Sign invocation proposal

    poetry run python amee.py script sign \
        invoke.script_proposal \
        ace.keyfile \
        Invoke \
        --use-test-password

    heading Execute script invocation

    poetry run python amee.py script execute \
        invoke.script_signature \
        --network devnet # <:!:govern_script

    wait

    heading Deleting JSON files

    rm -f *.keyfile
    rm -f *.multisig
    rm -f *.publication_proposal
    rm -f *.publication_signature
    rm -f *.script_proposal
    rm -f *.script_signature

else echo Invalid subscript name

fi

# Demo scripts <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
