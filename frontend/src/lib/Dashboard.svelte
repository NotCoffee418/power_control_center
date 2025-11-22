<script>
  import { onMount, onDestroy } from 'svelte';

  let dashboardData = $state(null);
  let recentCommands = $state([]);
  let loading = $state(true);
  let commandsLoading = $state(true);
  let error = $state(null);
  let lastUpdate = $state(null);
  let refreshInterval = null;

  async function fetchDashboardData() {
    try {
      const response = await fetch('/api/dashboard/status');
      const result = await response.json();
      
      if (result.success) {
        dashboardData = result.data;
        error = null;
        lastUpdate = new Date();
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

  function formatPower(kw) {
    if (kw == null) return 'N/A';
    return `${kw.toFixed(2)} kW`;
  }

  function formatNetPower(watts) {
    if (watts == null) return 'N/A';
    const kw = (watts / 1000).toFixed(2);
    if (watts < 0) {
      return `${Math.abs(kw)} kW (exporting)`;
    } else if (watts > 0) {
      return `${kw} kW (importing)`;
    } else {
      return '0.00 kW (balanced)';
    }
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
</script>

<div class="dashboard">
  <header>
    <h1>Power Control Center</h1>
    {#if lastUpdate}
      <p class="last-update">Last updated: {lastUpdate.toLocaleTimeString()}</p>
    {/if}
  </header>

  {#if loading}
    <div class="loading">Loading dashboard data...</div>
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
          <div class="data-item net-power" class:importing={dashboardData.net_power_w > 0} class:exporting={dashboardData.net_power_w < 0}>
            <span class="label">Net Power</span>
            <span class="value large">{formatNetPower(dashboardData.net_power_w)}</span>
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
                  <th>Action</th>
                  <th>Details</th>
                  <th>Temp</th>
                  <th>Net / Solar</th>
                  <th>Cause</th>
                </tr>
              </thead>
              <tbody>
                {#each recentCommands as command}
                  <tr>
                    <td class="timestamp">{formatTimestamp(command.action_timestamp)}</td>
                    <td class="device-name">{command.device_identifier}</td>
                    <td class="action-cell">
                      <span class="action-type {getActionTypeClass(command.action_type)}">
                        {formatActionType(command.action_type)}
                      </span>
                    </td>
                    <td class="command-details">
                      {#if command.action_type === 'on'}
                        <span class="detail-item">Mode: {command.mode === 1 ? 'Heat' : command.mode === 4 ? 'Cool' : command.mode}</span>
                        <span class="detail-item">Fan: {formatFanSpeed(command.fan_speed)}</span>
                        <span class="detail-item">Set: {command.request_temperature ? `${command.request_temperature.toFixed(1)}°C` : 'N/A'}</span>
                      {:else}
                        <span class="detail-item">—</span>
                      {/if}
                    </td>
                    <td class="measured-temp">
                      {command.measured_temperature ? `${command.measured_temperature.toFixed(1)}°C` : '—'}
                    </td>
                    <td class="measured-power">
                      <div class="power-values">
                        <div class="power-row">
                          <span class="power-label">Net:</span>
                          <span class="power-value">{command.measured_net_power_watt != null ? `${(command.measured_net_power_watt / 1000).toFixed(2)} kW` : '—'}</span>
                        </div>
                        <div class="power-row">
                          <span class="power-label">Solar:</span>
                          <span class="power-value">{command.measured_solar_production_watt != null ? `${(command.measured_solar_production_watt / 1000).toFixed(2)} kW` : '—'}</span>
                        </div>
                      </div>
                    </td>
                    <td class="cause-cell">
                      {#if command.cause_label && command.cause_label !== 'Undefined'}
                        <span class="cause-badge" title={command.cause_description}>
                          {command.cause_label}
                        </span>
                      {:else}
                        <span class="detail-item">—</span>
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
  }

  header {
    text-align: center;
    margin-bottom: 2rem;
  }

  h1 {
    margin: 0;
    font-size: 2.5rem;
    color: #646cff;
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
    color: #646cff;
  }

  .environmental-power .data-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1.5rem;
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
    border-color: #646cff;
    box-shadow: 0 0 20px rgba(100, 108, 255, 0.2);
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
    background: rgba(100, 108, 255, 0.1);
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
    color: #646cff;
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

  .commands-table th:nth-child(1) { width: 12%; }  /* Time */
  .commands-table th:nth-child(2) { width: 10%; }  /* Device */
  .commands-table th:nth-child(3) { width: 8%; }   /* Action */
  .commands-table th:nth-child(4) { width: 23%; }  /* Details */
  .commands-table th:nth-child(5) { width: 8%; }   /* Temp */
  .commands-table th:nth-child(6) { width: 18%; }  /* Net / Solar */
  .commands-table th:nth-child(7) { width: 20%; }  /* Cause */
  /* Total: 99% to allow for borders and padding */

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
    color: #646cff;
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

  .command-details {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .detail-item {
    font-size: 0.8rem;
    opacity: 0.8;
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

    .commands-table {
      font-size: 0.75rem;
    }

    .commands-table th,
    .commands-table td {
      padding: 0.5rem;
    }
  }
</style>
