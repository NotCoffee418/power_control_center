/// Vague request for changing temperature
/// To be specified by settings
pub(super) enum RequestTemp {
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
