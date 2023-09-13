ACCOUNTS_DIR=/app/accounts

ACE_ADDR=$(cat $ACCOUNTS/ace.address)
BEE_ADDR=$(cat $ACCOUNTS/bee.address)
CAD_ADDR=$(cat $ACCOUNTS/cad.address)
DEE_ADDR=$(cat $ACCOUNTS/dee.address)

DEE_COIN_MINT=10000

echo Accounts:
echo Ace: $ACE_ADDR
echo Bee: $BEE_ADDR
echo Cad: $CAD_ADDR
echo Dee: $DEE_ADDR

wait 5

echo Minting $DEE_COIN_MINT DeeCoin to Dee


