# Account addresses.
ACCOUNTS_DIR=/app/accounts
ACE_ADDR=$(cat $ACCOUNTS_DIR/ace.address)
BEE_ADDR=$(cat $ACCOUNTS_DIR/bee.address)
CAD_ADDR=$(cat $ACCOUNTS_DIR/cad.address)
DEE_ADDR=$(cat $ACCOUNTS_DIR/dee.address)

# Functions.
MINT_DEE_COIN=$DEE_ADDR::dee_coin::mint
MINT_USDC=$DEE_ADDR::usdc::mint
CREATE_POOL=0x1::lockstream::create
LOCK=0x1::lockstream::lock
METADATA=0x1::lockstream::metadata
LOCKERS=0x1::lockstream::lockers
CLAIM=0x1::lockstream::claim
BALANCE=0x1::coin::balance

# Types.
DEE_COIN_TYPE=$DEE_ADDR::dee_coin::DeeCoin
USDC_COIN_TYPE=$DEE_ADDR::usdc::USDC

# Coin amounts.
DEE_COIN_MINT=10000
ACE_USDC_LOCK_1=100
ACE_USDC_LOCK_2=400
ACE_USDC_MINT=$(expr $ACE_USDC_LOCK_1 + $ACE_USDC_LOCK_2)
BEE_USDC_LOCK=200
BEE_USDC_MINT=$BEE_USDC_LOCK
CAD_USDC_LOCK=300
CAD_USDC_MINT=$CAD_USDC_LOCK

# Period start delays, relative to prior time in sequence.
STREAM_START_DELAY=20
STREAM_END_DELAY=60
CLAIM_LAST_CALL_DELAY=30
PREMIER_SWEEP_LAST_CALL_DELAY=30

# Claim times.
ACE_CLAIM_TIME_1=15
BEE_CLAIM_TIME_1=30
CAD_CLAIM_TIME=45
ACE_CLAIM_TIME_2=60
BEE_CLAIM_TIME_2=70

# Print account addresses.
echo "Accounts:
Ace: $ACE_ADDR
Bee: $BEE_ADDR
Cad: $CAD_ADDR
Dee: $DEE_ADDR\n"

# Fund users
echo Minting $DEE_COIN_MINT DeeCoin to Dee:
sleep 1
aptos move run \
    --args u64:$DEE_COIN_MINT \
    --assume-yes \
    --function-id $MINT_DEE_COIN \
    --profile dee \

echo "\n\nMinting $ACE_USDC_MINT USDC to Ace:"
sleep 2
aptos move run \
    --args u64:$ACE_USDC_MINT \
    --assume-yes \
    --function-id $MINT_USDC \
    --profile ace \

echo "\n\nMinting $BEE_USDC_MINT USDC to Bee:"
sleep 2
aptos move run \
    --args u64:$BEE_USDC_MINT \
    --assume-yes \
    --function-id $MINT_USDC \
    --profile bee \

echo "\n\nMinting $CAD_USDC_MINT USDC to Cad:"
sleep 2
aptos move run \
    --args u64:$CAD_USDC_MINT \
    --assume-yes \
    --function-id $MINT_USDC \
    --profile cad \

# Calculate period times relative to creation time.
sleep 2
CREATION_TIME=$(date +%s)
STREAM_START_TIME=$(expr $CREATION_TIME + $STREAM_START_DELAY)
STREAM_END_TIME=$(expr $STREAM_START_TIME + $STREAM_END_DELAY)
CLAIM_LAST_CALL_TIME=$(expr $STREAM_END_TIME + $CLAIM_LAST_CALL_DELAY)
PREMIER_SWEEP_LAST_CALL_TIME=$(
    expr $CLAIM_LAST_CALL_TIME + $PREMIER_SWEEP_LAST_CALL_DELAY)

# Create pool.
echo "\n\nCreating pool at $CREATION_TIME:"
aptos move run \
    --args \
        u64:$DEE_COIN_MINT \
        u64:$STREAM_START_TIME \
        u64:$STREAM_END_TIME \
        u64:$CLAIM_LAST_CALL_TIME \
        u64:$PREMIER_SWEEP_LAST_CALL_TIME \
    --assume-yes \
    --function-id $CREATE_POOL \
    --profile dee \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

# Lock assets.
echo "\n\n Locking $ACE_USDC_LOCK_1 USDC for Ace into pool:"
sleep 2
aptos move run \
    --args \
        address:$DEE_ADDR \
        u64:$ACE_USDC_LOCK_1 \
    --assume-yes \
    --function-id $LOCK \
    --profile ace \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

echo "\n\n Locking $BEE_USDC_LOCK USDC for Bee into pool:"
sleep 2
aptos move run \
    --args \
        address:$DEE_ADDR \
        u64:$BEE_USDC_LOCK \
    --assume-yes \
    --function-id $LOCK \
    --profile bee \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

echo "\n\n Locking $CAD_USDC_LOCK USDC for Cad into pool:"
sleep 2
aptos move run \
    --args \
        address:$DEE_ADDR \
        u64:$CAD_USDC_LOCK \
    --assume-yes \
    --function-id $LOCK \
    --profile cad \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

echo "\n\n Locking $ACE_USDC_LOCK_2 more USDC for Ace into pool:"
sleep 2
aptos move run \
    --args \
        address:$DEE_ADDR \
        u64:$ACE_USDC_LOCK_2 \
    --assume-yes \
    --function-id $LOCK \
    --profile ace \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

# Print diagnostic info.
echo "\n\n Pool metadata:"
sleep 2
aptos move view \
    --args address:$DEE_ADDR \
    --function-id $METADATA \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE \
    --profile ace

