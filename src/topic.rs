use std::collections::{HashMap, HashSet};
use std::hash::Hash as StdHash;
use std::sync::Arc;

use async_trait::async_trait;
use p2panda_core::{Hash, PublicKey};
use p2panda_net::TopicId;
use p2panda_sync::log_sync::TopicLogMap;
use p2panda_sync::TopicQuery;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Every meshpit node writes to one single log which is identified by the node's public key and
/// this default log id.
const DEFAULT_LOG_ID: LogId = 1;

pub type LogId = u64;

/// Nodes converge around topics they are interested in to exchange data.
///
/// In meshpit a topic is a simple string we convert to a BLAKE3 hash. If two peers are interested
/// in the same topic, they'll find each other.
#[derive(Clone, Debug, PartialEq, Eq, StdHash, Serialize, Deserialize)]
pub struct Topic([u8; 32]);

impl Topic {
    pub fn new(topic: &str) -> Self {
        Self(Hash::new(topic.as_bytes()).into())
    }
}

impl TopicQuery for Topic {}

impl TopicId for Topic {
    fn id(&self) -> [u8; 32] {
        self.0
    }
}

/// Remember which topic was used by which "author", identified by it's public key.
#[derive(Debug)]
pub struct AuthorStore(Arc<RwLock<HashMap<Topic, HashSet<PublicKey>>>>);

impl AuthorStore {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub async fn add_author(&mut self, topic: Topic, public_key: PublicKey) {
        let mut authors = self.0.write().await;
        authors
            .entry(topic)
            .and_modify(|public_keys| {
                public_keys.insert(public_key);
            })
            .or_insert({
                let mut public_keys = HashSet::new();
                public_keys.insert(public_key);
                public_keys
            });
    }

    pub async fn authors(&self, topic: &Topic) -> Option<HashSet<PublicKey>> {
        let authors = self.0.read().await;
        authors.get(topic).cloned()
    }
}

#[async_trait]
impl TopicLogMap<Topic, LogId> for AuthorStore {
    /// During sync other peers are interested in all our append-only logs for a certain topic.
    /// This method tells the sync protocol which logs we have available from which author for that
    /// given topic.
    async fn get(&self, topic: &Topic) -> Option<HashMap<PublicKey, Vec<LogId>>> {
        let authors = self.authors(topic).await;
        authors.map(|authors| {
            let mut map = HashMap::with_capacity(authors.len());
            for author in authors {
                map.insert(author, vec![DEFAULT_LOG_ID]);
            }
            map
        })
    }
}
