<script>
  import { onMount } from 'svelte';

  // Constants
  /** ID used for new unsaved nodesets that haven't been saved to the database yet */
  const NEW_NODESET_ID = -1;

  // Props
  let { isOpen = $bindable(true), currentNodesetId = $bindable(null), nodes = $bindable([]), edges = $bindable([]) } = $props();

  // State
  let drawerHeight = $state(300);
  let isResizing = $state(false);
  let loading = $state(false);
  let evaluating = $state(false);

  // Input values (stored as strings for text inputs to allow validation)
  let selectedDevice = $state('');
  let temperatureStr = $state('22.0');
  let isAutoMode = $state(true);
  let solarProductionStr = $state('0');
  let outdoorTempStr = $state('20.0');
  let avgNext24hOutdoorTempStr = $state('20.0');
  let userIsHome = $state(true);
  let pirDetected = $state(false);
  let pirMinutesAgoStr = $state('0');
  let lastChangeMinutesStr = $state('60');
  let netPowerWattStr = $state('0');

  // Active Command state
  let activeCommandIsDefined = $state(false);
  let activeCommandIsOn = $state(false);
  let activeCommandTemperatureStr = $state('22.0');
  let activeCommandMode = $state('Cool'); // 'Heat', 'Cool', or 'Off'
  let activeCommandFanSpeedStr = $state('0'); // 0-5, where 0 is auto
  let activeCommandSwingStr = $state('0'); // 0 = off, 1 = on
  let activeCommandIsPowerful = $state(false);

  // Available devices
  let devices = $state([]);
  
  // Simulation result
  let simulationResult = $state(null);
  let errorMessage = $state('');

  // Helper function to round a float to 1 decimal place
  function roundToOneDecimal(value) {
    return Math.round(value * 10) / 10;
  }

  // Validation helpers - return true if the value is valid
  // Uses regex to ensure only standard numeric formats are accepted (no scientific notation)
  function isValidFloat(str) {
    if (str === '' || str === null || str === undefined) return false;
    const trimmed = String(str).trim();
    // Only allow standard decimal format: optional minus, digits, optional decimal point with digits
    if (!/^-?\d+(\.\d+)?$/.test(trimmed)) return false;
    const parsed = parseFloat(trimmed);
    return !isNaN(parsed) && isFinite(parsed);
  }

  function isValidInteger(str) {
    if (str === '' || str === null || str === undefined) return false;
    const trimmed = String(str).trim();
    // Only allow integers: optional minus, digits only (no decimal point, no scientific notation)
    if (!/^-?\d+$/.test(trimmed)) return false;
    const parsed = parseInt(trimmed, 10);
    return !isNaN(parsed) && isFinite(parsed);
  }

  // Get parsed numeric values (or default if invalid)
  function getTemperature() {
    return isValidFloat(temperatureStr) ? parseFloat(temperatureStr) : 0;
  }
  function getSolarProduction() {
    return isValidInteger(solarProductionStr) ? parseInt(solarProductionStr, 10) : 0;
  }
  function getOutdoorTemp() {
    return isValidFloat(outdoorTempStr) ? parseFloat(outdoorTempStr) : 0;
  }
  function getAvgNext24hOutdoorTemp() {
    return isValidFloat(avgNext24hOutdoorTempStr) ? parseFloat(avgNext24hOutdoorTempStr) : 0;
  }
  function getPirMinutesAgo() {
    return isValidInteger(pirMinutesAgoStr) ? parseInt(pirMinutesAgoStr, 10) : 0;
  }
  function getLastChangeMinutes() {
    return isValidInteger(lastChangeMinutesStr) ? parseInt(lastChangeMinutesStr, 10) : 0;
  }
  function getNetPowerWatt() {
    return isValidInteger(netPowerWattStr) ? parseInt(netPowerWattStr, 10) : 0;
  }
  function getActiveCommandTemperature() {
    return isValidFloat(activeCommandTemperatureStr) ? parseFloat(activeCommandTemperatureStr) : 0;
  }
  function getActiveCommandFanSpeed() {
    return isValidInteger(activeCommandFanSpeedStr) ? parseInt(activeCommandFanSpeedStr, 10) : 0;
  }
  function getActiveCommandSwing() {
    return isValidInteger(activeCommandSwingStr) ? parseInt(activeCommandSwingStr, 10) : 0;
  }
  function getActiveCommandModeInt() {
    // Convert mode string to integer (1 = Heat, 4 = Cool, 0 = Off)
    switch (activeCommandMode) {
      case 'Heat': return 1;
      case 'Cool': return 4;
      case 'Off': return 0;
      default: return 0;
    }
  }

  // Check if all inputs are valid
  function areAllInputsValid() {
    const baseValid = isValidFloat(temperatureStr) &&
           isValidInteger(solarProductionStr) &&
           isValidFloat(outdoorTempStr) &&
           isValidFloat(avgNext24hOutdoorTempStr) &&
           isValidInteger(pirMinutesAgoStr) &&
           isValidInteger(lastChangeMinutesStr) &&
           isValidInteger(netPowerWattStr);
    
    // If active command is defined, validate its fields too
    if (activeCommandIsDefined) {
      return baseValid &&
             isValidFloat(activeCommandTemperatureStr) &&
             isValidInteger(activeCommandFanSpeedStr) &&
             isValidInteger(activeCommandSwingStr);
    }
    
    return baseValid;
  }

  // Load live inputs from backend
  async function loadLiveInputs() {
    loading = true;
    errorMessage = '';
    try {
      const response = await fetch('/api/simulator/live-inputs');
      const result = await response.json();
      
      if (result.success && result.data) {
        const data = result.data;
        
        // Update devices list
        devices = data.devices.map(d => d.name);
        
        // Select first device if none selected
        if (!selectedDevice && devices.length > 0) {
          selectedDevice = devices[0];
        }
        
        // Update device-specific values for selected device
        const deviceData = data.devices.find(d => d.name === selectedDevice);
        if (deviceData) {
          if (deviceData.temperature !== null) {
            temperatureStr = String(roundToOneDecimal(deviceData.temperature));
          }
          isAutoMode = deviceData.is_auto_mode;
          pirDetected = deviceData.pir_recently_triggered;
          pirMinutesAgoStr = String(deviceData.pir_minutes_ago ?? 0);
          lastChangeMinutesStr = String(deviceData.last_change_minutes ?? 60);
        }
        
        // Update environmental values (round floats to 1 decimal)
        if (data.solar_production !== null) {
          solarProductionStr = String(data.solar_production);
        }
        if (data.outdoor_temp !== null) {
          outdoorTempStr = String(roundToOneDecimal(data.outdoor_temp));
        }
        if (data.avg_next_24h_outdoor_temp !== null) {
          avgNext24hOutdoorTempStr = String(roundToOneDecimal(data.avg_next_24h_outdoor_temp));
        }
        if (data.net_power_watt !== null) {
          netPowerWattStr = String(data.net_power_watt);
        }
        userIsHome = data.user_is_home;
      } else {
        errorMessage = result.error || 'Failed to load live inputs';
      }
    } catch (e) {
      console.error('Error loading live inputs:', e);
      errorMessage = 'Failed to connect to server';
    } finally {
      loading = false;
    }
  }

  // Evaluate the workflow with current inputs
  async function evaluate() {
    // Validate all inputs before evaluating
    if (!areAllInputsValid()) {
      errorMessage = 'Please fix invalid input values (highlighted in red)';
      return;
    }
    
    evaluating = true;
    errorMessage = '';
    simulationResult = null;
    
    try {
      // Build the request payload
      // Always use the currently displayed nodes/edges from the editor (what the user sees)
      // This ensures the simulator runs on the current state, not a saved version
      const payload = {
        device: selectedDevice,
        temperature: getTemperature(),
        is_auto_mode: isAutoMode,
        solar_production: getSolarProduction(),
        outdoor_temp: getOutdoorTemp(),
        avg_next_24h_outdoor_temp: getAvgNext24hOutdoorTemp(),
        user_is_home: userIsHome,
        pir_detected: pirDetected,
        pir_minutes_ago: getPirMinutesAgo(),
        last_change_minutes: getLastChangeMinutes(),
        net_power_watt: getNetPowerWatt(),
        // Always pass -1 to indicate we're using inline nodes/edges
        nodeset_id: NEW_NODESET_ID,
        // Always include the current nodes and edges from the editor
        nodes: nodes || [],
        edges: edges || [],
        // Active Command data (for testing with specific active command states)
        active_command: activeCommandIsDefined ? {
          is_defined: true,
          is_on: activeCommandIsOn,
          temperature: getActiveCommandTemperature(),
          mode: getActiveCommandModeInt(),
          fan_speed: getActiveCommandFanSpeed(),
          swing: getActiveCommandSwing(),
          is_powerful: activeCommandIsPowerful,
        } : null,
      };
      
      const response = await fetch('/api/simulator/evaluate', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(payload),
      });
      
      const result = await response.json();
      
      if (result.success && result.data) {
        simulationResult = result.data;
        if (!result.data.success) {
          errorMessage = result.data.error || 'Simulation failed';
        }
      } else {
        errorMessage = result.error || 'Failed to evaluate workflow';
      }
    } catch (e) {
      console.error('Error evaluating workflow:', e);
      errorMessage = 'Failed to connect to server';
    } finally {
      evaluating = false;
    }
  }

  // Handle resize
  function startResize(e) {
    isResizing = true;
    e.preventDefault();
  }

  function handleMouseMove(e) {
    if (!isResizing) return;
    const newHeight = window.innerHeight - e.clientY;
    drawerHeight = Math.max(100, Math.min(600, newHeight));
  }

  function handleMouseUp() {
    isResizing = false;
  }

  // When device changes, update device-specific inputs
  async function onDeviceChange() {
    // Re-fetch live inputs for the new device
    await loadLiveInputs();
  }

  onMount(() => {
    // Load devices list on mount
    loadLiveInputs();
    
    // Add global mouse event listeners for resize
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  });

  // Get mode display string
  function getModeDisplay(mode) {
    switch (mode) {
      case 'Colder': return '❄️ Cooling';
      case 'Warmer': return '🔥 Heating';
      case 'Off': return '⏹️ Off';
      case 'NoChange': return '➡️ No Change';
      default: return mode;
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div 
  class="simulator-drawer" 
  class:open={isOpen}
  class:resizing={isResizing}
  style="height: {isOpen ? drawerHeight : 40}px"
  role="region"
  aria-label="Simulator drawer"
>
  <!-- Resize handle -->
  {#if isOpen}
    <div 
      class="resize-handle" 
      onmousedown={startResize}
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize drawer"
    ></div>
  {/if}
  
  <!-- Header -->
  <div class="drawer-header">
    <button 
      class="toggle-btn" 
      onclick={() => isOpen = !isOpen}
      aria-expanded={isOpen}
    >
      {isOpen ? '▼' : '▲'} Simulator
    </button>
    
    {#if isOpen}
      <div class="header-actions">
        <button 
          class="btn btn-live" 
          onclick={loadLiveInputs}
          disabled={loading}
        >
          {loading ? '⏳' : '📡'} Get Live Inputs
        </button>
        <button 
          class="btn btn-evaluate" 
          onclick={evaluate}
          disabled={evaluating || !selectedDevice}
        >
          {evaluating ? '⏳' : '▶️'} Evaluate
        </button>
      </div>
    {/if}
  </div>
  
  {#if isOpen}
    <div class="drawer-content">
      <!-- Inputs Panel -->
      <div class="inputs-panel">
        <h4>Simulation Inputs</h4>
        
        <div class="input-grid">
          <!-- Device Selection -->
          <div class="input-group">
            <label for="device">Device</label>
            <select 
              id="device" 
              bind:value={selectedDevice}
              onchange={onDeviceChange}
            >
              {#if devices.length === 0}
                <option value="">Loading...</option>
              {:else}
                {#each devices as device}
                  <option value={device}>{device}</option>
                {/each}
              {/if}
            </select>
          </div>
          
          <!-- Indoor Temperature (float) -->
          <div class="input-group">
            <label for="temperature">Indoor Temp (°C)</label>
            <input 
              type="text" 
              id="temperature" 
              bind:value={temperatureStr}
              class:invalid={!isValidFloat(temperatureStr)}
              placeholder="e.g. 22.5"
            />
          </div>
          
          <!-- Auto Mode -->
          <div class="input-group checkbox-group">
            <label for="autoMode">
              <input 
                type="checkbox" 
                id="autoMode" 
                bind:checked={isAutoMode}
              />
              Auto Mode
            </label>
          </div>
          
          <!-- Solar Production (integer) -->
          <div class="input-group">
            <label for="solar">Solar (W)</label>
            <input 
              type="text" 
              id="solar" 
              bind:value={solarProductionStr}
              class:invalid={!isValidInteger(solarProductionStr)}
              placeholder="e.g. 1000"
            />
          </div>
          
          <!-- Outdoor Temperature (float) -->
          <div class="input-group">
            <label for="outdoorTemp">Outdoor Temp (°C)</label>
            <input 
              type="text" 
              id="outdoorTemp" 
              bind:value={outdoorTempStr}
              class:invalid={!isValidFloat(outdoorTempStr)}
              placeholder="e.g. 20.0"
            />
          </div>
          
          <!-- Avg Next 24h Outdoor Temp (float) -->
          <div class="input-group" title="Average outdoor temperature forecasted for the next 24 hours">
            <label for="avgOutdoor">Avg Next 24h (°C)</label>
            <input 
              type="text" 
              id="avgOutdoor" 
              bind:value={avgNext24hOutdoorTempStr}
              class:invalid={!isValidFloat(avgNext24hOutdoorTempStr)}
              placeholder="e.g. 20.0"
            />
          </div>
          
          <!-- User Is Home -->
          <div class="input-group checkbox-group">
            <label for="userHome">
              <input 
                type="checkbox" 
                id="userHome" 
                bind:checked={userIsHome}
              />
              User Is Home
            </label>
          </div>
          
          <!-- PIR Detected -->
          <div class="input-group checkbox-group">
            <label for="pirDetected">
              <input 
                type="checkbox" 
                id="pirDetected" 
                bind:checked={pirDetected}
              />
              PIR Detected
            </label>
          </div>
          
          <!-- PIR Minutes Ago (integer) -->
          {#if pirDetected}
            <div class="input-group">
              <label for="pirMinutes">PIR Minutes Ago</label>
              <input 
                type="text" 
                id="pirMinutes" 
                bind:value={pirMinutesAgoStr}
                class:invalid={!isValidInteger(pirMinutesAgoStr)}
                placeholder="e.g. 5"
              />
            </div>
          {/if}
          
          <!-- Last Change Minutes (integer) -->
          <div class="input-group">
            <label for="lastChange">Last Change (min)</label>
            <input 
              type="text" 
              id="lastChange" 
              bind:value={lastChangeMinutesStr}
              class:invalid={!isValidInteger(lastChangeMinutesStr)}
              placeholder="e.g. 60"
            />
          </div>
          
          <!-- Net Power (integer, positive = consuming, negative = producing) -->
          <div class="input-group">
            <label for="netPower">Net Power (W)</label>
            <input 
              type="text" 
              id="netPower" 
              bind:value={netPowerWattStr}
              class:invalid={!isValidInteger(netPowerWattStr)}
              placeholder="e.g. -500"
            />
          </div>
        </div>
        
        <!-- Active Command Section -->
        <div class="active-command-section">
          <h5>Active Command (Last Sent)</h5>
          
          <div class="input-group checkbox-group">
            <label for="activeCommandDefined">
              <input 
                type="checkbox" 
                id="activeCommandDefined" 
                bind:checked={activeCommandIsDefined}
              />
              Is Defined
            </label>
          </div>
          
          {#if activeCommandIsDefined}
            <div class="active-command-grid">
              <!-- Is On -->
              <div class="input-group checkbox-group">
                <label for="activeCommandOn">
                  <input 
                    type="checkbox" 
                    id="activeCommandOn" 
                    bind:checked={activeCommandIsOn}
                  />
                  AC Is On
                </label>
              </div>
              
              <!-- Mode -->
              <div class="input-group">
                <label for="activeCommandMode">Mode</label>
                <select 
                  id="activeCommandMode" 
                  bind:value={activeCommandMode}
                >
                  <option value="Heat">Heat</option>
                  <option value="Cool">Cool</option>
                  <option value="Off">Off</option>
                </select>
              </div>
              
              <!-- Temperature -->
              <div class="input-group">
                <label for="activeCommandTemp">Temperature (°C)</label>
                <input 
                  type="text" 
                  id="activeCommandTemp" 
                  bind:value={activeCommandTemperatureStr}
                  class:invalid={!isValidFloat(activeCommandTemperatureStr)}
                  placeholder="e.g. 22.0"
                />
              </div>
              
              <!-- Fan Speed -->
              <div class="input-group">
                <label for="activeCommandFan">Fan Speed (0-5)</label>
                <input 
                  type="text" 
                  id="activeCommandFan" 
                  bind:value={activeCommandFanSpeedStr}
                  class:invalid={!isValidInteger(activeCommandFanSpeedStr)}
                  placeholder="0=Auto"
                />
              </div>
              
              <!-- Swing -->
              <div class="input-group">
                <label for="activeCommandSwing">Swing (0/1)</label>
                <input 
                  type="text" 
                  id="activeCommandSwing" 
                  bind:value={activeCommandSwingStr}
                  class:invalid={!isValidInteger(activeCommandSwingStr)}
                  placeholder="0=Off"
                />
              </div>
              
              <!-- Is Powerful -->
              <div class="input-group checkbox-group">
                <label for="activeCommandPowerful">
                  <input 
                    type="checkbox" 
                    id="activeCommandPowerful" 
                    bind:checked={activeCommandIsPowerful}
                  />
                  Powerful Mode
                </label>
              </div>
            </div>
          {/if}
        </div>
      </div>
      
      <!-- Results Panel -->
      <div class="results-panel">
        <h4>Simulation Result</h4>
        
        {#if errorMessage}
          <div class="error-message">{errorMessage}</div>
        {/if}
        
        {#if simulationResult}
          {#if simulationResult.success && simulationResult.plan}
            <div class="result-card">
              <div class="result-section">
                <h5>Decision</h5>
                <div class="result-row">
                  <span class="result-label">Mode:</span>
                  <span class="result-value mode-{simulationResult.plan.mode.toLowerCase()}">
                    {getModeDisplay(simulationResult.plan.mode)}
                  </span>
                </div>
                <div class="result-row">
                  <span class="result-label">Reason:</span>
                  <span class="result-value cause-label">{simulationResult.plan.cause_label}</span>
                </div>
              </div>
              
              {#if simulationResult.ac_state}
                <div class="result-section">
                  <h5>AC State</h5>
                  <div class="result-row">
                    <span class="result-label">Power:</span>
                    <span class="result-value">{simulationResult.ac_state.is_on ? '🟢 On' : '🔴 Off'}</span>
                  </div>
                  {#if simulationResult.ac_state.is_on}
                    <div class="result-row">
                      <span class="result-label">Mode:</span>
                      <span class="result-value">{simulationResult.ac_state.mode}</span>
                    </div>
                    <div class="result-row">
                      <span class="result-label">Target Temp:</span>
                      <span class="result-value">{simulationResult.ac_state.temperature}°C</span>
                    </div>
                    <div class="result-row">
                      <span class="result-label">Fan Speed:</span>
                      <span class="result-value">{simulationResult.ac_state.fan_speed === 0 ? 'Auto' : simulationResult.ac_state.fan_speed}</span>
                    </div>
                    <div class="result-row">
                      <span class="result-label">Powerful:</span>
                      <span class="result-value">{simulationResult.ac_state.powerful_mode ? 'Yes' : 'No'}</span>
                    </div>
                  {/if}
                </div>
              {/if}
              
              <div class="result-section cause-description">
                <h5>Explanation</h5>
                <p>{simulationResult.plan.cause_description}</p>
              </div>
            </div>
          {:else if simulationResult.error}
            <div class="error-message">{simulationResult.error}</div>
          {/if}
        {:else}
          <div class="no-result">
            <p>Click "Evaluate" to simulate the workflow with the current inputs.</p>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .simulator-drawer {
    position: relative;
    background: #2d2d2d;
    border-top: 2px solid #404040;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .simulator-drawer.resizing {
    user-select: none;
  }

  .resize-handle {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 8px;
    cursor: ns-resize;
    background: transparent;
    z-index: 10;
  }

  .resize-handle:hover {
    background: rgba(0, 188, 212, 0.3);
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0rem 1rem;
    background: #252525;
    border-bottom: 1px solid #404040;
    min-height: 40px;
  }

  .toggle-btn {
    background: transparent;
    border: none;
    color: #e0e0e0;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    transition: background 0.2s;
  }

  .toggle-btn:hover {
    background: #3d3d3d;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .btn {
    padding: 0.375rem 0.75rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 500;
    transition: all 0.2s;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-live {
    background: #757575;
    color: white;
  }

  .btn-live:hover:not(:disabled) {
    background: #888;
  }

  .btn-evaluate {
    background: #00BCD4;
    color: white;
  }

  .btn-evaluate:hover:not(:disabled) {
    background: #00ACC1;
  }

  .drawer-content {
    flex: 1;
    display: flex;
    overflow: hidden;
    gap: 1rem;
    padding: 1rem;
  }

  .inputs-panel,
  .results-panel {
    flex: 1;
    background: #1a1a1a;
    border-radius: 8px;
    padding: 1rem;
    overflow-y: auto;
  }

  .inputs-panel h4,
  .results-panel h4 {
    margin: 0 0 0.75rem 0;
    color: #e0e0e0;
    font-size: 1rem;
    border-bottom: 1px solid #404040;
    padding-bottom: 0.5rem;
  }

  .error-message {
    background: rgba(244, 67, 54, 0.2);
    border: 1px solid #F44336;
    color: #F44336;
    padding: 0.5rem;
    border-radius: 4px;
    margin-bottom: 0.75rem;
    font-size: 0.85rem;
  }

  .input-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 0.75rem;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .input-group label {
    font-size: 0.75rem;
    color: #aaa;
    font-weight: 500;
  }

  .input-group input[type="text"],
  .input-group select {
    padding: 0.375rem 0.5rem;
    border: 1px solid #404040;
    border-radius: 4px;
    background: #2d2d2d;
    color: #e0e0e0;
    font-size: 0.875rem;
  }

  .input-group input[type="text"]:focus,
  .input-group select:focus {
    outline: none;
    border-color: #00BCD4;
  }

  /* Invalid input styling - red border for validation errors */
  .input-group input.invalid {
    border-color: #F44336;
    background: rgba(244, 67, 54, 0.1);
  }

  .input-group input.invalid:focus {
    border-color: #F44336;
    box-shadow: 0 0 0 1px rgba(244, 67, 54, 0.3);
  }

  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    font-size: 0.875rem;
    color: #e0e0e0;
  }

  .checkbox-group input[type="checkbox"] {
    width: 1rem;
    height: 1rem;
    cursor: pointer;
  }

  .no-result {
    color: #888;
    text-align: center;
    padding: 2rem;
  }

  .no-result p {
    margin: 0;
  }

  .result-card {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .result-section {
    background: #2d2d2d;
    border-radius: 6px;
    padding: 0.75rem;
  }

  .result-section h5 {
    margin: 0 0 0.5rem 0;
    font-size: 0.8rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .result-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.25rem 0;
    font-size: 0.875rem;
  }

  .result-label {
    color: #aaa;
  }

  .result-value {
    color: #e0e0e0;
    font-weight: 500;
  }

  .result-value.mode-colder {
    color: #4ECDC4;
  }

  .result-value.mode-warmer {
    color: #FF6B6B;
  }

  .result-value.mode-off {
    color: #888;
  }

  .result-value.mode-nochange {
    color: #aaa;
  }

  .cause-label {
    background: #404040;
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
    font-size: 0.8rem;
  }

  .cause-description {
    font-size: 0.85rem;
  }

  .cause-description p {
    margin: 0;
    color: #bbb;
    line-height: 1.4;
  }

  /* Active Command Section Styles */
  .active-command-section {
    margin-top: 1rem;
    padding-top: 0.75rem;
    border-top: 1px solid #404040;
  }

  .active-command-section h5 {
    margin: 0 0 0.5rem 0;
    color: #aaa;
    font-size: 0.85rem;
    font-weight: 500;
  }

  .active-command-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 0.5rem;
    margin-top: 0.5rem;
    padding: 0.5rem;
    background: #252525;
    border-radius: 4px;
  }

  .active-command-grid .input-group {
    gap: 0.15rem;
  }

  .active-command-grid .input-group label {
    font-size: 0.7rem;
  }

  .active-command-grid .input-group input[type="text"],
  .active-command-grid .input-group select {
    padding: 0.25rem 0.4rem;
    font-size: 0.8rem;
  }

  .active-command-grid .checkbox-group label {
    font-size: 0.8rem;
  }

  .active-command-grid .checkbox-group input[type="checkbox"] {
    width: 0.875rem;
    height: 0.875rem;
  }
</style>
