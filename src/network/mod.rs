pub mod adapters;
pub mod detector;
pub mod routes;
pub mod types;
pub mod virtual_filter;

#[allow(unused_imports)]
pub use adapters::get_network_adapters;
pub use detector::inspect_network;
#[allow(unused_imports)]
pub use routes::get_default_routes;
#[allow(unused_imports)]
pub use types::{AdapterType, NetworkAdapter, NetworkDiagnostic, OperStatus, RouteEntry};
