extern crate alsa;
extern crate serde;

use std::io::Write;
use alsa::seq;
use std::error::Error;
use std::ffi::CString;
use clap::{Arg, App};
use log::{info, error, debug};
use std::collections::hash_map::HashMap;
use serde::{Deserialize, Serialize};
use std::fs::File;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
struct Connection{
    sender: String,
    dest: String
}



fn parse_cmdline() -> clap::ArgMatches<'static> {
    App::new("Alsa Seq connection Keeper")
       .version("0.1.0")
       .author("David Moreno <dmoreno@coralbits.com>")
       .about("Keeps the connections between reboots and disconnects.

When connecting and disconnecting MIDI gear the connections are lost.

This daemon ensures that if you manually connected something, \
on next reboot or connection it will be connected. If you disconnect \
manually it also stays disconnected.

Set logging level with RUST_LOG=\"debug\" envvar.")
       .arg(
           Arg::with_name("all")
               .short("a")
               .long("all")
               .help("Autoconnects all inputs to all inputs. Does not check previous state.")
           )
       .arg(
           Arg::with_name("disconnect")
               .short("dz")
               .long("disconnect|zero")
               .help("Disconnects everything to start with.")
           )
       .get_matches()
}

fn connect_all(seq: &seq::Seq) -> Result<(), Box<Error>>{
    info!("Connectiong all inputs to outputs");
    for from_info in seq::ClientIter::new(&seq){
        for from_port in seq::PortIter::new(&seq, from_info.get_client()){
            if from_port.get_capability().contains(seq::SUBS_READ) && !from_port.get_capability().contains(seq::NO_EXPORT){
                for to_info in seq::ClientIter::new(&seq){
                    for to_port in seq::PortIter::new(&seq, to_info.get_client()){
                        if to_port.get_capability().contains(seq::SUBS_WRITE) && !to_port.get_capability().contains(seq::NO_EXPORT) && from_port.get_client() != to_port.get_client(){
                            let subs = seq::PortSubscribe::empty()?;
                            subs.set_sender(seq::Addr{ client: from_port.get_client(), port: from_port.get_port() });
                            subs.set_dest(seq::Addr{ client: to_port.get_client(), port: to_port.get_port() });
                            match seq.subscribe_port(&subs) {
                                Ok(_) => {
                                    info!("Connected {:?}({:?}) -> {:?}({:?})", from_port, from_port.get_type(), to_port, to_port.get_type ());
                                },
                                Err(err) =>
                                    error!("Connected {:?}({:?}) -> {:?}({:?}): {:?}",
                                        from_port, from_port.get_type(), to_port, to_port.get_type (), err)
                            }
                        }
                    }
                }
            }

        }
    }

    Ok(())
}

fn setup_alsaseq() -> Result<seq::Seq, Box<Error>>{
    let seq = seq::Seq::open(None, Some(alsa::Direction::Capture), true)?;
    seq.set_client_name(&CString::new("ASeqKeep")?)?;

    let mut dinfo = seq::PortInfo::empty()?;
    dinfo.set_capability(seq::WRITE);
    // dinfo.set_type(seq::MIDI_GENERIC | seq::APPLICATION);
    // dinfo.set_capability(seq::WRITE | seq::SUBS_WRITE);
    // dinfo.set_type(seq::MIDI_GENERIC | seq::APPLICATION);
    dinfo.set_name(&CString::new("Input")?);
    seq.create_port(&dinfo)?;

    // Connect to announce
    connect(
            &seq,
            &seq::Addr{ client: 0, port: 1 },
            &seq::Addr{ client: seq.client_id()?, port: dinfo.get_port() }
    )?;

    Ok(seq)
}

fn connect(seq: &seq::Seq, sender: &seq::Addr, dest: &seq::Addr) -> Result<(), Box<Error>> {
    let subs = seq::PortSubscribe::empty()?;
    subs.set_sender(*sender);
    subs.set_dest(*dest);
    seq.subscribe_port(&subs)?;

    debug!("Connected {:?} -> {:?}", sender, dest);
    Ok(())
}

fn get_port_name(seq: &seq::Seq, source: seq::Addr) -> Result<String, Box<Error>>{
    let client_info = match seq.get_any_client_info(source.client) {
        Ok(info) => info,
        _ => {
            return Ok(format!("{}:{}", source.client, source.port));
        }
    };
    // Not in cache, calculate
    let origin = format!("{}:{}",
        client_info.get_name()?,
        seq.get_any_port_info(source)?.get_name()?,
    );

    Ok(origin)
}

fn connect_by_name(seq: &seq::Seq, conn: &Connection, ids: &HashMap<String, seq::Addr>) -> Result<(), Box<Error>>{
    connect(
        seq,
        ids.get(&conn.sender).ok_or("Sender not found")?,
        ids.get(&conn.dest).ok_or("Dest not found")?
    )?;

    info!("Connected {:?}", conn);
    Ok(())
}

fn try_connections(seq: &seq::Seq, port: &str, connections: &Vec<Connection>, ids: &HashMap<String, seq::Addr>) -> Result<(), Box<Error>>{
    for conn in connections {
        if port == conn.sender || port == conn.dest {
            connect_by_name(&seq, &conn, &ids)?;
        }
    }
    Ok(())
}

fn try_all_connections(seq: &seq::Seq, connections: &Vec<Connection>, ids: &HashMap<String, seq::Addr>) -> Result<(), Box<Error>>{
    for conn in connections {
        match connect_by_name(&seq, &conn, &ids) {
            Ok(()) => (),
            err => {
                debug!("Connection not available yet {:?}: {:?}. Will connect when both sides are ready.", conn, err);
            }
        }
    }
    Ok(())
}

