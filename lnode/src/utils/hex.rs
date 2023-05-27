use bitcoin::secp256k1::PublicKey;
use lightning::ln::msgs::NetAddress;
use std::{fmt::Write, net::IpAddr, str::FromStr};

pub fn to_vec(hex: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(hex.len() / 2);

    let mut b = 0;
    for (idx, c) in hex.as_bytes().iter().enumerate() {
        b <<= 4;
        match *c {
            b'A'..=b'F' => b |= c - b'A' + 10,
            b'a'..=b'f' => b |= c - b'a' + 10,
            b'0'..=b'9' => b |= c - b'0',
            _ => return None,
        }
        if (idx & 1) == 1 {
            out.push(b);
            b = 0;
        }
    }

    Some(out)
}

#[inline]
pub fn hex_str(value: &[u8]) -> String {
    let mut res = String::with_capacity(2 * value.len());
    for v in value {
        write!(&mut res, "{:02x}", v).expect("Unable to write");
    }
    res
}

pub fn to_compressed_pubkey(hex: &str) -> Option<PublicKey> {
    if hex.len() != 33 * 2 {
        return None;
    }
    let data = match to_vec(&hex[0..33 * 2]) {
        Some(bytes) => bytes,
        None => return None,
    };
    match PublicKey::from_slice(&data) {
        Ok(pk) => Some(pk),
        Err(_) => None,
    }
}

pub fn str_to_u8(alias: &str) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[..alias.len()].copy_from_slice(alias.as_bytes());
    bytes
}

pub fn ipv_addr(s: &str, port: u16) -> Vec<NetAddress> {
    // define a vector
    let mut addr = Vec::new();
    match IpAddr::from_str(s) {
        Ok(IpAddr::V4(a)) => {
            addr.push(NetAddress::IPv4 {
                addr: a.octets(),
                port,
            });
        }
        Ok(IpAddr::V6(a)) => {
            addr.push(NetAddress::IPv6 {
                addr: a.octets(),
                port,
            });
        }
        Err(_) => {
            println!("ERROR: invalid IPv4 address: {}", s);
        }
    };
    addr
}
