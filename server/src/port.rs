#![allow(dead_code)]
use std::{collections::HashSet, sync::LazyLock};

use anyhow::{Result, bail};
use parking_lot::Mutex;
use rand_set::RandSet;

pub struct Port(u16);

pub fn alloc(pointed_port: Option<u16>) -> Result<Port> {
    let port = PORT_MANAGER.lock().alloc(pointed_port)?;
    Ok(Port(port))
}

impl Drop for Port {
    fn drop(&mut self) {
        PORT_MANAGER.lock().dealloc(self.0);
    }
}

pub struct PortManager {
    free_ports: RandSet<u16>,
}

impl PortManager {
    fn new() -> Self {
        Self {
            free_ports: RandSet::new(),
        }
    }

    pub fn init(allowed: HashSet<u16>) -> Self {
        Self {
            free_ports: RandSet::from_iter(allowed),
        }
    }

    fn alloc(&mut self, pointed_port: Option<u16>) -> Result<u16> {
        let target = match pointed_port {
            Some(port) => {
                if self.free_ports.contains(&port) {
                    port
                } else {
                    bail!("Port {port} is not free");
                }
            }
            None => {
                if let Some(port) = self.free_ports.get_rand() {
                    *port
                } else {
                    bail!("No free port");
                }
            }
        };
        self.free_ports.remove(&target);
        Ok(target)
    }

    fn dealloc(&mut self, port: u16) {
        self.free_ports.insert(port);
    }
}

pub static PORT_MANAGER: LazyLock<Mutex<PortManager>> =
    LazyLock::new(|| Mutex::new(PortManager::new()));
