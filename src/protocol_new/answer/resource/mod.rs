mod cname;
mod soa;
mod ipv4;
mod basic;

pub use cname::CnameResource;
pub use soa::SoaResource;
pub use ipv4::Ipv4Resource;
pub use basic::BasicData;

pub trait Resource<T> {
    fn get_name(&self) -> &String;
    fn get_ttl(&self) -> u32;
    fn get_data(&self) -> &T;
}