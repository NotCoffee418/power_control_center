<script>
  import { onMount, onDestroy } from 'svelte';
  import { format } from 'timeago.js';

  // Constants for time conversions
  const SECONDS_TO_MILLISECONDS = 1000;
  const SECONDS_PER_MINUTE = 60;
  const LOADING_TIMEOUT_MS = 10000; // 10 seconds timeout before showing error

  let dashboardData = $state(null);
  let recentCommands = $state([]);
  let loading = $state(true);
  let commandsLoading = $state(true);
  let error = $state(null);
  let lastUpdate = $state(null);
  let refreshInterval = null;
  let loadingTimeoutId = null;
  
  // User Is Home control state
  let homeOverrideHours = $state(4);
  let settingHomeOverride = $state(false);
  let homeOverrideError = $state(null);

  async function fetchDashboardData() {
    try {
      const response = await fetch('/api/dashboard/status');
      const result = await response.json();
      
      if (result.success) {
        dashboardData = result.data;
        error = null;
        lastUpdate = new Date();
        // Clear the timeout since we got data successfully
        if (loadingTimeoutId) {
          clearTimeout(loadingTimeoutId);
          loadingTimeoutId = null;
        }
      } else {
        error = result.error || 'Failed to fetch dashboard data';
      }
    } catch (e) {
      error = `Error fetching data: ${e.message}`;
      console.error('Dashboard fetch error:', e);
    } finally {
      loading = false;
    }
  }

  async function fetchRecentCommands() {
    try {
      const response = await fetch('/api/dashboard/recent-commands?page=1&per_page=10');
      const result = await response.json();
      
      if (result.success) {
        recentCommands = result.data.commands;
      } else {
        console.error('Failed to fetch recent commands:', result.error);
      }
    } catch (e) {
      console.error('Error fetching recent commands:', e);
    } finally {
      commandsLoading = false;
    }
  }

  onMount(() => {
    fetchDashboardData();
    fetchRecentCommands();
    // Set timeout to show error if loading takes too long
    loadingTimeoutId = setTimeout(() => {
      if (loading && !dashboardData && !error) {
        error = 'Connection timed out. Please check your network.';
        loading = false;
      }
    }, LOADING_TIMEOUT_MS);
    // Refresh every 10 seconds
    refreshInterval = setInterval(() => {
      fetchDashboardData();
      fetchRecentCommands();
    }, 10000);
  });

  onDestroy(() => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
    if (loadingTimeoutId) {
      clearTimeout(loadingTimeoutId);
    }
  });

  function formatTemperature(temp) {
    return temp != null ? `${temp.toFixed(1)}°C` : 'N/A';
  }

  function formatTrend(trend) {
    if (trend == null) return 'N/A';
    const sign = trend >= 0 ? '+' : '';
    return `${sign}${trend.toFixed(1)}°C`;
  }

  function getTrendIndicator(trend) {
    if (trend == null) return '→';
    if (trend > 1) return '↑↑';
    if (trend > 0.2) return '↑';
    if (trend < -1) return '↓↓';
    if (trend < -0.2) return '↓';
    return '→';
  }

  function formatFanSpeed(fanSpeed) {
    if (fanSpeed == null) return 'N/A';
    return fanSpeed === 0 ? 'Auto' : fanSpeed.toString();
  }

  function formatNetPower(watts) {
    if (watts == null) return 'N/A';
    // Invert the sign: negative for consuming (importing), positive for producing (exporting)
    const invertedWatts = -watts;
    return `${invertedWatts} W`;
  }

  function formatTimestamp(timestamp) {
    if (timestamp == null) return 'N/A';
    const date = new Date(timestamp * 1000);
    return date.toLocaleString();
  }

  function formatActionType(actionType) {
    if (actionType === 'on') return 'On';
    if (actionType === 'off') return 'Off';
    if (actionType === 'toggle-powerful') return 'Powerful';
    return actionType;
  }

  function getActionTypeClass(actionType) {
    if (actionType === 'on') return 'action-on';
    if (actionType === 'off') return 'action-off';
    if (actionType === 'toggle-powerful') return 'action-powerful';
    return '';
  }

  /**
   * Calculate the average indoor temperature across all devices
   * @param {Array} devices - Array of device objects with indoor_temperature property
   * @returns {number|null} The average temperature, or null if no valid temperatures exist
   */
  function calculateAverageIndoorTemp(devices) {
    if (!devices || devices.length === 0) return null;
    
    const validTemps = devices
      .map(d => d.indoor_temperature)
      .filter(t => t !== null && t !== undefined);
    
    if (validTemps.length === 0) return null;
    
    const sum = validTemps.reduce((acc, temp) => acc + temp, 0);
    return sum / validTemps.length;
  }

  /**
   * Format PIR detection time as relative time (e.g., "5 minutes ago")
   * @param {number|null} timestamp - Unix timestamp in seconds
   * @returns {string|null} Formatted time string or null if no detection
   */
  function formatPirDetectionTime(timestamp) {
    if (!timestamp) return null;
    return format(timestamp * SECONDS_TO_MILLISECONDS);
  }

  /**
   * Check if PIR detection is recent based on timeout configuration
   * @param {number|null} timestamp - Unix timestamp in seconds
   * @param {number} timeoutMinutes - PIR timeout in minutes
   * @returns {boolean} True if recent, false otherwise
   */
  function isPirDetectionRecent(timestamp, timeoutMinutes) {
    if (!timestamp || !timeoutMinutes) return false;
    const now = Date.now() / SECONDS_TO_MILLISECONDS;
    const minutesAgo = (now - timestamp) / SECONDS_PER_MINUTE;
    return minutesAgo < timeoutMinutes;
  }

  /**
   * Format the home override status message
   * @param {number|null} timestamp - Unix timestamp in seconds when override expires
   * @returns {string} Human readable status message
   */
  function formatHomeOverrideStatus(timestamp) {
    if (!timestamp) {
      return 'No home time override';
    }
    const date = new Date(timestamp * SECONDS_TO_MILLISECONDS);
    return `Home until ${date.toLocaleString()}`;
  }

  /**
   * Set user home override for the specified number of hours
   */
  async function setHomeOverride() {
    if (homeOverrideHours < 1 || homeOverrideHours > 168) {
      homeOverrideError = 'Hours must be between 1 and 168';
      return;
    }
    
    settingHomeOverride = true;
    homeOverrideError = null;
    
    try {
      const response = await fetch('/api/user-home/set', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ hours: homeOverrideHours }),
      });
      
      const result = await response.json();
      
      if (result.success) {
        // Refresh dashboard data immediately to show the update
        await fetchDashboardData();
      } else {
        homeOverrideError = result.error || 'Failed to set home override';
      }
    } catch (e) {
      homeOverrideError = `Error: ${e.message}`;
      console.error('Error setting home override:', e);
    } finally {
      settingHomeOverride = false;
    }
  }

  /**
   * Clear user home override
   */
  async function clearHomeOverride() {
    settingHomeOverride = true;
    homeOverrideError = null;
    
    try {
      const response = await fetch('/api/user-home/clear', {
        method: 'POST',
      });
      
      const result = await response.json();
      
      if (result.success) {
        // Refresh dashboard data immediately to show the update
        await fetchDashboardData();
      } else {
        homeOverrideError = result.error || 'Failed to clear home override';
      }
    } catch (e) {
      homeOverrideError = `Error: ${e.message}`;
      console.error('Error clearing home override:', e);
    } finally {
      settingHomeOverride = false;
    }
  }
