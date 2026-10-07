pub mod adapters;
pub mod alerts;
pub mod bufferbloat;
pub mod detector;
pub mod dns;
pub mod icmp;
pub mod mtr;
pub mod mtu;
pub mod pinger;
pub mod routes;
pub mod stats;
pub mod types;
pub mod virtual_filter;
pub mod wan;
pub mod wifi;

#[allow(unused_imports)]
pub use adapters::get_network_adapters;
pub use detector::inspect_network;
#[allow(unused_imports)]
pub use routes::get_default_routes;
#[allow(unused_imports)]
pub use types::{AdapterType, NetworkAdapter, NetworkDiagnostic, OperStatus, RouteEntry};
