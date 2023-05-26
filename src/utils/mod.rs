use self::disk::FilesystemLogger;
use crate::types::NetworkGraph;
use bitcoin::{BlockHash, Network};
use std::{fs::File, io::BufReader, path::Path, sync::Arc};

pub mod convert;
pub mod disk;
pub mod hex;
pub mod sweep;

pub fn read_network(
    path: &Path,
    genesis_hash: BlockHash,
    logger: Arc<FilesystemLogger>,
) -> NetworkGraph {
    if let Ok(file) = File::open(path) {
        if let Ok(graph) = NetworkGraph::read(&mut BufReader::new(file), logger.clone()) {
            return graph;
        }
    }
    NetworkGraph::new(Network::Regtest, logger)
}
