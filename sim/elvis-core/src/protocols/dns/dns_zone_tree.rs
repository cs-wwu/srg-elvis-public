
use slab_tree::*;
use std::{any::TypeId, sync::Arc};
use tokio::sync::{Barrier, Mutex};
use super::dns_parsing::DnsResourceRecord;


#[derive(Debug)]
pub struct DnsZoneNode {
    pub label: String,
    pub record_list: Vec<DnsResourceRecord>,
}

impl DnsZoneNode {
    pub fn new(label: String, record_list: Vec<DnsResourceRecord>) -> Self {
        Self {
            label,
            record_list,
        }
    }
}

#[derive(Debug, Default)]
pub struct DnsZoneTree {
    pub tree: Arc<Mutex<Tree<DnsZoneNode>>>,
}

impl DnsZoneTree {
    pub fn new() -> Self {
        Self {
            tree: Default::default()
        }
    }

    pub async fn tree_add_root(
        &self,
        root: DnsZoneNode
    ) {
        let mut lock = self.tree.lock().await;
        lock.set_root(root);
    }

    pub async fn tree_add_child(
        &self,
        parent: NodeId,
        child: DnsZoneNode
    ) {
        let mut lock = self.tree.lock().await;
        lock.get_mut(parent).expect("No such node exists").append(child);
    }
}