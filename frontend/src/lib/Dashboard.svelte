<script>
  import { onMount, onDestroy } from 'svelte';

  let dashboardData = $state(null);
  let loading = $state(true);
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

  onMount(() => {
    fetchDashboardData();
    // Refresh every 10 seconds
    refreshInterval = setInterval(fetchDashboardData, 10000);
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
      <!-- Environmental Data Section -->
      <div class="section environmental">
        <h2>Environmental</h2>
        <div class="data-grid">
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
            <span class="label">Solar Production</span>
            <span class="value large">
              {dashboardData.solar_production_watts != null 
                ? `${dashboardData.solar_production_watts} W` 
                : 'N/A'}
            </span>
          </div>
        </div>
      </div>

      <!-- Power Data Section -->
      <div class="section power">
        <h2>Power Grid</h2>
        <div class="data-grid">
          <div class="data-item">
            <span class="label">Consumption</span>
            <span class="value large consumption">{formatPower(dashboardData.current_consumption_kw)}</span>
          </div>
          <div class="data-item">
            <span class="label">Production</span>
            <span class="value large production">{formatPower(dashboardData.current_production_kw)}</span>
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
            <div class="device-card" class:active={device.is_on}>
              <div class="device-header">
                <h3>{device.name}</h3>
                <span class="status-badge" class:on={device.is_on}>
                  {device.is_on ? 'ON' : 'OFF'}
                </span>
              </div>
              
              {#if device.is_on}
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

  .environmental .data-grid,
  .power .data-grid {
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

  .data-item .value.consumption {
    color: #ff6b6b;
  }

  .data-item .value.production {
    color: #51cf66;
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

  .status-badge.on {
    background: #4caf50;
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

  @media (max-width: 768px) {
    h1 {
      font-size: 2rem;
    }

    .environmental .data-grid {
      grid-template-columns: 1fr;
    }

    .device-cards {
      grid-template-columns: 1fr;
    }
  }
</style>
