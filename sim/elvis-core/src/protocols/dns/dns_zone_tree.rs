use slab_tree::*;
use crate::FxDashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use super::{dns_parsing::DnsResourceRecord, domain_name::DomainName};


#[derive(Debug)]
pub struct DnsZoneNode {
    pub name: String,
    pub parent_name: String,
    pub record_list: Vec<DnsResourceRecord>,
}

impl DnsZoneNode {
    pub fn new(name: String, parent_name: String, record_list: Vec<DnsResourceRecord>) -> Self {
        Self {
            name,
            parent_name,
            record_list,
        }
    }
}

#[derive(Debug, Default)]
pub struct DnsZoneTree {
    pub tree: Arc<Mutex<Tree<DnsZoneNode>>>,
    pub name_to_id: FxDashMap<String, slab_tree::NodeId>,
}

impl DnsZoneTree {
    pub fn new() -> Self {
        Self {
            tree: Default::default(),
            name_to_id: Default::default(),
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
                // println!("Domain Name Label: {:?} | Zone Tree Node Label: {:?}", s, n.data().name);
                if n.data().name == *s {
                    id = n.node_id();
                    // println!("A match!");
                }
                // else {
                //     println!("No match!");
                // }
            }
        }
        let data = lock.get(id).unwrap().data();
        // println!("{:?}", lock.get(id).unwrap().data().name);
        if data.name == qname.0[0] {
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
