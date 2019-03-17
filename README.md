# ALSA Sequencer Connection Keeper

`aseqkeeper` watches for the connecitons on the ALSA sequencer, and it tries
to keep them connected.

The two most common situations where this is needed:

1. is when a device is disconnected, and then connected again
2. when the system reboots

If the user manually makes an ALSA sequence port connection (`aconnect` or
`qjackctl`), it notices and stores it into a file. The same if the user
manually disconnects the sequencer ports.

But if it is the system because of an `exit` (in ALSA terms), it keeps this
memory of a previous connection. Next time the same device connects (checked
by name), `aseqkeeper` performs the connection for you.

Also if the daemon stops (for example shutdown of the computer), it keeps this
file and on next run (system boot) it uses this knowledge to set up again the
ports, and check for more connections.

## Compiling

```sh
cargo build --release
```

## Running

```sh
target/release/aseqrunner
```

## Installation

You can directly system install it with:

```sh
make install
```

It will compile `aseqkeeper` and install a systemd service file to keep it
running. It will ask for your passsword to be able to install the files.
It will run as the installing user.

It will not enable it by default, so on reboots it will not automatically start.
To enable it:

```sh
sudo systemctl aseqkeeper
```

And to run it (will not run at enable, and can be run not enabling it):

```sh
sudo service aseqkeeper start
```
