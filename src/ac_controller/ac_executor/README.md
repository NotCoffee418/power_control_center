# AC Executor Module

This module is responsible for executing AC control plans by managing device state and sending API calls to AC remotes only when changes are needed.

## Overview

The AC executor acts as an intelligent middleware between the planning module and the actual AC API calls. It:
1. Tracks the current state of each AC device
2. Compares new plans against current state
3. Only sends IR remote signals when changes are detected
4. Converts high-level plans (Colder/Warmer/NoChange) into concrete AC settings

## Architecture

### Core Components

#### `AcStateManager`
- Thread-safe singleton that tracks the state of all AC devices
- Uses `RwLock<HashMap>` for concurrent access
- Automatically initializes with "off" state for new devices
- Tracks initialization status to ensure commands are sent on first execution after startup

#### `AcState`
- Represents the complete state of an AC device:
  - `is_on`: Whether the AC is powered on
  - `mode`: Heat (1) or Cool (4) mode
  - `fan_speed`: 0 (auto) to 5 (max)
  - `temperature`: Target temperature in Celsius
  - `swing`: 0 (off) or 1 (on)
  - `powerful_mode`: Whether powerful mode is active
- Implements `PartialEq` for automatic change detection

#### `plan_to_state()`
Converts abstract plans into concrete states:
- `RequestMode::NoChange` → AC off
- `RequestMode::Colder(intensity)` → Cooling with appropriate settings
- `RequestMode::Warmer(intensity)` → Heating with appropriate settings

## Usage Example

```rust
use crate::ac_controller::ac_executor;
use crate::ac_controller::plan_types::{AcDevices, RequestMode, Intensity};

// Execute a plan for the living room AC
let device = AcDevices::LivingRoom;
let plan = RequestMode::Colder(Intensity::Medium);

match ac_executor::execute_plan(&device, &plan).await {
    Ok(true) => println!("AC state changed successfully"),
    Ok(false) => println!("No change needed, AC already in desired state"),
    Err(e) => eprintln!("Error executing plan: {}", e),
}
```

## Intensity Level Mappings

### Low Intensity
- **Purpose**: Maintain non-extreme temperatures when user is away
- **Settings**: 26°C cooling / 19°C heating, auto fan, no powerful mode

### Medium Intensity
- **Purpose**: Comfortable temperature for when user is home
- **Settings**: 22°C both modes, auto fan, no powerful mode

### High Intensity
- **Purpose**: Maximum comfort when excess solar power available
- **Settings**: 20°C cooling / 24°C heating, max fan (5), powerful mode enabled

## State Management

The executor automatically handles state transitions:

1. **Off → On**: Sends turn_on_ac() with all parameters
2. **On → Off**: Sends turn_off_ac()
3. **On → On (different settings)**: Sends turn_on_ac() with new parameters
4. **Powerful mode toggle**: Sends toggle_powerful() when needed

### Startup Behavior

On application startup, the executor doesn't know the actual physical state of AC devices. To ensure synchronization:

- **First Execution**: Commands are always sent for each device on the first call to `execute_plan()`, regardless of the tracked state
- **Subsequent Executions**: Normal state comparison is used to minimize API calls
- This ensures the physical AC state matches the calculated plan even if the device was left in a different state

The initialization tracking is automatically reset when calling `reset_device_state()` or `reset_all_states()`.

## API Call Optimization

The module ensures minimal API calls by:
- Tracking previous state for each device
- Tracking initialization status for startup synchronization
- Comparing new plans against current state using `PartialEq`
- Skipping API calls when state hasn't changed (except on first execution)
- Only sending the minimum required commands

## Error Handling

All API errors are propagated to the caller, allowing for:
- Retry logic at higher levels
- Logging of failures
- Graceful degradation

The state is only updated on successful API calls, ensuring consistency.

## Testing

The module includes comprehensive tests for:
- State equality and change detection
- Plan-to-state conversion
- State manager operations
- Integration scenarios

Run tests with:
```bash
cargo test ac_executor
```

## Constants

- `AC_MODE_HEAT = 1`: Heating mode
- `AC_MODE_COOL = 4`: Cooling mode

These constants match the API expected by the AC remote devices.

## Thread Safety

All state management operations are thread-safe:
- `AcStateManager` uses `RwLock` for concurrent access
- Multiple devices can be controlled simultaneously
- State updates are atomic

## Future Enhancements

Potential improvements (not yet implemented):
- Persistent state across application restarts
- State verification by querying sensor data
- Automatic recovery from desynchronization
- Rate limiting for API calls
