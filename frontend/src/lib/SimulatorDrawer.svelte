<script>
  import { onMount } from 'svelte';

  // Props
  let { isOpen = $bindable(true) } = $props();

  // State
  let drawerHeight = $state(300);
  let isResizing = $state(false);
  let loading = $state(false);
  let evaluating = $state(false);

  // Input values
  let selectedDevice = $state('');
  let temperature = $state(22.0);
  let isAutoMode = $state(true);
  let solarProduction = $state(0);
  let outdoorTemp = $state(20.0);
  let avgNext12hOutdoorTemp = $state(20.0);
  let userIsHome = $state(true);
  let pirDetected = $state(false);
  let pirMinutesAgo = $state(0);

  // Available devices
  let devices = $state([]);
  
  // Simulation result
  let simulationResult = $state(null);
  let errorMessage = $state('');

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
            temperature = deviceData.temperature;
          }
          isAutoMode = deviceData.is_auto_mode;
          pirDetected = deviceData.pir_recently_triggered;
          pirMinutesAgo = deviceData.pir_minutes_ago ?? 0;
        }
        
        // Update environmental values
        if (data.solar_production !== null) {
          solarProduction = data.solar_production;
        }
        if (data.outdoor_temp !== null) {
          outdoorTemp = data.outdoor_temp;
        }
        if (data.avg_next_12h_outdoor_temp !== null) {
          avgNext12hOutdoorTemp = data.avg_next_12h_outdoor_temp;
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
    evaluating = true;
    errorMessage = '';
    simulationResult = null;
    
    try {
      const response = await fetch('/api/simulator/evaluate', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          device: selectedDevice,
          temperature: temperature,
          is_auto_mode: isAutoMode,
          solar_production: solarProduction,
          outdoor_temp: outdoorTemp,
          avg_next_12h_outdoor_temp: avgNext12hOutdoorTemp,
          user_is_home: userIsHome,
          pir_detected: pirDetected,
          pir_minutes_ago: pirMinutesAgo,
        }),
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

  // Get intensity display string
  function getIntensityDisplay(intensity) {
    switch (intensity) {
      case 'Low': return '🔋 Low';
      case 'Medium': return '⚡ Medium';
      case 'High': return '⚡⚡ High (Powerful)';
      default: return intensity;
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
        
        {#if errorMessage}
          <div class="error-message">{errorMessage}</div>
        {/if}
        
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
          
          <!-- Indoor Temperature -->
          <div class="input-group">
            <label for="temperature">Indoor Temp (°C)</label>
            <input 
              type="number" 
              id="temperature" 
              bind:value={temperature}
              step="0.5"
              min="-10"
              max="50"
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
          
          <!-- Solar Production -->
          <div class="input-group">
            <label for="solar">Solar (W)</label>
            <input 
              type="number" 
              id="solar" 
              bind:value={solarProduction}
              step="100"
              min="0"
              max="10000"
            />
          </div>
          
          <!-- Outdoor Temperature -->
          <div class="input-group">
            <label for="outdoorTemp">Outdoor Temp (°C)</label>
            <input 
              type="number" 
              id="outdoorTemp" 
              bind:value={outdoorTemp}
              step="0.5"
              min="-30"
              max="50"
            />
          </div>
          
          <!-- Avg Next 12h Outdoor Temp -->
          <div class="input-group">
            <label for="avgOutdoor">Avg Next 12h (°C)</label>
            <input 
              type="number" 
              id="avgOutdoor" 
              bind:value={avgNext12hOutdoorTemp}
              step="0.5"
              min="-30"
              max="50"
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
          
          <!-- PIR Minutes Ago -->
          {#if pirDetected}
            <div class="input-group">
              <label for="pirMinutes">PIR Minutes Ago</label>
              <input 
                type="number" 
                id="pirMinutes" 
                bind:value={pirMinutesAgo}
                step="1"
                min="0"
                max="120"
              />
            </div>
          {/if}
        </div>
      </div>
      
      <!-- Results Panel -->
      <div class="results-panel">
        <h4>Simulation Result</h4>
        
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
                  <span class="result-label">Intensity:</span>
                  <span class="result-value">
                    {getIntensityDisplay(simulationResult.plan.intensity)}
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
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    background: #2d2d2d;
    border-top: 2px solid #404040;
    z-index: 100;
    display: flex;
    flex-direction: column;
    transition: height 0.3s ease;
  }

  .simulator-drawer.resizing {
    transition: none;
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
  }

  .resize-handle:hover {
    background: rgba(0, 188, 212, 0.3);
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1rem;
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

  .input-group input[type="number"],
  .input-group select {
    padding: 0.375rem 0.5rem;
    border: 1px solid #404040;
    border-radius: 4px;
    background: #2d2d2d;
    color: #e0e0e0;
    font-size: 0.875rem;
  }

  .input-group input[type="number"]:focus,
  .input-group select:focus {
    outline: none;
    border-color: #00BCD4;
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
</style>
