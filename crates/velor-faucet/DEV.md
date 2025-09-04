# Velor Faucet

This doc contains information relevant to the development of more obscure parts of the faucet.

## Manually testing Checkers
Run a local tap with a fake funder and in-memory storage:
```
cargo run -- run -c configs/testing_checkers.yaml
```

See that a regular request fails:
```
$ curl -H 'Content-Type: application/json' -X POST -d '{"amount": 5, "address": "3c769ea16f38fdc218341c63ff8c1c5c7dcbb4d5d850675e92b09997fd36e8f0"}' localhost:10212/fund
{
  "error_code": "Rejected",
  "message": "Request rejected by 2 checkers",
  "rejection_reasons": [
    {
      "code": "CaptchaInvalid",
      "reason": "Captcha header CAPTCHA_KEY not found"
    },
    {
      "code": "RequestNotFromIntendedSource",
      "reason": "Magic header what_wallet not found"
    }
  ],
  "txn_hashes": []
}
```

To build a request that passes, first make a request for a captcha:
```
curl localhost:10212/request_captcha -v --output /tmp/image.png
```

Take note of the captcha key in the header and solve the captcha image. Then make a request:
```
curl -H 'Content-Type: application/json' -d '{"amount": 5, "address": "3c769ea16f38fdc218341c63ff8c1c5c7dcbb4d5d850675e92b09997fd36e8f0"}' -H 'CAPTCHA_KEY: 814861163' -H 'CAPTCHA_VALUE: Es9XE' -H 'what_wallet: my_favorite_wallet' localhost:10212/fund
```

See how this also includes headers to identify the wallet.

## Manually testing Bypassers
Run a local tap with a fake funder and in-memory storage:
```
cargo run -- run -c configs/testing_bypassers.yaml
```

See that a request that should fail (in this case because it's missing the magic headers) succeeds:
```
$ curl -H 'Content-Type: application/json' -X POST -d '{"amount": 5, "address": "3c769ea16f38fdc218341c63ff8c1c5c7dcbb4d5d850675e92b09997fd36e8f0"}' localhost:10212/fund
[]
```

From the server:
```
2022-09-26T21:20:47.126692Z [tokio-runtime-worker] INFO src/endpoints/fund.rs:295 Allowing request from 127.0.0.1 to bypass checks / storage
```

## Manually testing PostgresStorage
First, make sure you've got [postgres](https://www.postgresql.org/) installed and you know your user, database, etc. The tap assumes some reasonable defaults.

Run the tap with postgres storage and a fake funder.
```
cargo run -- run -c configs/testing_postgres.yaml
```
By default this will handle running the database migrations for you.

Submit a request:
```
curl -H 'Content-Type: application/json' -d '{"amount": 100, "address": "3c769ea16f38fdc218341c63ff8c1c5c7dcbb4d5d850675e92b09997fd36e8f0"}' localhost:10212/fund
```

## Testing RedisStorage
First, install redis 6.x (e.g. `brew install redis@6.2`) and run a local redis (with persistent storage), e.g. like this:
```
redis-server --save 60 1 --loglevel warning
```

Then run a localnet:
```
cargo run -p velor -- node run-local-testnet --force-restart --assume-yes
```

Run the tap with redis storage and a fake funder.
```
cargo run -- run -c configs/testing_redis.yaml
```

Submit a request:
```
curl -H 'Content-Type: application/json' -d '{"amount": 100, "address": "3c769ea16f38fdc218341c63ff8c1c5c7dcbb4d5d850675e92b09997fd36e8f0"}' localhost:10212/fund
```

Keep doing so and eventually you'll get rejected. See also that if you induce a 500 in the funder, the counter ultimately does not get incremented, so we don't punish users for issues on our side. I have also verified that the key does indeed get expired next day, so we don't track ratelimit information beyond when we need it. For historical investigation we can look at the application logs instead.

## Manually testing MintFunder
Run a localnet:
```
cargo run -p velor -- node run-local-testnet --force-restart --assume-yes
```

Run the tap with in-memory storage and the mint funder. As above we use just the IpBlocklist checker:
```
cargo run -- run -c configs/testing_mint_funder_local.yaml
```

Submit a request:
```
curl -H 'Content-Type: application/json' -d '{"amount": 100, "address": "3c769ea16f38fdc218341c63ff8c1c5c7dcbb4d5d850675e92b09997fd36e8f1"}' localhost:10212/fund
```

Ensure the legacy endpoint works too:
```
curl -X POST 'http://127.0.0.1:10212/mint?amount=100&address=0xd0f523c9e73e6f3d68c16ae883a9febc616e484c4998a72d8899a1009e5a89d6'
```

Ensure sending errors to `/fund` works as expected:
```
curl -H 'Content-Type: application/json' -d '{"amount": "g", "address": "3c769ea16f38fdc218341c63ff8c1c5c7dcbb4d5d850675e92b09997fd36e8f1"}' localhost:10212/fund
curl -H 'Content-Type: application/json' -d '{"amount": 0, "address": "3c769ea16f38fdc218341c63ff8c1c5c7dcbb4d5d850675e92b09997fd36e8f1"}' localhost:10212/fund
```

Same thing for `/mint`:
```
curl -v -X POST 'http://127.0.0.1:10212/mint'
curl -v -X POST 'http://127.0.0.1:10212/mint?amount=0&address=0xd0f523c9e73e6f3d68c16ae883a9febc616e484c4998a72d88'
```

Hit the API to confirm that the account did indeed get funded.
