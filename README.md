Daniel's blockchain demo implemenation using rust

refer to https://hackernoon.com/learn-blockchains-by-building-one-117428612f46


# how to use

## start a server
```sh
cargo run
```

## access api via httpie

display chain

```sh
http GET  http://localhost:8000/chain
http POST http://localhost:8000/transactions/new sender=a recipient=b amount:=3.0
http GET  http://localhost:8000/mine
```

create a new transaction

```sh
http GET  http://localhost:8000/mine
```

mine a new block and add all pending transactions

```sh
http GET  http://localhost:8000/mine
```
