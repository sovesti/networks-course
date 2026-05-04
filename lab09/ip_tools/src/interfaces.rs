use std::{io, net::SocketAddr};

use getifaddrs::{Interface, getifaddrs};

pub fn show_wireless_interfaces() -> io::Result<()> {
    wireless_interfaces()?.for_each(show_interface);
    Ok(())
}

pub fn my_address(port: u16) -> io::Result<SocketAddr> {
    wireless_interfaces()?
        .map(|interface| interface.address)
        .max_by_key(|address| address.netmask())
        .map(|address| address.ip_addr())
        .flatten()
        .map(|address| SocketAddr::new(address, port))
        .ok_or_else(|| io::Error::other("No IPv4 address found"))
}

pub fn wireless_interfaces() -> io::Result<impl Iterator<Item = Interface>> {
    Ok(getifaddrs()?
        .filter(is_ipv4)
        .filter(has_mask)
        .filter(is_wireless))
}

fn show_interface(interface: Interface) {
    println!("Interface: {}", interface.name);
    println!(
        "  IP Address: {}",
        to_string_or_unknown(interface.address.ip_addr())
    );
    println!(
        "  Netmask: {}",
        to_string_or_unknown(interface.address.netmask())
    );
    println!();
}

fn to_string_or_unknown<T: ToString>(value: Option<T>) -> String {
    value
        .map(|addr| addr.to_string())
        .unwrap_or("unknown".to_string())
}

fn is_ipv4(interface: &Interface) -> bool {
    interface
        .address
        .ip_addr()
        .is_some_and(|addr| addr.is_ipv4())
}

fn has_mask(interface: &Interface) -> bool {
    interface.address.netmask().is_some()
}

fn is_wireless(interface: &Interface) -> bool {
    interface.name.starts_with("wireless")
}
