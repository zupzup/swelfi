# swelfi

## Dev

```bash
cargo watch -c -w src -x run
```

## TODOs

* [x] Draw UI sketch
* [x] Use WGPU backend
* Build UI outline with dummy data
* Fetch wireless interfaces (https://www.baeldung.com/linux/connect-network-cli)
    * auto-set default
    * [x] make selectable
    * [x] iwconfig parsing
* [x] implement enable/disable wifi
* [x] Create watch-workflow
* fetch wifis
    * show in a list
    * highlight connected wifi
* connect/disconnect to/from wifi
* implement different schemes (WEP/WPA/WPA2)
* implement "forget"
* implement auto-connect on startup (necessary?)
* Custom style (check Settings from examples)
* System Tray / Sway status bar (on click, open)
* Start with `sudo -E x`
* Consider using libnl instead of iw/iwlist etc. ([libnl](http://www.infradead.org/~tgr/libnl/)
    * [libnl impl in python](https://github.com/Robpol86/libnl/blob/master/example_scan_access_points.py)
