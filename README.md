# swelfi

## Dev

```bash
cargo watch -c -w src -x run
```

## TODOs

* [x] Draw UI sketch
* [x] Use WGPU backend
* [x] Build UI outline with dummy data
* Fetch wireless interfaces (https://www.baeldung.com/linux/connect-network-cli)
    * [x] auto-set default
    * [x] make selectable
    * [x] iwconfig parsing
* [x] implement enable/disable wifi
* [x] Create watch-workflow
* [x] fetch wifis
    * [x]show in a list
* [x] fetch wifis asynchronously (thread+channel) and show loading indicator
* [x] highlight connected wifi
* [x] add refresh networks button
* [x] connect/disconnect to/from wifi
* correctly identify currently connected wifi (iw dev $interface info)
* implement disconnect from a network
* implement connect to a network
* implement different schemes (WEP/WPA/WPA2)
* implement "forget"
* Custom style (check Settings from examples)

