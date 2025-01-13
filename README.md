# meshpit

> meshpit was developed as an introduction into p2p and local-first networking and artistic tool for the "Radical Networks" course, January 25 at NYU Berlin organized by [Sarah Grant](https://github.com/chootka).

meshpit is an experimental, small command line tool turning any program which can send and receive UDP packets to a p2p application with the help of [p2panda](https://p2panda.org).

meshpit automatically discovers peers who are interested in the same "topic" - in your local network and the internet - connects to them directly, syncs previously sent data and establishes a gossip overlay to broadcast new messages.

meshpit can be combined with any other software and enables it to be part of a p2p network with local-first and eventual consistency guarantees.

## Installation

Check out the [Releases](https://github.com/adzialocha/meshpit/releases/) section with published binaries for Linux, RaspberryPi, MacOS and Windows or read [how you can compile](#development) meshpit yourself.

## Usage

```
Usage: meshpit [OPTIONS]

Options:
  -t, --topic <STRING>
          Define a short text-string which will be automatically hashed and
          used as a "topic".

          If peers are configured to the same topic, they will find each other
          automatically, connect and sync data.

  -b, --bootstrap <PUBLIC_KEY>
          Mention the public key of another peer to use it as a "bootstrap
          node" for discovery over the internet.

          If no value is given here, meshpit can only find other peers in your
          local area network.

  -s, --udp-server <ADDR:PORT>
          UDP server address and port. Send your data to this address, it will
          automatically be forwarded to all peers in the network who are
          subscribed to the same topic.

          meshpit will use localhost and a random port by default. Set it to
          0.0.0.0 (bind to all network interfaces) if you want the UDP server
          to be accessible for other devices on the network.

  -c, --udp-client <ADDR:PORT>
          UDP client address and port (default is 49494). meshpit will
          automatically forward all received data from other peers to this
          address

  -n, --no-sync
          Disable sync for this node.

          Nodes without sync will not "catch up" on past data and only receive
          new messages via the broadcast gossip overlay.

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
# Start meshpit node with DEBUG logging
meshpit -l DEBUG

# Start meshpit node with alternative topic and bootstrap node
meshpit -t hello -b 2a97ed5278e22002d0a0611bb9f77eb5f6ebc50b5fb6975e62f06bcf602d6037

# UDP server binding to all network interfaces and custom port. It is reachable
# now for other devices on the network
meshpit -s 0.0.0.0:41414

# Write messages to UDP server with "netcat"
nc -u <server address> <server port>

# Listen for messages to UDP client (tested on Linux)
nc -lzu -p <client port>

# Listen for messages to UDP client (tested on MacOS)
nc -kul <client port>
```

## Development

Make sure you have the [Rust development environment](https://www.rust-lang.org/learn/get-started) installed on your machine.

```bash
# Run with DEBUG logging enabled
cargo run -- --log-level DEBUG

# Compile release binary
cargo build --release
```

## License

[`MIT`](LICENSE)
