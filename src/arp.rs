extern crate pnet;
extern crate pnet_datalink;

use crate::net;
use std::{thread, time};
use std::time::Instant;
use std::net::{Ipv4Addr};
use anyhow::{anyhow, Result};

use pnet::packet::ethernet::EtherTypes;
use pnet::packet::ethernet::MutableEthernetPacket;
use pnet::packet::{MutablePacket, Packet};
use pnet_datalink::{Channel, MacAddr, NetworkInterface};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};

/// The minimum size of an Ethernet Frame Header.
const ETH_HEADER_SIZE: usize = 42;

/// The size of an ARP header.
const ARP_HEADER_SIZE: usize = 28;

/// A data structure holding the IP address
/// and the MAC address of an ARP host.
#[derive(Debug, Clone, Copy)]
pub struct ArpHost {
  pub ip: Ipv4Addr,
  pub mac: MacAddr
}

/// A macro used to expand the code required to
/// build a new Ethernet header.
macro_rules! eth {
  ( $buffer:expr, $src:expr, $dst:expr, $ether_type:expr, $payload:expr ) => {
    {
      let mut ethernet_packet = MutableEthernetPacket::new($buffer).unwrap();

      ethernet_packet.set_source($src);
      ethernet_packet.set_destination($dst);
      ethernet_packet.set_ethertype($ether_type);
      ethernet_packet.set_payload($payload);
      ethernet_packet
    }
  };
}

/// A macro used to expand the code required to
/// build a new ARP header.
macro_rules! arp {
  ( $buffer:expr, $arp_type:expr, $source:expr, $target:expr ) => {
    {
      let mut arp_packet = MutableArpPacket::new($buffer).unwrap();

      arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
      arp_packet.set_protocol_type(EtherTypes::Ipv4);
      arp_packet.set_hw_addr_len(6);
      arp_packet.set_proto_addr_len(4);
      arp_packet.set_operation($arp_type);
      arp_packet.set_sender_hw_addr($source.mac);
      arp_packet.set_sender_proto_addr($source.ip);
      arp_packet.set_target_hw_addr($target.mac);
      arp_packet.set_target_proto_addr($target.ip);
      arp_packet
    }
  };
}

pub fn send_arp_reply(
  interface: &NetworkInterface,
  source: ArpHost,
  target: ArpHost
) -> Option<std::io::Result<()>> {
  // Creating a communication channel for communicating at the
  // data link layer (Layer 2). This is the layer on which
  // Ethernet headers are sent and received.
  let (mut tx, mut _rx) = match pnet_datalink::channel(interface, Default::default()) {
    Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
    Ok(_) => return None,
    Err(_e) => return None
  };

  let mut eth_buffer = [0u8; ETH_HEADER_SIZE];
  let mut arp_buffer = [0u8; ARP_HEADER_SIZE];

  // Creating the ARP header.
  let mut arp_packet = arp!(
    &mut arp_buffer,
    ArpOperations::Reply,
    source,
    target
  );

  // Creating the Ethernet header.
  let eth_packet = eth!(
    &mut eth_buffer,
    interface.mac.unwrap(),
    target.mac,
    EtherTypes::Arp,
    arp_packet.packet_mut()
  );

  // Sending the packet over the wire.
  tx.send_to(eth_packet.packet(), None)
}

/// This function will use the given `interface` to send an ARP request
/// asking for the MAC address associated with the given target IP.
/// It returns a `Result` that will contain the target MAC address
/// if successful, an error otherwise.
pub fn resolve_arp_host(
  interface: &NetworkInterface,
  target_ip: Ipv4Addr
) -> Result<ArpHost> {
  // Retrieving the IPv4 address associated
  // with the network interface.
  let source_ip: Ipv4Addr = net::get_ipv4_addr_of(interface).unwrap();
  
  // Creating a communication channel for communicating at the
  // data link layer (Layer 2). This is the layer on which
  // Ethernet headers are sent and received.
  let (mut sender, mut rx) = match pnet_datalink::channel(interface, Default::default()) {
      Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
      Ok(_) => return Err(anyhow!("Unknown channel type")),
      Err(e) => return Err(anyhow!(e))
  };

  let mut eth_buffer = [0u8; ETH_HEADER_SIZE];
  let mut arp_buffer = [0u8; ARP_HEADER_SIZE];

  // Creating the ARP header.
  let mut arp_packet = arp!(
    &mut arp_buffer,
    ArpOperations::Request,
    ArpHost { ip: source_ip, mac: interface.mac.unwrap() },
    ArpHost { ip: target_ip, mac: MacAddr::zero() }
  );

  // Creating the Ethernet header.
  let eth_packet = eth!(
    &mut eth_buffer,
    interface.mac.unwrap(),
    MacAddr::broadcast(),
    EtherTypes::Arp,
    arp_packet.packet_mut()
  );

  // Keeping track of the time before starting the
  // MAC address resolution.
  let start = Instant::now();

  loop {
    // Sending the ARP request over the wire.
    sender
      .send_to(eth_packet.packet(), None)
      .unwrap()
      .unwrap();

    // Synchronously waiting for an Ethernet packet.
    let buf = rx.next().unwrap();
    let arp = ArpPacket::new(&buf[MutableEthernetPacket::minimum_packet_size()..]).unwrap();

    // Verifying that the target and sender of the ARP packet
    // is an expected reply.
    if arp.get_operation() == ArpOperations::Reply
      && arp.get_target_hw_addr() == interface.mac.unwrap()
      && arp.get_sender_proto_addr() == target_ip {
      return Ok(
        ArpHost { ip: target_ip, mac: arp.get_sender_hw_addr() }
      );
    }

    // After 10 seconds, we cancel the MAC address resolution
    // since we didn't have a response from the remote host.
    if start.elapsed().as_secs() >= 10 {
      return Err(
        anyhow!("Could not resolve the MAC address of IP {}", target_ip)
      );
    }

    thread::sleep(time::Duration::from_secs(1));
  }
}