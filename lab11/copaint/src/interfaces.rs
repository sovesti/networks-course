use std::{io, net::IpAddr};

use getifaddrs::{Address, Interface, getifaddrs};

pub fn my_ip() -> io::Result<IpAddr> {
    my_address()?
        .and_then(|address| address.ip_addr())
        .ok_or_else(|| io::Error::other("No IPv4 address found"))
}

fn my_address() -> io::Result<Option<Address>> {
    Ok(wireless_interfaces()?
        .map(|interface| interface.address)
        .max_by_key(|address| address.netmask()))
}

pub fn wireless_interfaces() -> io::Result<impl Iterator<Item = Interface>> {
    Ok(getifaddrs()?
        .filter(is_ipv4)
        .filter(has_mask)
        .filter(is_wireless))
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
