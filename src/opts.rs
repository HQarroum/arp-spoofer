use clap::{Command, arg, ArgMatches};

/// Returns an argument matcher for the application
/// that will parse the command-line arguments.
pub fn get_matches() -> ArgMatches {
  Command::new("arp-spoofer")
    .version("1.0.0")
    .author("Halim Qarroum <hqm.post@gmail.com>")
    .about("Executes an ARP cache poisonning attack against a remote host.")
    .arg(
      arg!(--interface <VALUE>)
        .required(true)
        .short('i')
        .help("The network interface to use to send ARP packets")
    )
    .arg(
      arg!(--target <VALUE>)
        .required(true)
        .short('t')
        .help("The IP address of the target machine to spoof the ARP cache of")
    )
    .arg(
      arg!(--gateway <VALUE>)
        .required(true)
        .short('g')
        .help("The IP address of the default gateway")
    )
    .get_matches()
}