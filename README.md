# Swapi

[![Build Status](https://travis-ci.org/mmacedoeu/swapi.svg?branch=master)](https://travis-ci.org/mmacedoeu/swapi)
[![Language (Rust)](https://img.shields.io/badge/powered_by-Rust-blue.svg)](http://www.rust-lang.org/)

Swapi is a CRSD Create Read Search Delete show case for the following features:

## Features

* Async/Sync [actors](https://github.com/actix/actix).
* Actor communication in a local/thread context.
* Uses [Futures](https://crates.io/crates/futures) for asynchronous message handling.
* HTTP1/HTTP2 support ([actix-web](https://github.com/actix/actix-web))
* Typed messages (No `Any` type).
* Patched Mentat Datomic's embedded [database](https://github.com/mmacedoeu/mentat)
* Multi-producer multi-consumer [channels](https://github.com/crossbeam-rs/crossbeam-channel)
* Json speculative parsing with [Pikkr](https://github.com/pikkr/pikkr) which is based on [Y. Li, N. R. Katsipoulakis, B. Chandramouli, J. Goldstein, and D. Kossmann. Mison: a fast JSON parser for data analytics. In *VLDB*, 2017](http://www.vldb.org/pvldb/vol10/p1118-li.pdf). Benchmark Result:
![](https://raw.githubusercontent.com/pikkr/pikkr/master/img/benchmark.png)
* REST interface
* CORS enabled
* Http client requests with actors based [Scatter-Gather](http://www.enterpriseintegrationpatterns.com/patterns/messaging/BroadcastAggregate.html) Pattern
* Client requests with Least Recent Used frontal cache [LRU Time Cache](https://github.com/maidsafe/lru_time_cache)

## Not Featured

* Json result paging
* Authentication
* database replication
* Json error handling

## Install

To compile and install you need to first install Rust [compiler](https://www.rust-lang.org/en-US/install.html)

`curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly-2018-05-10`

Compile for release

`cargo build --release`

## Platform support

Should compile and work on all rust compiler supported [plataforms](https://forge.rust-lang.org/platform-support.html) but only tested for 64bit linux

### Docker support

Ongoing testing for Docker based build and CI with Dockerfile present and almost ready

### Cloud support

Support for heroku based Cloud services with provided Procfile, should work on any cloud provider based on Docker and with minor manifest files on any Cloud provider

## Usage

Display help:

`./target/release/swapi --help`

```
Star Wars Api

USAGE:
    swapi [OPTIONS] [ARGS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -e <expire>             Time in seconds for cache expiration. default 7 days. [default: 604800]
    -l <LOG_PATTERN>        Sets a custom logging
    -p <PORT>               Api tcp listener port

ARGS:
    <IP>    Specify the hostname portion of the REST API server, IP should be an interface's IP address, or all (all
            interfaces) or local. [default: local]
    <db>    Specify the base database storage path.
```

Run with full trace:

`./target/release/swapi -l trace`

Run with no logging:

`./target/release/swapi -l warn,actix_web::middleware::logger=warn`

### Testing

## Manual Testing

Manual Testing is done with your preferred http client cli like [curl](https://github.com/curl/curl), [Httpie](https://github.com/jakubroztocil/httpie), [http-prompt](https://github.com/eliangcs/http-prompt), or any http test tool like [postman](https://www.getpostman.com/)

### Read All

`http :8080/sw`

### Search by name

`http :8080/sw/?search=Tato`

### Get by id

`http :8080/sw/<uuid>` like `http :8080/sw/0c298919-76f0-42d7-868b-0a0d70d14903`

### Delete

`http DELETE :8080/sw/<uuid>` like `http DELETE :8080/sw/0c298919-76f0-42d7-868b-0a0d70d14903`

### Create

`http POST :8080/sw name=Hoth climate=frozen terrain='tundra, ice caves, mountain ranges'`

## Load testing

Using [Vegeta](https://github.com/tsenart/vegeta) with 8 cores during 10 seconds and 5000 requests per second

### For read all

`echo "GET http://localhost:8080/sw" | vegeta attack -duration=10s -rate=5000 | tee results.bin | vegeta report -reporter=plot > plot.html`

![alt text](https://github.com/mmacedoeu/swapi/raw/master/vegeta-plot.png "Read All Latency plot")

`echo "GET http://localhost:8080/sw" | vegeta attack -duration=10s -rate=5000 | tee results.bin | vegeta report`

```
Requests      [total, rate]            50000, 5000.10
Duration      [total, attack, wait]    10.000322134s, 9.999799929s, 522.205µs
Latencies     [mean, 50, 95, 99, max]  583.374µs, 457.817µs, 1.038971ms, 3.024058ms, 15.524554ms
Bytes In      [total, mean]            19950000, 399.00
Bytes Out     [total, mean]            0, 0.00
Success       [ratio]                  100.00%
Status Codes  [code:count]             200:50000
Error Set:
```

### Get by id

`echo "GET http://localhost:8080/sw/de09bbe9-a993-4ede-89aa-6713e8fc2976" | vegeta attack -duration=10s -rate=5000 | tee results.bin | vegeta report`

```
Requests      [total, rate]            50000, 5000.10
Duration      [total, attack, wait]    10.00022721s, 9.999799894s, 427.316µs
Latencies     [mean, 50, 95, 99, max]  477.733µs, 396.327µs, 785.652µs, 2.179568ms, 11.341423ms
Bytes In      [total, mean]            6850000, 137.00
Bytes Out     [total, mean]            0, 0.00
Success       [ratio]                  100.00%
Status Codes  [code:count]             200:50000  
Error Set:
```

### Search by name

`echo "GET http://127.0.0.1:8080/sw/?search=Tato" | vegeta attack -duration=10s -rate=5000 | tee results.bin | vegeta report`

```
Requests      [total, rate]            50000, 5000.10
Duration      [total, attack, wait]    10.000370959s, 9.999799882s, 571.077µs
Latencies     [mean, 50, 95, 99, max]  727.904µs, 482.15µs, 1.551523ms, 4.861028ms, 22.019851ms
Bytes In      [total, mean]            5650000, 113.00
Bytes Out     [total, mean]            0, 0.00
Success       [ratio]                  100.00%
Status Codes  [code:count]             200:50000
Error Set:
```

With histogram:

`echo "GET http://127.0.0.1:8080/sw/?search=Tato" | vegeta attack -duration=10s -rate=5000 | tee results.bin | vegeta report -reporter='hist[0,1ms,2ms,3ms,4ms]'`

```
Bucket         #      %       Histogram
[0s,    1ms]   46790  93.58%  ######################################################################
[1ms,   2ms]   1773   3.55%   ##
[2ms,   3ms]   624    1.25%
[3ms,   4ms]   297    0.59%
[4ms,   +Inf]  516    1.03%
```
