# meshpit

meshpit is an experimental, small command line tool to send and receive UDP packets among peers in a p2p network with the help of [p2panda](https://p2panda.org).

meshpit automatically discovers peers who are interested in the same "topic" - in your local network and the internet - connects to them directly, syncs previously sent data and establishes a gossip overlay to broadcast new data.

meshpit can be combined with any other software which sends and receives UDP packets and enables it to be part of a p2p network with local-first and eventual consistency guarantees.

## Installation

Check out the [Releases](https://github.com/adzialocha/meshpit/releases/) section where we publish binaries for Linux, RaspberryPi, MacOS and Windows or read [how you can compile](#development) `meshpit` yourself.

## Usage

```
Usage: meshpit [OPTIONS]

Options:
  -t, --topic <STRING>
          Define a short text-string which will be automatically hashed and
          used as a "topic".

          If peers are configured to the same topic, they will find each other
          automatically, connect and sync data with each other.

  -b, --bootstrap <PUBLIC_KEY>
          Mention the public key of another peer to use it as a "bootstrap
          node" for discovery over the internet.

          If no value is given here, meshpit can only find other peers in your
          local area network.

  -s, --udp-server <ADDR:PORT>
          UDP server address and port. Send your data to this address, it will
          automatically be forwarded to all peers in the network who are
          subscribed to the same topic.

          Meshpit will use localhost and a random port by default.

  -c, --udp-client <ADDR:PORT>
          UDP client address and port (default is 49494). Meshpit will
          automatically forward all received data from other peers to this
          address

  -l, --log-level <LEVEL>
          Set log verbosity. Use this for learning more about how your node
          behaves or for debugging.

          Possible log levels are: ERROR, WARN, INFO, DEBUG, TRACE. They are
          scoped to "meshpit" by default.

          If you want to adjust the scope for deeper inspection use a filter
          value, for example "=TRACE" for logging _everything_ or
          "meshpit=INFO,p2panda_net=DEBUG" etc.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Example

```bash
# Start meshpit node
meshpit -l DEBUG

# Write messages to UDP server
nc -u <server address> <server port>

# Read messages to UDP client
nc -lzu -p <client port>
```

## Development

```bash
# Run with DEBUG logging enabled
cargo run -- --log-level DEBUG

# Compile release binary
cargo build --release
```

## License

[`MIT`](LICENSE)
