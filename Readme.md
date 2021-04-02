# IO Test

Fetches the 10 most popular packages from NPM.

Splits them into 3 chunks, compresses them and serves them via NGINX.

## Build & run the server

```
$ make build-server
$ make run-server
```

## Build & run the client

Run these commands in another shell

```
$ make build-client
$ make run-client
```

## Results

With two local containers on a fast NVMe drive.

CHUNK_COUNT=1

```
real    0m0.303s
user    0m0.040s
sys     0m0.124s
```

CHUNK_COUNT=3

```
real    0m0.192s
user    0m0.033s
sys     0m0.121s
```

CHUNK_COUNT=5

```
real    0m0.183s
user    0m0.040s
sys     0m0.142s
```
