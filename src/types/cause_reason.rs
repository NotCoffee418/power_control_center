use serde::{Serialize, Deserialize};

/// Enum representing the reason/cause for an AC action or decision
/// Each variant has a unique ID for database storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum CauseReason {
    /// Default/undefined reason
    Undefined = 0,
    /// AC is OFF due to cold outdoor temperature (ice exception)
    /// Prevents AC from running when outdoor temp < 5°C to avoid ice formation
    IceException = 1,
    /// AC is OFF due to PIR (motion) detection
    /// Automatically turns off AC when motion is detected to avoid blowing air directly at people
    PirDetection = 2,
    /// Low intensity operation - nobody home
    /// Minimal AC operation when user is away to maintain basic temperature control
    NobodyHome = 3,
    /// Low intensity operation - mild outdoor temperature
    /// Outdoor temperature is close to room temperature, minimal AC needed
    MildTemperature = 4,
    /// High intensity operation - major temperature change forecast
    /// Weather forecast shows significant temperature change, preemptive action needed
    MajorTemperatureChangePending = 5,
    /// High intensity operation - excessive solar power available
    /// High solar production allows powerful AC operation without grid impact
    ExcessiveSolarPower = 6,
}

impl CauseReason {
    /// Get the numeric ID for database storage
    pub fn id(&self) -> i32 {
        *self as i32
    }

    /// Get the human-readable label for this cause
    pub fn label(&self) -> &'static str {
        match self {
            CauseReason::Undefined => "Undefined",
            CauseReason::IceException => "Ice Exception",
            CauseReason::PirDetection => "PIR Detection",
            CauseReason::NobodyHome => "Nobody Home",
            CauseReason::MildTemperature => "Mild Temperature",
            CauseReason::MajorTemperatureChangePending => "Major Temperature Change Pending",
            CauseReason::ExcessiveSolarPower => "Excessive Solar Power",
        }
    }

    /// Get the detailed description explaining this cause
    pub fn description(&self) -> &'static str {
        match self {
            CauseReason::Undefined => "No specific reason recorded",
            CauseReason::IceException => "AC is OFF because outdoor temperature is below 5°C. When running in cold conditions, the AC unit would go through a defrost cycle that pulls warm air out of the room, making heating inefficient. We rely solely on central heating instead. This exception is bypassed if indoor temperature drops below 12°C to prevent the room from becoming too cold.",
            CauseReason::PirDetection => "AC is OFF due to motion detection. The PIR (Passive Infrared) sensor detected movement near the AC unit, and the system automatically turns off the AC to avoid blowing air directly at people, which can be uncomfortable.",
            CauseReason::NobodyHome => "Operating at low intensity because nobody is home. The system maintains basic temperature control while minimizing energy usage when the space is unoccupied.",
            CauseReason::MildTemperature => "Operating at low intensity because outdoor temperature is close to the desired indoor temperature. Minimal climate control is needed in these mild conditions.",
            CauseReason::MajorTemperatureChangePending => "Operating at high intensity due to a significant temperature change forecast. The system is taking preemptive action to prepare for upcoming weather changes.",
            CauseReason::ExcessiveSolarPower => "Operating at high intensity (Powerful mode) to utilize excess solar power production. This aggressive climate control has minimal environmental and cost impact when solar production is high.",
        }
    }

    /// Convert from a numeric ID to a CauseReason
    pub fn from_id(id: i32) -> Self {
        match id {
            0 => CauseReason::Undefined,
            1 => CauseReason::IceException,
            2 => CauseReason::PirDetection,
            3 => CauseReason::NobodyHome,
            4 => CauseReason::MildTemperature,
            5 => CauseReason::MajorTemperatureChangePending,
            6 => CauseReason::ExcessiveSolarPower,
            _ => CauseReason::Undefined, // Default to Undefined for unknown IDs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cause_reason_ids() {
        assert_eq!(CauseReason::Undefined.id(), 0);
        assert_eq!(CauseReason::IceException.id(), 1);
        assert_eq!(CauseReason::PirDetection.id(), 2);
        assert_eq!(CauseReason::NobodyHome.id(), 3);
        assert_eq!(CauseReason::MildTemperature.id(), 4);
        assert_eq!(CauseReason::MajorTemperatureChangePending.id(), 5);
        assert_eq!(CauseReason::ExcessiveSolarPower.id(), 6);
    }

    #[test]
    fn test_cause_reason_labels() {
        assert_eq!(CauseReason::Undefined.label(), "Undefined");
        assert_eq!(CauseReason::IceException.label(), "Ice Exception");
        assert_eq!(CauseReason::PirDetection.label(), "PIR Detection");
        assert_eq!(CauseReason::NobodyHome.label(), "Nobody Home");
        assert_eq!(CauseReason::MildTemperature.label(), "Mild Temperature");
        assert_eq!(CauseReason::MajorTemperatureChangePending.label(), "Major Temperature Change Pending");
        assert_eq!(CauseReason::ExcessiveSolarPower.label(), "Excessive Solar Power");
    }

    #[test]
    fn test_cause_reason_descriptions() {
        assert!(!CauseReason::Undefined.description().is_empty());
        assert!(!CauseReason::IceException.description().is_empty());
        assert!(CauseReason::IceException.description().contains("5°C"));
        assert!(CauseReason::IceException.description().contains("12°C"));
    }

    #[test]
    fn test_from_id() {
        assert_eq!(CauseReason::from_id(0), CauseReason::Undefined);
        assert_eq!(CauseReason::from_id(1), CauseReason::IceException);
        assert_eq!(CauseReason::from_id(2), CauseReason::PirDetection);
        assert_eq!(CauseReason::from_id(3), CauseReason::NobodyHome);
        assert_eq!(CauseReason::from_id(4), CauseReason::MildTemperature);
        assert_eq!(CauseReason::from_id(5), CauseReason::MajorTemperatureChangePending);
        assert_eq!(CauseReason::from_id(6), CauseReason::ExcessiveSolarPower);
        assert_eq!(CauseReason::from_id(999), CauseReason::Undefined); // Unknown defaults to Undefined
    }

    #[test]
    fn test_round_trip_conversion() {
        let causes = vec![
            CauseReason::Undefined, 
            CauseReason::IceException, 
            CauseReason::PirDetection,
            CauseReason::NobodyHome,
            CauseReason::MildTemperature,
            CauseReason::MajorTemperatureChangePending,
            CauseReason::ExcessiveSolarPower,
        ];
        for cause in causes {
            let id = cause.id();
            let converted = CauseReason::from_id(id);
            assert_eq!(converted, cause);
        }
    }
}
