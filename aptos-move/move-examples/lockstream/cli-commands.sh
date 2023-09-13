# Load in/print account addresses.
ACCOUNTS_DIR=/app/accounts
ACE_ADDR=$(cat $ACCOUNTS_DIR/ace.address)
BEE_ADDR=$(cat $ACCOUNTS_DIR/bee.address)
CAD_ADDR=$(cat $ACCOUNTS_DIR/cad.address)
DEE_ADDR=$(cat $ACCOUNTS_DIR/dee.address)
echo "Accounts:
Ace: $ACE_ADDR
Bee: $BEE_ADDR
Cad: $CAD_ADDR
Dee: $DEE_ADDR\n\n"

echo $(cat .aptos/config.yaml)

# Declare functions.
MINT_DEE_COIN=$DEE_ADDR::dee_coin::mint

# Declare types.
DEE_COIN_TYPE=$DEE_ADDR::dee_coin::DeeCoin

#wait 5

DEE_COIN_MINT_AMOUNT=10000
echo Minting $DEE_COIN_MINT_AMOUNT DeeCoin to Dee
aptos move run \
    --args u64:$DEE_COIN_MINT_AMOUNT \
    --assume-yes \
    --function-id $MINT_DEE_COIN \
    --profile dee \
    --type-args $DEE_COIN_TYPE