echo "\n\n Locker info:"
sleep 2
aptos move view \
    --args address:$DEE_ADDR \
    --function-id $LOCKERS \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE \
    --profile ace

# Wait until lockstream has started.
CURRENT_TIME=$(date +%s)
DELAY=$(expr $STREAMING_PERIOD_START - $CURRENT_TIME)
echo "\n\n The time is now $CURRENT_TIME
The streaming period starts at $STREAMING_PERIOD_START
Waiting $DELAY seconds"
sleep $DELAY

# Wait another second.
echo "\n\n The streaming period has begun
Waiting another second"
sleep 1

echo "\n\n Locker info:"
aptos move view \
    --args address:$DEE_ADDR \
    --function-id $LOCKERS \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE \
    --profile ace

# Make Ace's first claim.
echo "\n\n Waiting for $ACE_CLAIM_TIME_1 seconds into stream for Ace's claim 1"
CURRENT_TIME=$(date +%s)
DELAY=$(expr $STREAMING_PERIOD_START + $ACE_CLAIM_TIME_1 - $CURRENT_TIME)
sleep $DELAY

echo "\n\n Claiming for Ace:"
aptos move run \
    --args \
        address:$DEE_ADDR \
    --assume-yes \
    --function-id $CLAIM \
    --profile ace \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

echo "\n\n Ace's DeeCoin balance:"
aptos move view \
    --args address:$ACE_ADDR \
    --function-id $BALANCE \
    --type-args $DEE_COIN_TYPE \
    --profile ace

# Make Bee's claim.
echo "\n\n Waiting for $BEE_CLAIM_TIME_1 seconds into stream for Bee's claim 1"
CURRENT_TIME=$(date +%s)
DELAY=$(expr $STREAMING_PERIOD_START + $BEE_CLAIM_TIME_1 - $CURRENT_TIME)
sleep $DELAY

echo "\n\n Claiming for Bee:"
aptos move run \
    --args \
        address:$DEE_ADDR \
    --assume-yes \
    --function-id $CLAIM \
    --profile bee \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

echo "\n\n Bee's DeeCoin balance:"
aptos move view \
    --args address:$BEE_ADDR \
    --function-id $BALANCE \
    --type-args $DEE_COIN_TYPE \
    --profile bee

# Make Cad's claim.
echo "\n\n Waiting for $CAD_CLAIM_TIME seconds into stream for Cad's claim"
CURRENT_TIME=$(date +%s)
DELAY=$(expr $STREAMING_PERIOD_START + $CAD_CLAIM_TIME - $CURRENT_TIME)
sleep $DELAY

echo "\n\n Claiming for Cad:"
aptos move run \
    --args \
        address:$DEE_ADDR \
    --assume-yes \
    --function-id $CLAIM \
    --profile cad \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

echo "\n\n Cad's DeeCoin balance:"
aptos move view \
    --args address:$CAD_ADDR \
    --function-id $BALANCE \
    --type-args $DEE_COIN_TYPE \
    --profile cad

# Make Ace's second claim.
echo "\n\n Wait for $ACE_CLAIM_TIME_2 seconds after stream start for Ace's claim 2"
CURRENT_TIME=$(date +%s)
DELAY=$(expr $STREAMING_PERIOD_START + $ACE_CLAIM_TIME_2 - $CURRENT_TIME)
sleep $DELAY

echo "\n\n Claiming for Ace:"
aptos move run \
    --args \
        address:$DEE_ADDR \
    --assume-yes \
    --function-id $CLAIM \
    --profile ace \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

echo "\n\n Ace's DeeCoin balance:"
aptos move view \
    --args address:$ACE_ADDR \
    --function-id $BALANCE \
    --type-args $DEE_COIN_TYPE \
    --profile ace

# Make Bee's claim.
echo "\n\n Waiting for $BEE_CLAIM_TIME_2 seconds after stream start Bee's claim 2"
CURRENT_TIME=$(date +%s)
DELAY=$(expr $STREAMING_PERIOD_START + $BEE_CLAIM_TIME_2 - $CURRENT_TIME)
sleep $DELAY

echo "\n\n Claiming for Bee:"
aptos move run \
    --args \
        address:$DEE_ADDR \
    --assume-yes \
    --function-id $CLAIM \
    --profile bee \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE

echo "\n\n Bee's DeeCoin balance:"
aptos move view \
    --args address:$BEE_ADDR \
    --function-id $BALANCE \
    --type-args $DEE_COIN_TYPE \
    --profile bee

# Print diagnostic info.
echo "\n\n Pool metadata:"
sleep 2
aptos move view \
    --args address:$DEE_ADDR \
    --function-id $METADATA \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE \
    --profile ace

echo "\n\n Locker info:"
sleep 2
aptos move view \
    --args address:$DEE_ADDR \
    --function-id $LOCKERS \
    --type-args \
        $DEE_COIN_TYPE \
        $USDC_COIN_TYPE \
    --profile ace

# Quote claim amounts.

echo "\n\n Ace's USDC balance:"
sleep 2
aptos move view \
    --args address:$ACE_ADDR \
    --function-id $BALANCE \
    --type-args $USDC_COIN_TYPE \
    --profile ace

echo "\n\n Bee's USDC balance:"
sleep 2
aptos move view \
    --args address:$BEE_ADDR \
    --function-id $BALANCE \
    --type-args $USDC_COIN_TYPE \
    --profile bee

echo "\n\n Cad's USDC balance:"
sleep 2
aptos move view \
    --args address:$BEE_ADDR \
    --function-id $BALANCE \
    --type-args $USDC_COIN_TYPE \
    --profile cad