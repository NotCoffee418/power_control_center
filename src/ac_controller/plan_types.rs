

// Configurable parameters for AC behavior plans
/// Minimum active solar power to consider Powerful (High Intensity) mode
const SOLAR_HIGH_INTENSITY_WATT_THRESHOLD: u32 = 2000; // Watts


/// Vague request for changing temperature
/// To be specified by settings
pub(super) enum RequestMode {
    Colder(Intensity),
    Warmer(Intensity),
    NoChange,
}

/// Intensity levels of desired temperature change
pub(super) enum Intensity {
    Low,    // Maintain not freezing/smelting temperature
    Medium, // Keep it comfortable
    High,   // Using Powerful, when excess solar power available
}

/// AcDevices must be defined in config
pub(super) enum AcDevices {
    LivingRoom,
    Veranda,
}
impl AcDevices {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            AcDevices::LivingRoom => "LivingRoom",
            AcDevices::Veranda => "Veranda",
        }
    }
}
