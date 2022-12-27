pub mod resolution {

  /// Represents either a target or a gateway type.
  pub enum HostType {
    Target,
    Gateway
  }

  // Emojis used in the command-line.
  static SEARCH_EMOJI:  console::Emoji<'_, '_> = console::Emoji("🔍  ", "");
  static SUCCESS_EMOJI: console::Emoji<'_, '_> = console::Emoji("✅  ", "");
  
  /// Logs that the ARP resolution is in progress
  /// for a given host.
  pub fn search(host_type: HostType, ip: std::net::Ipv4Addr) {
    let type_name = if matches!(host_type, HostType::Target) { String::from("target") } else { String::from("gateway") };
    println!(
      "{} {}Resolving the MAC address of the {} ({}) ...",
      console::style("[ARP Resolution]").bold().dim(),
      SEARCH_EMOJI,
      type_name,
      ip
    );
  }

  /// Logs that the ARP resolution has been successful
  /// and that a host MAC address has been found.
  pub fn found(host: crate::arp::ArpHost) {
    println!(
      "{} {}Found host at {}",
      console::style("[ARP Resolution]").bold().dim(),
      SUCCESS_EMOJI,
      host.mac
    );
  }
}

pub mod poisoning {

  /// Represents the states of an ARP cache poisoning process.
  pub enum State {
    InProgress,
    CleanUp,
    Stopped
  }

  // Emojis used in the command-line.
  static POISON_EMOJI:  console::Emoji<'_, '_> = console::Emoji("👾  ", "");
  static CLEANUP_EMOJI: console::Emoji<'_, '_> = console::Emoji("🧹  ", "");

  /// Displays a log 
  pub fn state(state: State) {
    match state {
      State::InProgress => {
        println!(
          "{} {}Poisoning the target and gateway cache ... (Ctrl-c to interrupt)",
          console::style("[ARP Poisoning]").bold().dim(),
          POISON_EMOJI
        );
      },
      State::CleanUp => {
        println!(
          "{} {}Restoring the target and gateway ARP caches ...",
          console::style("[ARP Poisoning]").bold().dim(),
          CLEANUP_EMOJI
        );
      },
      State::Stopped => {
        println!(
          "{} {}ARPCache poisoned has stopped, remote caches have been restored.",
          console::style("[ARP Poisoning]").bold().dim(),
          CLEANUP_EMOJI
        );
      }
    }
    
  }
}