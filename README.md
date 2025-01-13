# meshpit

meshpit is an experimental, small command line tool to send and receive UDP packets among peers in a p2p network with the help of [p2panda](https://p2panda.org).

meshpit automatically discovers peers who are interested in the same "topic" - in your local network and the internet - connects to them directly, syncs previously sent data and establishes a gossip overlay to broadcast new data.

meshpit can be combined with any other software which sends and receives UDP packets and enables it to be part of a p2p network with local-first and eventual consistency guarantees.

## Usage

```bash
# Start meshpit node
meshpit -l DEBUG

# Write messages to UDP server
nc -u <server addr> <server port>

# Read messages to UDP client
nc -lzu -p <client port>
```

## License

[`MIT`](LICENSE)
