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
print_heading() {
    print_lines 2
    echo === $@ ===
    print_lines 2
}


# Helper functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

# Return if no arguments passed
if test "$#" = 0; then return

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

# Invoke keyfile operations.
elif test $1 = keyfiles; then

    rm -f the_aptos_foundation.keyfile
    rm -f the_aptos_foundation.account_store
    rm -f from_store.keyfile

    print_heading Generate
    # :!:>generate_keyfile
    poetry run python amee.py keyfile generate \
        The Aptos Foundation # <:!:generate_keyfile

    print_heading Extract
    # :!:>extract_keyfile
    poetry run python amee.py k extract \
        the_aptos_foundation.keyfile \
        the_aptos_foundation.account_store # <:!:extract_keyfile

    print_heading Generate from store
    # :!:>generate_from_store
    poetry run python amee.py keyfile g \
        The Aptos Foundation \
        --account-store the_aptos_foundation.account_store \
        --outfile from_store.keyfile # <:!:generate_from_store

    print_heading Change password
    # :!:>change_password
    poetry run python amee.py keyfile change-password \
        from_store.keyfile # <:!:change_password

    print_heading Verify
    # :!:>verify_password
    poetry run python amee.py keyfile verify \
        from_store.keyfile # <:!:verify_password

    print_heading Deleting keyfiles and account store
    rm -f the_aptos_foundation.keyfile
    rm -f the_aptos_foundation.account_store
    rm -f from_store.keyfile
fi