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
        }
    }

    /// Get the detailed description explaining this cause
    pub fn description(&self) -> &'static str {
        match self {
            CauseReason::Undefined => "No specific reason recorded",
            CauseReason::IceException => "AC is OFF because outdoor temperature is below 5°C. The AC will pull hot air out of the room to de-ice itself, so we rely solely on central heating instead. This exception is bypassed if indoor temperature drops below 12°C.",
        }
    }

    /// Convert from a numeric ID to a CauseReason
    pub fn from_id(id: i32) -> Self {
        match id {
            0 => CauseReason::Undefined,
            1 => CauseReason::IceException,
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
    }

    #[test]
    fn test_cause_reason_labels() {
        assert_eq!(CauseReason::Undefined.label(), "Undefined");
        assert_eq!(CauseReason::IceException.label(), "Ice Exception");
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
        assert_eq!(CauseReason::from_id(999), CauseReason::Undefined); // Unknown defaults to Undefined
    }

    #[test]
    fn test_round_trip_conversion() {
        let causes = vec![CauseReason::Undefined, CauseReason::IceException];
        for cause in causes {
            let id = cause.id();
            let converted = CauseReason::from_id(id);
            assert_eq!(converted, cause);
        }
    }
}
