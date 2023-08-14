use slab_tree::*;
use std::{any::TypeId, sync::Arc};
use tokio::sync::{Barrier, Mutex};
use super::{dns_parsing::DnsResourceRecord, domain_name::DomainName};


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
    ) -> NodeId {
        let mut lock = self.tree.lock().await;
        lock.get_mut(parent).expect("No such node exists").append(child).node_id()
    }

    // pub async fn tree_get_label(
    //     &self,
    //     node: NodeId
    // ) -> String {
    //     let label: String;
    //     let lock = self.tree.lock().await;
    //     label = lock.get(node).unwrap().data().label.clone();
    //     label
    // }

    pub async fn get_best_zone_match(
        &self,
        qname: DomainName
    ) -> Result<Vec<DnsResourceRecord>, DnsTreeError> {
        let r_iter = qname.0.iter().rev();
        let lock = self.tree.lock().await;
        let mut id = lock.root_id().unwrap();
        for s in r_iter {
            // println!("Domain Name Label: {:?}", s);
            for n in lock.get(id).unwrap().children() {
                // println!("Domain Name Label: {:?} | Zone Tree Node Label: {:?}", s, n.data().label);
                if n.data().label == *s {
                    id = n.node_id();
                    // println!("A match!");
                }
                else {
                    // println!("No match!");
                }
            }
        }
        let data = lock.get(id).unwrap().data();
        // println!("{:?}", lock.get(id).unwrap().data().label);
        if data.label == qname.0[0] {
            Ok(data.record_list.to_owned())
        } else {
            Err(DnsTreeError::Tree)
        }
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DnsTreeError {
    #[error("DNS tree lookup had no result")]
    Tree,
    // #[error("Unspecified DNS Server error")]
    // Other,
}
