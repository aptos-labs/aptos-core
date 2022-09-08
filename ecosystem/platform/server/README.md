# Community Platform

## Development

### Install Ruby

```shell
rbenv install 3.1.2
```

> If getting any errors try

```shell
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

```shell
brew install postgres
brew services start postgresql
```

### Install ImageMagick

```shell
brew install pkg-config imagemagick vips
```

or

```shell
sudo apt-get install libmagickwand-dev
```

### Start the app

```shell
bin/setup
bin/dev
```

App should run on http://127.0.0.1:3001
