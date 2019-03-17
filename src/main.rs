extern crate alsa;

use alsa::seq;
use std::error;
use std::ffi::CString;

fn main() -> Result<(), Box<error::Error>> {
    let seq = seq::Seq::open(None, None, true)?;
    seq.set_client_name(&CString::new("ConnectAll")?)?;

    println!("Clients");
    for from_info in seq::ClientIter::new(&seq){
        for from_port in seq::PortIter::new(&seq, from_info.get_client()){
            if from_port.get_capability().contains(seq::SUBS_READ) && !from_port.get_capability().contains(seq::NO_EXPORT){
                for to_info in seq::ClientIter::new(&seq){
                    for to_port in seq::PortIter::new(&seq, to_info.get_client()){
                        if to_port.get_capability().contains(seq::SUBS_WRITE) && !to_port.get_capability().contains(seq::NO_EXPORT)&& from_port.get_client() != to_port.get_client(){
                            println!("Connect {:?}({:?}) -> {:?}({:?})", from_port, from_port.get_type(), to_port, to_port.get_type ());
                            let subs = seq::PortSubscribe::empty()?;
                            subs.set_sender(seq::Addr{ client: from_port.get_client(), port: from_port.get_port() });
                            subs.set_dest(seq::Addr{ client: to_port.get_client(), port: to_port.get_port() });
                            match seq.subscribe_port(&subs) {
                                Ok(_) => (),
                                Err(err) => 
                                    println!("ERROR: {:?}", err)
                            }
                        }
                    }
                }
            }

        }
    }

    Ok(())
}
