# Community Platform

## Development

### Install Ruby
```
rbenv install 3.1.2
```
> If getting any errors try
```
brew install openssl
```

it should add
```
#openssl
export LDFLAGS="-L/opt/homebrew/opt/openssl@3/lib"
export CPPFLAGS="-I/opt/homebrew/opt/openssl@3/include"
```
to your ~/.zshrc as well, and then `rbenv install 3.1.2` should work

### Install postgres
```
brew install postgres
brew services start postgresql
```

### Start the app

```
$ bin/setup
$ bin/dev
```

App should run on http://127.0.0.1:3001