fn load_connections() -> Result<Vec<Connection>, Box<Error>> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("aseqkeeper").unwrap();
    let path = xdg_dirs.place_config_file("connections.json")?;
    let file = match File::open(&path) {
        Ok(f) => f,
        _ => {
            info!("Starting with empty config file.");
            return Ok(Vec::new())
        }
    };

    let connections = serde_json::from_reader(file)?;
    Ok(connections)
}

fn store_connections(connections: &mut Vec<Connection>) -> Result<(), Box<Error>>{
    connections.sort();
    connections.dedup();

    let json =  serde_json::to_string_pretty(connections)?;

    let xdg_dirs = xdg::BaseDirectories::with_prefix("aseqkeeper").unwrap();
    let path = xdg_dirs.place_config_file("connections.json")?;
    let display = path.display();

    let mut file = File::create(&path)?;

    // Write the `LOREM_IPSUM` string to `file`, returns `io::Result<()>`
    match file.write_all(&json.as_bytes()) {
        Err(why) => {
            panic!("couldn't write to {}: {}", display,
                                               why.description())
        },
        Ok(_) => debug!("Successfully wrote connections to {}", display),
    }

    Ok(())
}

fn get_port_ids(seq: &seq::Seq) -> Result<HashMap<String, seq::Addr>, Box<Error>> {
    let mut ids = HashMap::new();

    for info in seq::ClientIter::new(&seq){
        for port in seq::PortIter::new(&seq, info.get_client()){
            if !port.get_capability().contains(seq::NO_EXPORT) {
                let addr = seq::Addr{ client: port.get_client(), port: port.get_port() };
                ids.insert(get_port_name(seq, addr)?, addr);
            }
        }
    }

    Ok(ids)
}

fn is_port_no_export(seq: &seq::Seq, addr: &seq::Addr) -> bool {
    match seq.get_any_port_info(*addr) {
        Ok(port) => {
            port.get_capability().contains(seq::NO_EXPORT)
        }
        _ => {
            false
        }
    }
}

fn main() -> Result<(), Box<Error>> {
    flexi_logger::Logger::with_env_or_str("debug")
                .start()
                .unwrap();
    let options = parse_cmdline();

    let seq = setup_alsaseq()?;

    if options.occurrences_of("all") > 0 {
        connect_all(&seq)?;
    }

    // These are the connections I want to keep. This is stored between runs.
    let mut connections: Vec<Connection> = load_connections()?;
    // This is the map from names to the addresses. This is run dependant.
    let mut ids: HashMap<String, seq::Addr> = get_port_ids(&seq)?;
    debug!("Current subscriptions are {:?}", connections);
    debug!("Current ids are {:?}", ids);
    try_all_connections(&seq, &connections, &ids)?;

    let mut input = seq.input();
    use alsa::PollDescriptors;
    let seqp = (&seq, Some(alsa::Direction::Capture));
    let mut fds = Vec::<libc::pollfd>::new();
    fds.append(&mut seqp.get()?);

    info!("Waiting for connection changes...");
    loop {
        alsa::poll::poll(&mut fds, 1000)?;
        while input.event_input_pending(true)? != 0 {
            let ev = input.event_input()?;
            match ev.get_type() {
                seq::EventType::PortSubscribed => {
                    let conn: seq::Connect = ev.get_data().ok_or("Expected connection")?;

                    if !is_port_no_export(&seq, &conn.sender) && !is_port_no_export(&seq, &conn.dest){
                        let sender = get_port_name(&seq, conn.sender)?;
                        let dest = get_port_name(&seq, conn.dest)?;
                        ids.insert(sender.clone(), conn.sender.clone());
                        ids.insert(dest.clone(), conn.dest.clone());

                        if None == connections.iter().position(|conn| (conn.sender == sender && conn.dest == dest)) {
                            let conn = Connection{sender: sender, dest: dest};
                            info!("New connection learnt: {:?}", &conn);
                            connections.push(conn);
                            // debug!("Current subscriptions are {:?}", connections);

                            store_connections(&mut connections)?;
                        }
                    }
                },
                seq::EventType::PortUnsubscribed => {
                    let conn: seq::Connect = ev.get_data().ok_or("Expected connection")?;
                    let sender = get_port_name(&seq, conn.sender)?;
                    let dest = get_port_name(&seq, conn.dest)?;
                    let conn = Connection{sender: sender.clone(), dest: dest.clone()};

                    info!("New disconnection learnt: {:?}", &conn);
                    // info!("New port unsubscribed: {} -> {}", &sender, &dest);
                    if let Some(idx) = connections.iter().position(|conn| (conn.sender == sender && conn.dest == dest)) {
                        // info!("Found subscription at {}", idx);
                        connections.remove(idx);
                    }
                    // debug!("Current subscriptions are {:?}", connections);
                    store_connections(&mut connections)?;
                },
                seq::EventType::PortStart => {
                    let addr: seq::Addr = ev.get_data().ok_or("Expected address")?;
                    let portname = get_port_name(&seq, addr)?;
                    // info!("Port start. Check if should connect something to {}", portname);
                    // debug!("Current subscriptions are {:?}", connections);
                    ids.insert(portname.clone(), addr.clone());

                    match try_connections(&seq, &portname, &connections, &ids) {
                        Ok(()) => (),
                        error => error!("{:?}", error)
                    };
                },
                _other => {
                    // debug!("Ingnoring event: {:?}", ev)
                }
            }
        }
    }
}
