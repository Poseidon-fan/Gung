use std::{collections::HashSet, sync::LazyLock};

use anyhow::{Result, bail};
use parking_lot::Mutex;
use rand_set::RandSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Port(pub u16);

pub fn alloc(pointed_port: Option<u16>) -> Result<Port> {
    let port = PORT_MANAGER.lock().alloc(pointed_port)?;
    Ok(Port(port))
}

pub fn init(allowed: Option<HashSet<u16>>) -> Result<()> {
    if let Some(allowed) = allowed {
        PORT_MANAGER.lock().init(allowed);
    }
    Ok(())
}

impl Drop for Port {
    fn drop(&mut self) {
        PORT_MANAGER.lock().dealloc(self.0);
    }
}

pub struct PortManager {
    free_ports: RandSet<u16>,
}

impl Default for PortManager {
    fn default() -> Self {
        Self {
            free_ports: RandSet::from_iter(0..65535),
        }
    }
}

impl PortManager {
    pub fn init(&mut self, allowed: HashSet<u16>) {
        self.free_ports = RandSet::from_iter(allowed);
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

static PORT_MANAGER: LazyLock<Mutex<PortManager>> =
    LazyLock::new(|| Mutex::new(PortManager::default()));
