pub mod connection;
pub mod migrations;
pub mod models;
pub mod repository;
pub mod worker;

#[allow(unused_imports)]
pub use connection::open_optimized_connection;
#[allow(unused_imports)]
pub use models::{HostRecord, LatencySampleRecord, NetworkEventRecord};
#[allow(unused_imports)]
pub use repository::{
    cleanup_old_records, get_all_hosts, get_host_by_address, get_recent_samples_for_host,
    upsert_host,
};
pub use worker::{DatabaseHandle, start_database_worker};
