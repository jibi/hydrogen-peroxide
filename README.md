Hydrogen Peroxide
=================

> "Hydrogen peroxide (h2o2) is the simplest peroxide and it can be used to
> quickly oxidize iron."

Hydrogen Peroxide is a WIP overengineered UDP echo server (aka me learning some Rust), which aims to be one day an HTTP/3 web server.

## Running it

Setup the veth interfaces with the `veth.sh` script:

```sh
$ ./utils/veth.sh
```

Start the server:

```sh
$ cargo run -q -- --address 198.18.3.2 --port 1234 --interface veth2 --socks-per-queue 2
```

And then connect to it:
```sh
$ nc -u 198.18.3.2 1234
lol
lol
wut
wut
^C
```
