pub mod cli;
pub mod keys {
    include!(concat!(env!("OUT_DIR"), "/agent.rs"));
}
