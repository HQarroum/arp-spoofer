extern crate pnet_datalink;

use std::net::{Ipv4Addr, IpAddr};
use pnet_datalink::{NetworkInterface};
use anyhow::{Result, Context};

/// Returns the IPv4 address associayted with the
/// given network interface. If multiple IPv4 addresses
/// are associated with the network interface, the first
/// one is returned.
/// If no IPv4 addresses are associated with the network
/// interface, None is returned.
pub fn get_ipv4_addr_of(interface: &NetworkInterface) -> Option<Ipv4Addr> {
  interface
    .ips
    .iter() 
    .find(|ip| ip.is_ipv4())
    .map(|ip| match ip.ip() {
        IpAddr::V4(ip) => ip,
        _ => unreachable!()
    })
}

/// Returns a `Result` carrying the network interface
/// matching the given name if found, `None` otherwise.
pub fn find_network_interface(name: &String) -> Result<NetworkInterface> {
  // Looking for a network interface matching
  // the given name.
  let network_interface = pnet_datalink::interfaces()
    .into_iter()
    .find(|iface| iface.name == *name)
    .with_context(|| format!("Network interface {} does not exist", *name))?;
  
  // Verifying whether the network interface is up.
  if !network_interface.is_up() {
    return Err(anyhow::anyhow!("Network interface {} is not up", network_interface.name));
  }

  Ok(network_interface)
}