/// AcDevices must be defined in config
#[derive(Debug)]
pub enum AcDevices {
    LivingRoom,
    Veranda,
}

impl AcDevices {
    pub fn as_str(&self) -> &'static str {
        match self {
            AcDevices::LivingRoom => "LivingRoom",
            AcDevices::Veranda => "Veranda",
        }
    }

    /// Convert a device name string to AcDevices enum
    /// Returns None if the device name is not recognized
    pub fn from_str(device: &str) -> Option<Self> {
        match device {
            "LivingRoom" => Some(AcDevices::LivingRoom),
            "Veranda" => Some(AcDevices::Veranda),
            _ => None,
        }
    }

    /// Get all AC devices for iteration
    pub fn all() -> Vec<Self> {
        vec![
            AcDevices::LivingRoom,
            AcDevices::Veranda,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ac_devices_as_str() {
        assert_eq!(AcDevices::LivingRoom.as_str(), "LivingRoom");
        assert_eq!(AcDevices::Veranda.as_str(), "Veranda");
    }

    #[test]
    fn test_ac_devices_from_str() {
        // Test valid device names
        assert!(matches!(AcDevices::from_str("LivingRoom"), Some(AcDevices::LivingRoom)));
        assert!(matches!(AcDevices::from_str("Veranda"), Some(AcDevices::Veranda)));
        
        // Test invalid device names
        assert!(AcDevices::from_str("Unknown").is_none());
        assert!(AcDevices::from_str("").is_none());
        assert!(AcDevices::from_str("livingroom").is_none()); // Case sensitive
    }

    #[test]
    fn test_ac_devices_round_trip() {
        // Test that as_str and from_str are consistent
        let devices = vec![AcDevices::LivingRoom, AcDevices::Veranda];
        for device in devices {
            let device_str = device.as_str();
            let parsed = AcDevices::from_str(device_str);
            assert!(parsed.is_some());
            assert_eq!(parsed.unwrap().as_str(), device_str);
        }
    }
}
