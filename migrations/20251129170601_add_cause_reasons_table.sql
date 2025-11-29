-- Create cause_reasons table to store cause/reason definitions in the database
-- This replaces the hardcoded CauseReason enum

CREATE TABLE cause_reasons (
    id INTEGER PRIMARY KEY,
    label VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    is_hidden BOOLEAN NOT NULL DEFAULT 0
);

-- Insert the default cause reasons from the hardcoded enum
-- ID 0 must be the "Undefined" reason
INSERT INTO cause_reasons (id, label, description, is_hidden) VALUES
    (0, 'Undefined', 'No specific reason recorded', 0),
    (1, 'Ice Exception', 'AC is OFF because outdoor temperature is below 2°C. When running in cold conditions, the AC unit would go through a defrost cycle that pulls warm air out of the room, making heating inefficient. We rely solely on central heating instead. This exception is bypassed if indoor temperature drops below 12°C or if solar production is above 1000W.', 0),
    (2, 'PIR Detection', 'AC is OFF due to motion detection. The PIR (Passive Infrared) sensor detected movement near the AC unit, and the system automatically turns off the AC to avoid blowing air directly at people, which can be uncomfortable.', 0),
    (3, 'Nobody Home', 'Operating at low intensity because nobody is home. The system maintains basic temperature control while minimizing energy usage when the space is unoccupied.', 0),
    (4, 'Mild Temperature', 'Operating at low intensity because outdoor temperature is close to the desired indoor temperature. Minimal climate control is needed in these mild conditions.', 0),
    (5, 'Major Temperature Change Pending', 'Operating at high intensity due to a significant temperature change forecast. The system is taking preemptive action to prepare for upcoming weather changes.', 0),
    (6, 'Excessive Solar Power', 'Operating at high intensity (Powerful mode) to utilize excess solar power production. This aggressive climate control has minimal environmental and cost impact when solar production is high.', 0),
    (7, 'Manual to Auto Transition', 'The AC device was switched from manual control to automatic mode. The system is sending the appropriate command to immediately establish the desired climate control state.', 0);
