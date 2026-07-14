//! LAN Device Discovery using mDNS (Bonjour/Avahi).
//!
//! Each IRIS instance advertises itself as an mDNS service and
//! simultaneously browses for other instances on the local network.

use crate::protocol::{Capabilities, DeviceInfo, MDNS_SERVICE_TYPE};
use anyhow::Result;
use log::{info, warn};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Holds the state of discovered devices
pub struct DiscoveryService {
    daemon: ServiceDaemon,
    /// Service name used for our own advertisement
    our_service_name: String,
    /// Currently discovered devices (device_id -> DeviceInfo)
    devices: Arc<Mutex<HashMap<String, DeviceInfo>>>,
    /// Broadcast channel to notify Flutter UI of changes
    device_tx: broadcast::Sender<DiscoveryEvent>,
}

#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    DeviceAdded(DeviceInfo),
    DeviceRemoved(String), // device_id
    DeviceUpdated(DeviceInfo),
}

impl DiscoveryService {
    /// Create and start the discovery service.
    pub fn new(device_name: &str, _platform: &str) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        let our_service_name = format!("{}-{}", device_name, uuid_v4());

        let devices = Arc::new(Mutex::new(HashMap::new()));
        let (device_tx, _) = broadcast::channel(64);

        let service = Self {
            daemon,
            our_service_name,
            devices,
            device_tx,
        };

        Ok(service)
    }

    /// Start advertising this device and browsing for others.
    pub fn start(
        &self,
        device_name: &str,
        platform: &str,
        tcp_port: u16,
        device_id: &str,
    ) -> Result<()> {
        // Get local IP addresses
        let local_ips = get_local_ips();

        // Build properties as HashMap (required by mdns-sd v0.11 IntoTxtProperties)
        let mut txt_properties = std::collections::HashMap::new();
        txt_properties.insert("device_id".to_string(), device_id.to_string());
        txt_properties.insert("device_name".to_string(), device_name.to_string());
        txt_properties.insert("platform".to_string(), platform.to_string());
        txt_properties.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        txt_properties.insert("screen_share".to_string(), "true".to_string());
        txt_properties.insert("file_transfer".to_string(), "true".to_string());

        let service_info = ServiceInfo::new(
            MDNS_SERVICE_TYPE,
            &self.our_service_name,
            &format!("{}.local.", hostname()),
            &local_ips[..],
            tcp_port,
            Some(txt_properties),
        )
        .map_err(|e| anyhow::anyhow!("Failed to create service info: {}", e))?;

        self.daemon.register(service_info)?;

        // Start browsing
        let devices = self.devices.clone();
        let tx = self.device_tx.clone();
        let browser = self.daemon.browse(MDNS_SERVICE_TYPE)?;

        tokio::spawn(async move {
            loop {
                match browser.recv_async().await {
                    Ok(event) => {
                        match event {
                            ServiceEvent::ServiceResolved(info) => {
                                let device = service_info_to_device(&info);
                                if let Some(ref device) = device {
                                    let device_id = device.device_id.clone();
                                    let device_name = device.device_name.clone();
                                    let is_new = {
                                        let mut map = devices.lock().unwrap();
                                        let is_new = !map.contains_key(&device_id);
                                        map.insert(device_id.clone(), device.clone());
                                        is_new
                                    };
                                    if is_new {
                                        let _ = tx.send(DiscoveryEvent::DeviceAdded(device.clone()));
                                        info!("Device discovered: {} ({})", device_name, device_id);
                                    } else {
                                        let _ = tx.send(DiscoveryEvent::DeviceUpdated(device.clone()));
                                    }
                                }
                            }
                            ServiceEvent::ServiceRemoved(_service_type, fullname) => {
                                // Extract device_id from service properties
                                let mut map = devices.lock().unwrap();
                                // Find and remove by full service name
                                let removed: Vec<String> = map
                                    .iter()
                                    .filter(|(_, v)| {
                                        v.device_name == fullname
                                            || fullname.contains(&v.device_id)
                                    })
                                    .map(|(k, _)| k.clone())
                                    .collect();
                                for id in &removed {
                                    map.remove(id);
                                    let _ = tx.send(DiscoveryEvent::DeviceRemoved(id.clone()));
                                    info!("Device removed: {}", id);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        warn!("mDNS browse error: {}", e);
                        break;
                    }
                }
            }
        });

        info!(
            "Discovery started. Advertising as '{}' on port {}",
            device_name, tcp_port
        );
        Ok(())
    }

    /// Get a receiver for discovery events (to send to Flutter)
    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveryEvent> {
        self.device_tx.subscribe()
    }

    /// Get current list of discovered devices
    pub fn get_devices(&self) -> Vec<DeviceInfo> {
        self.devices.lock().unwrap().values().cloned().collect()
    }
}

impl Drop for DiscoveryService {
    fn drop(&mut self) {
        // Unregister our service on drop
        let _ = self.daemon.unregister(&self.our_service_name);
    }
}

/// Convert mDNS ServiceInfo to DeviceInfo
fn service_info_to_device(info: &ServiceInfo) -> Option<DeviceInfo> {
    let props = info.get_properties();
    let device_id = props.get("device_id")?.val_str().to_string();
    let device_name = props
        .get("device_name")
        .map(|p| p.val_str().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let platform = props
        .get("platform")
        .map(|p| p.val_str().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let screen_share = props
        .get("screen_share")
        .map(|v| v.val_str() == "true")
        .unwrap_or(true);
    let file_transfer = props
        .get("file_transfer")
        .map(|v| v.val_str() == "true")
        .unwrap_or(true);

    let ip_addresses: Vec<String> = info
        .get_addresses()
        .iter()
        .map(|a| a.to_string())
        .collect();

    Some(DeviceInfo {
        device_id,
        device_name,
        platform,
        ip_addresses,
        capabilities: Capabilities {
            screen_share,
            file_transfer,
            max_resolution: (1920, 1080),
        },
    })
}

/// Get local non-loopback IP addresses
fn get_local_ips() -> Vec<IpAddr> {
    if_addrs::get_if_addrs()
        .map(|ifaces| {
            ifaces
                .into_iter()
                .filter(|i| !i.is_loopback())
                .map(|i| i.ip())
                .collect()
        })
        .unwrap_or_default()
}

fn hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Simple UUID v4 generation (no external crate needed for basic use)
fn uuid_v4() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        (bytes[6] & 0x0f) | 0x40, bytes[7],
        (bytes[8] & 0x3f) | 0x80, bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}