</script>

<div class="dashboard">
  <header>
    <div class="header-content">
      <div class="title-row">
        <img src="/icon.png" alt="" role="presentation" class="site-icon" onerror={(e) => e.target.style.display='none'} />
        <h1>Power Control Center</h1>
      </div>
      <a href="/nodes" class="nav-link">Open Node Editor</a>
    </div>
    {#if lastUpdate}
      <p class="last-update">Last updated: {lastUpdate.toLocaleTimeString()}</p>
    {/if}
  </header>

  {#if loading}
    <div class="loading">
      <div class="loading-spinner"></div>
      <p>Loading dashboard data...</p>
    </div>
  {:else if error}
    <div class="error">{error}</div>
  {:else if dashboardData}
    <div class="grid-container">
      <!-- Environmental & Power Data Section -->
      <div class="section environmental-power">
        <h2>Environmental & Power</h2>
        <div class="data-grid">
          <div class="data-item">
            <span class="label">Avg Indoor Temp</span>
            <span class="value large">{formatTemperature(calculateAverageIndoorTemp(dashboardData.devices))}</span>
          </div>
          <div class="data-item">
            <span class="label">Outdoor Temp</span>
            <span class="value large">{formatTemperature(dashboardData.outdoor_temp)}</span>
          </div>
          <div class="data-item">
            <span class="label">Forecast Trend</span>
            <span class="value">
              {getTrendIndicator(dashboardData.outdoor_temp_trend)}
              {formatTrend(dashboardData.outdoor_temp_trend)}
            </span>
          </div>
          <div class="data-item">
            <span class="label">Raw Solar Production</span>
            <span class="value large">
              {dashboardData.solar_production_watts != null 
                ? `${dashboardData.solar_production_watts} W` 
                : 'N/A'}
            </span>
          </div>
          <div class="data-item net-power" class:importing={dashboardData.net_power_w > 0} class:exporting={dashboardData.net_power_w < 0}>
            <span class="label">Net Power</span>
            <span class="value large">{formatNetPower(dashboardData.net_power_w)}</span>
          </div>
          <div class="data-item user-home" class:home={dashboardData.user_is_home} class:away={!dashboardData.user_is_home}>
            <span class="label">User Status</span>
            <span class="value large">{dashboardData.user_is_home ? '🏠 Home' : '🚶 Away'}</span>
          </div>
        </div>
      </div>

      <!-- User Is Home Control Section -->
      <div class="section user-home-control">
        <h2>User Is Home Control</h2>
        <div class="control-container">
          <div class="control-input-row">
            <label for="home-hours">Set user is home for next hours:</label>
            <input 
              id="home-hours"
              type="number" 
              min="1" 
              max="168" 
              bind:value={homeOverrideHours}
              disabled={settingHomeOverride}
              class="hours-input"
            />
            <button 
              onclick={setHomeOverride}
              disabled={settingHomeOverride}
              class="btn btn-set"
            >
              {settingHomeOverride ? 'Setting...' : 'Set'}
            </button>
            <button 
              onclick={clearHomeOverride}
              disabled={settingHomeOverride}
              class="btn btn-clear"
            >
              {settingHomeOverride ? 'Clearing...' : 'Clear'}
            </button>
          </div>
          
          {#if homeOverrideError}
            <div class="control-error">{homeOverrideError}</div>
          {/if}
          
          <div class="control-status" class:has-override={dashboardData.user_home_override_until}>
            {formatHomeOverrideStatus(dashboardData.user_home_override_until)}
          </div>
        </div>
      </div>

      <!-- AC Devices Section -->
      <div class="section devices">
        <h2>AC Devices</h2>
        <div class="device-cards">
          {#each dashboardData.devices as device}
            <div class="device-card" class:active={device.is_automatic_mode}>
              <div class="device-header">
                <h3>{device.name}</h3>
                <span class="status-badge" class:auto={device.is_automatic_mode} class:manual={!device.is_automatic_mode}>
                  {device.is_automatic_mode ? 'Auto' : 'Manual'}
                </span>
              </div>
              
              {#if device.is_automatic_mode}
                <div class="device-details">
                  <div class="detail-row">
                    <span class="detail-label">Mode:</span>
                    <span class="detail-value mode-{device.mode}">
                      {device.mode?.toUpperCase() || 'N/A'}
                    </span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">Target Temp:</span>
                    <span class="detail-value">{formatTemperature(device.temperature_setpoint)}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">Fan Speed:</span>
                    <span class="detail-value">{formatFanSpeed(device.fan_speed)}</span>
                  </div>
                  {#if device.powerful_mode}
                    <div class="detail-row">
                      <span class="powerful-badge">⚡ POWERFUL</span>
                    </div>
                  {/if}
                </div>
              {/if}

              <div class="device-temp">
                <span class="temp-label">Indoor Temp</span>
                <span class="temp-value">{formatTemperature(device.indoor_temperature)}</span>
              </div>

              <!-- PIR Detection Display -->
              {#if device.last_pir_detection}
                {@const isRecent = isPirDetectionRecent(device.last_pir_detection, dashboardData.pir_timeout_minutes)}
                <div class="pir-detection" class:recent={isRecent} class:not-recent={!isRecent}>
                  <span class="pir-label">Last PIR:</span>
                  <span class="pir-value">{formatPirDetectionTime(device.last_pir_detection)}</span>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>

      <!-- Recent Commands Section -->
      <div class="section recent-commands">
        <h2>Recent AC Commands</h2>
        {#if commandsLoading}
          <div class="loading-commands">Loading commands...</div>
        {:else if recentCommands.length === 0}
          <div class="no-commands">No commands recorded yet</div>
        {:else}
          <div class="commands-table-wrapper">
            <table class="commands-table">
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Device</th>
                  <th>Temp</th>
                  <th>Net / Solar</th>
                  <th>Action</th>
                  <th>Cause</th>
                </tr>
              </thead>
              <tbody>
                {#each recentCommands as command}
                  <tr>
                    <td class="timestamp" data-label="Time">{formatTimestamp(command.action_timestamp)}</td>
                    <td class="device-name" data-label="Device">{command.device_identifier}</td>
                    <td class="measured-temp" data-label="Temperature">
                      {command.measured_temperature ? `${command.measured_temperature.toFixed(1)}°C` : '—'}
                    </td>
                    <td class="measured-power" data-label="Net / Solar">
                      <div class="power-values">
                        <div class="power-row">
                          <span class="power-label">Net:</span>
                          <span class="power-value">{command.measured_net_power_watt != null ? `${command.measured_net_power_watt} W` : '—'}</span>
                        </div>
                        <div class="power-row">
                          <span class="power-label">Solar:</span>
                          <span class="power-value">{command.measured_solar_production_watt != null ? `${command.measured_solar_production_watt} W` : '—'}</span>
                        </div>
                      </div>
                    </td>
                    <td class="action-cell" data-label="Action">
                      <span class="action-type {getActionTypeClass(command.action_type)}">
                        {formatActionType(command.action_type)}
                      </span>
                    </td>
                    <td class="cause-cell" data-label="Cause">
                      {#if command.cause_label && command.cause_label !== 'Undefined'}
                        <span class="cause-badge" title={command.cause_description}>
                          {command.cause_label}
                        </span>
                      {:else}
                        <span>—</span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .dashboard {
    width: 100%;
    max-width: 1400px;
    margin: 0 auto;
    padding: 1rem;
    box-sizing: border-box;
  }

  header {
    text-align: center;
    margin-bottom: 2rem;
  }

  .header-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .site-icon {
    width: 48px;
    height: 48px;
    object-fit: contain;
    border-radius: 8px;
  }

  h1 {
    margin: 0;
    font-size: 2.5rem;
    color: #e0e0e0;
  }

  .nav-link {
    display: inline-block;
    padding: 0.5rem 1rem;
    background: #757575;
    border: 1px solid #757575;
    border-radius: 8px;
    color: white;
    text-decoration: none;
    font-weight: 500;
    transition: all 0.3s ease;
  }

  .nav-link:hover {
    background: #888;
    transform: translateY(-2px);
  }

  .last-update {
    margin: 0.5rem 0 0;
    font-size: 0.875rem;
    opacity: 0.7;
  }

  .loading, .error {
    text-align: center;
    padding: 2rem;
    font-size: 1.2rem;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 4px solid rgba(160, 160, 160, 0.2);
    border-top-color: #a0a0a0;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error {
    color: #ff6b6b;
  }

  .grid-container {
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  .section {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    padding: 1.5rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .section h2 {
    margin: 0 0 1rem 0;
    font-size: 1.5rem;
    color: #a0a0a0;
  }

  .environmental-power .data-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1.5rem;
  }

  @media (min-width: 768px) {
    .environmental-power .data-grid {
      grid-template-columns: repeat(3, 1fr);
    }
  }

  .data-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 1rem;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 8px;
  }

  .data-item .label {
    font-size: 0.875rem;
    opacity: 0.7;
    margin-bottom: 0.5rem;
  }

  .data-item .value {
    font-size: 1.5rem;
    font-weight: 600;
  }

  .data-item .value.large {
    font-size: 2rem;
  }

  .data-item.net-power.importing {
    background: rgba(255, 107, 107, 0.1);
    border: 1px solid rgba(255, 107, 107, 0.3);
  }

  .data-item.net-power.exporting {
    background: rgba(81, 207, 102, 0.1);
    border: 1px solid rgba(81, 207, 102, 0.3);
  }

  .data-item.user-home.home {
    background: rgba(81, 207, 102, 0.1);
    border: 1px solid rgba(81, 207, 102, 0.3);
  }

  .data-item.user-home.away {
    background: rgba(160, 160, 160, 0.1);
    border: 1px solid rgba(160, 160, 160, 0.3);
  }

  /* User Is Home Control Section */
  .user-home-control {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    padding: 1.5rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .control-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .control-input-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .control-input-row label {
    font-size: 1rem;
    color: #e0e0e0;
  }

  .hours-input {
    width: 80px;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 6px;
    color: #e0e0e0;
    font-size: 1rem;
    text-align: center;
  }

  .hours-input:focus {
    outline: none;
    border-color: #a0a0a0;
    background: rgba(255, 255, 255, 0.15);
  }

  .hours-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn {
    padding: 0.5rem 1.5rem;
    border: none;
    border-radius: 6px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.3s ease;
    color: white;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-set {
    background: linear-gradient(135deg, #4caf50 0%, #45a049 100%);
  }

  .btn-set:hover:not(:disabled) {
    background: linear-gradient(135deg, #45a049 0%, #3d8b40 100%);
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(76, 175, 80, 0.3);
  }

  .btn-clear {
    background: linear-gradient(135deg, #ff6b6b 0%, #ee5a52 100%);
  }

  .btn-clear:hover:not(:disabled) {
    background: linear-gradient(135deg, #ee5a52 0%, #d84a41 100%);
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(255, 107, 107, 0.3);
  }

  .control-error {
    color: #ff6b6b;
    font-size: 0.875rem;
    padding: 0.5rem;
    background: rgba(255, 107, 107, 0.1);
    border: 1px solid rgba(255, 107, 107, 0.3);
    border-radius: 6px;
  }

  .control-status {
    padding: 1rem;
    background: rgba(160, 160, 160, 0.1);
    border: 1px solid rgba(160, 160, 160, 0.3);
    border-radius: 8px;
    text-align: center;
    font-size: 1.1rem;
    font-weight: 600;
    color: #a0a0a0;
  }

  .control-status.has-override {
    background: rgba(81, 207, 102, 0.1);
    border: 1px solid rgba(81, 207, 102, 0.3);
    color: #51cf66;
  }

  .device-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1.5rem;
  }

  .device-card {
    background: rgba(255, 255, 255, 0.03);
    border: 2px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    padding: 1.25rem;
    transition: all 0.3s ease;
  }

  .device-card.active {
    border-color: #a0a0a0;
    box-shadow: 0 0 20px rgba(160, 160, 160, 0.2);
  }

  .device-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .device-header h3 {
    margin: 0;
    font-size: 1.25rem;
  }

  .status-badge {
    padding: 0.25rem 0.75rem;
    border-radius: 20px;
    font-size: 0.75rem;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.5);
  }

  .status-badge.auto {
    background: #4caf50;
    color: white;
  }

  .status-badge.manual {
    background: #ffa726;
    color: white;
  }

  .device-details {
    margin-bottom: 1rem;
    padding: 0.75rem;
    background: rgba(255, 255, 255, 0.02);
    border-radius: 8px;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .detail-row:last-child {
    border-bottom: none;
  }

  .detail-label {
    font-size: 0.875rem;
    opacity: 0.7;
  }

  .detail-value {
    font-weight: 600;
  }

  .mode-heat {
    color: #ff6b6b;
  }

  .mode-cool {
    color: #4dabf7;
  }

  .powerful-badge {
    display: inline-block;
    padding: 0.25rem 0.75rem;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    border-radius: 20px;
    font-size: 0.875rem;
    font-weight: 600;
  }

  .device-temp {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    background: rgba(160, 160, 160, 0.1);
    border-radius: 8px;
    margin-top: 0.75rem;
  }

  .temp-label {
    font-size: 0.875rem;
    opacity: 0.8;
  }

  .temp-value {
    font-size: 1.25rem;
    font-weight: 600;
    color: #a0a0a0;
  }

  /* PIR Detection Styles */
  .pir-detection {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    border-radius: 8px;
    margin-top: 0.75rem;
    transition: all 0.3s ease;
  }

  .pir-detection.recent {
    background: rgba(255, 107, 107, 0.15);
    border: 1px solid rgba(255, 107, 107, 0.3);
  }

  .pir-detection.not-recent {
    background: rgba(66, 153, 225, 0.15);
    border: 1px solid rgba(66, 153, 225, 0.3);
  }

  .pir-label {
    font-size: 0.875rem;
    opacity: 0.8;
  }

  .pir-value {
    font-size: 0.95rem;
    font-weight: 600;
  }

  .pir-detection.recent .pir-value {
    color: #ff6b6b;
  }

  .pir-detection.not-recent .pir-value {
    color: #4299e1;
  }

  /* Recent Commands Section */
  .recent-commands {
    margin-top: 1rem;
  }

  .loading-commands, .no-commands {
    text-align: center;
    padding: 1.5rem;
    opacity: 0.7;
  }

  .commands-table-wrapper {
    overflow-x: auto;
    width: 100%;
  }

  .commands-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
    table-layout: fixed;
  }

  .commands-table thead {
    background: rgba(255, 255, 255, 0.05);
  }

  .commands-table th {
    padding: 0.75rem;
    text-align: left;
    font-weight: 600;
    border-bottom: 2px solid rgba(255, 255, 255, 0.1);
    white-space: nowrap;
  }

  .commands-table th:nth-child(1) { width: 15%; }  /* Time */
  .commands-table th:nth-child(2) { width: 15%; }  /* Device */
  .commands-table th:nth-child(3) { width: 12%; }  /* Temp */
  .commands-table th:nth-child(4) { width: 23%; }  /* Net / Solar */
  .commands-table th:nth-child(5) { width: 10%; }  /* Action */
  .commands-table th:nth-child(6) { width: 25%; }  /* Cause */
  /* Total: 100% to allow for borders and padding */

  .commands-table td {
    padding: 0.75rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .commands-table tbody tr:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .timestamp {
    white-space: nowrap;
    opacity: 0.8;
  }

  .device-name {
    font-weight: 600;
    color: #a0a0a0;
  }

  .action-cell {
    text-align: center;
  }

  .action-type {
    padding: 0.25rem 0.65rem;
    border-radius: 6px;
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    border: 1px solid transparent;
    transition: all 0.2s ease;
    min-width: 52px; /* Ensures badge width fits uppercase "POWERFUL" text */
  }

  .action-type::before {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .action-on {
    background: linear-gradient(135deg, #66bb6a 0%, #4caf50 100%);
    color: white;
    border-color: rgba(76, 175, 80, 0.3);
  }

  .action-on::before {
    background: #c8e6c9;
    box-shadow: 0 0 4px #c8e6c9;
  }

  .action-off {
    background: linear-gradient(135deg, #ff8a80 0%, #ff6b6b 100%);
    color: white;
    border-color: rgba(255, 107, 107, 0.3);
  }

  .action-off::before {
    background: #ffcdd2;
    box-shadow: 0 0 4px #ffcdd2;
  }

  .action-powerful {
    background: linear-gradient(135deg, #7c8cff 0%, #646cff 100%);
    color: white;
    border-color: rgba(100, 108, 255, 0.3);
  }

  .action-powerful::before {
    background: #e3f2fd;
    box-shadow: 0 0 4px #e3f2fd;
  }

  .measured-temp {
    text-align: right;
    opacity: 0.8;
  }

  .measured-power {
    text-align: right;
    opacity: 0.8;
  }

  .power-values {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    align-items: flex-end;
  }

  .power-row {
    display: flex;
    gap: 0.5rem;
    font-size: 0.8rem;
  }

  .power-label {
    opacity: 0.6;
    font-weight: 500;
  }

  .power-value {
    font-weight: 600;
  }

  .cause-cell {
    text-align: center;
  }

  .cause-badge {
    display: inline-block;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    background: rgba(255, 165, 0, 0.2);
    color: #ffa500;
    cursor: help;
    transition: all 0.2s ease;
  }

  .cause-badge:hover {
    background: rgba(255, 165, 0, 0.3);
    transform: scale(1.05);
  }

  @media (max-width: 768px) {
    h1 {
      font-size: 2rem;
    }

    .environmental-power .data-grid {
      grid-template-columns: 1fr;
    }

    .device-cards {
      grid-template-columns: 1fr;
    }

    .control-input-row {
      flex-direction: column;
      align-items: stretch;
    }

    .control-input-row label {
      text-align: center;
    }

    .hours-input {
      width: 100%;
    }

    .btn {
      width: 100%;
    }

    /* Mobile: Convert table to card layout */
    .commands-table-wrapper {
      overflow-x: visible;
    }

    .commands-table {
      display: block;
      table-layout: auto;
    }

    .commands-table thead {
      display: none;
    }

    .commands-table tbody {
      display: block;
    }

    .commands-table tr {
      display: block;
      margin-bottom: 1rem;
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 8px;
      padding: 1rem;
    }

    .commands-table tr:hover {
      background: rgba(255, 255, 255, 0.05);
    }

    .commands-table td {
      display: block;
      text-align: left;
      padding: 0.5rem 0;
      border-bottom: none;
    }

    .commands-table td::before {
      content: attr(data-label);
      display: block;
      font-weight: 600;
      font-size: 0.75rem;
      opacity: 0.6;
      margin-bottom: 0.25rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .timestamp {
      white-space: normal;
    }

    .action-cell,
    .cause-cell,
    .measured-temp,
    .measured-power {
      text-align: left;
    }

    .power-values {
      align-items: flex-start;
    }

    .commands-table td:last-child {
      border-bottom: none;
    }
  }
</style>
