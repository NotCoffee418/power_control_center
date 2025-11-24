<script>
  import { onMount } from 'svelte';
  import { 
    SvelteFlow, 
    Controls, 
    Background, 
    MiniMap,
    useSvelteFlow
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  // Node and edge state
  let nodes = $state([]);
  let edges = $state([]);
  let loading = $state(true);
  let saveStatus = $state('');

  // Define custom node types for AC planner logic
  const nodeTypes = {
    trigger: { color: '#4CAF50', label: 'Trigger' },
    condition: { color: '#2196F3', label: 'Condition' },
    action: { color: '#FF9800', label: 'Action' },
    output: { color: '#9C27B0', label: 'Output' }
  };

  // Load configuration from backend
  async function loadConfiguration() {
    try {
      const response = await fetch('/api/nodes/configuration');
      const result = await response.json();
      
      if (result.success && result.data) {
        nodes = result.data.nodes || [];
        edges = result.data.edges || [];
        
        // If empty, create initial example nodes
        if (nodes.length === 0) {
          createInitialNodes();
        }
      }
    } catch (e) {
      console.error('Error loading node configuration:', e);
      createInitialNodes();
    } finally {
      loading = false;
    }
  }

  // Create initial example nodes that reflect the AC planner logic
  function createInitialNodes() {
    nodes = [
      {
        id: '1',
        type: 'input',
        position: { x: 100, y: 100 },
        data: { 
          label: '🔄 On Evaluate Event\n(Every 5 minutes)',
          description: 'Provides: indoor_temp, outdoor_temp, solar_power, user_home'
        },
        style: 'background: #4CAF50; color: white; padding: 10px; border-radius: 5px; min-width: 200px;'
      },
      {
        id: '2',
        type: 'default',
        position: { x: 100, y: 250 },
        data: { 
          label: '❄️ Ice Exception Check',
          description: 'outdoor_temp < 2°C?'
        },
        style: 'background: #2196F3; color: white; padding: 10px; border-radius: 5px; min-width: 180px;'
      },
      {
        id: '3',
        type: 'default',
        position: { x: 400, y: 250 },
        data: { 
          label: '🌡️ Temperature Check',
          description: 'indoor_temp vs thresholds'
        },
        style: 'background: #2196F3; color: white; padding: 10px; border-radius: 5px; min-width: 180px;'
      },
      {
        id: '4',
        type: 'default',
        position: { x: 700, y: 250 },
        data: { 
          label: '☀️ Solar Power Check',
          description: 'solar_power > 2000W?'
        },
        style: 'background: #2196F3; color: white; padding: 10px; border-radius: 5px; min-width: 180px;'
      },
      {
        id: '5',
        type: 'default',
        position: { x: 400, y: 400 },
        data: { 
          label: '🏠 User Home Check',
          description: 'user_home == true?'
        },
        style: 'background: #2196F3; color: white; padding: 10px; border-radius: 5px; min-width: 180px;'
      },
      {
        id: '6',
        type: 'output',
        position: { x: 400, y: 550 },
        data: { 
          label: '🎯 Output: AC Plan',
          description: 'Mode: Colder/Warmer/Off\nIntensity: Low/Medium/High'
        },
        style: 'background: #9C27B0; color: white; padding: 10px; border-radius: 5px; min-width: 200px;'
      }
    ];

    edges = [
      { id: 'e1-2', source: '1', target: '2', animated: true },
      { id: 'e1-3', source: '1', target: '3', animated: true },
      { id: 'e2-6', source: '2', target: '6', label: 'OFF' },
      { id: 'e3-4', source: '3', target: '4' },
      { id: 'e3-5', source: '3', target: '5' },
      { id: 'e4-6', source: '4', target: '6', label: 'High' },
      { id: 'e5-6', source: '5', target: '6', label: 'Medium' }
    ];
  }

  // Save configuration to backend
  async function saveConfiguration() {
    saveStatus = 'Saving...';
    try {
      const response = await fetch('/api/nodes/configuration', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          nodes: nodes,
          edges: edges
        })
      });
      
      const result = await response.json();
      
      if (result.success) {
        saveStatus = '✓ Saved';
        setTimeout(() => saveStatus = '', 2000);
      } else {
        saveStatus = '✗ Save failed';
        console.error('Save failed:', result.error);
      }
    } catch (e) {
      saveStatus = '✗ Save failed';
      console.error('Error saving configuration:', e);
    }
  }

  // Add new node
  function addNode(type) {
    const newNode = {
      id: `node-${Date.now()}`,
      type: type === 'trigger' ? 'input' : type === 'output' ? 'output' : 'default',
      position: { 
        x: Math.random() * 400 + 200, 
        y: Math.random() * 300 + 200 
      },
      data: { 
        label: `New ${nodeTypes[type].label}`,
        description: 'Edit me'
      },
      style: `background: ${nodeTypes[type].color}; color: white; padding: 10px; border-radius: 5px; min-width: 150px;`
    };
    
    nodes = [...nodes, newNode];
  }

  // Reset to initial state
  function resetNodes() {
    if (confirm('Reset to default nodes? This will discard your current configuration.')) {
      createInitialNodes();
      saveConfiguration();
    }
  }

  onMount(() => {
    loadConfiguration();
  });

  // Handle node changes
  function onNodesChange(changes) {
    // Apply changes to nodes
    nodes = nodes;
  }

  function onEdgesChange(changes) {
    // Apply changes to edges
    edges = edges;
  }

  function onConnect(connection) {
    edges = [...edges, { ...connection, id: `e${connection.source}-${connection.target}` }];
  }
</script>

<div class="node-editor-container">
  <div class="toolbar">
    <h1>🔧 Node-Based AC Logic Editor</h1>
    <div class="toolbar-buttons">
      <button onclick={() => addNode('trigger')} class="btn btn-trigger">
        + Trigger Node
      </button>
      <button onclick={() => addNode('condition')} class="btn btn-condition">
        + Condition Node
      </button>
      <button onclick={() => addNode('action')} class="btn btn-action">
        + Action Node
      </button>
      <button onclick={() => addNode('output')} class="btn btn-output">
        + Output Node
      </button>
      <button onclick={saveConfiguration} class="btn btn-save">
        💾 Save
      </button>
      <button onclick={resetNodes} class="btn btn-reset">
        🔄 Reset
      </button>
      <a href="/" class="btn btn-back">← Back to Dashboard</a>
    </div>
    {#if saveStatus}
      <span class="save-status">{saveStatus}</span>
    {/if}
  </div>

  <div class="info-panel">
    <h3>ℹ️ Prototype Node Editor</h3>
    <p>
      This is a prototype node-based logic system that may replace the current AC planner.
      The nodes represent the logic flow for controlling AC units based on various conditions.
    </p>
    <ul>
      <li><strong>🔄 Trigger Nodes:</strong> Event triggers (e.g., "On Evaluate Event")</li>
      <li><strong>🌡️ Condition Nodes:</strong> Logic checks (temperature, solar, user presence)</li>
      <li><strong>🎯 Output Nodes:</strong> Final AC plan decisions</li>
    </ul>
    <p><em>Note: This is a visual prototype. Execution logic is not yet implemented.</em></p>
  </div>

  {#if loading}
    <div class="loading">Loading node configuration...</div>
  {:else}
    <div class="flow-container">
      <SvelteFlow 
        {nodes} 
        {edges}
        onnodeschange={onNodesChange}
        onedgeschange={onEdgesChange}
        onconnect={onConnect}
        fitView
      >
        <Controls />
        <Background />
        <MiniMap />
      </SvelteFlow>
    </div>
  {/if}
</div>

<style>
  .node-editor-container {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #f5f5f5;
  }

  .toolbar {
    background: white;
    padding: 1rem;
    border-bottom: 2px solid #ddd;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }

  .toolbar h1 {
    margin: 0 0 1rem 0;
    font-size: 1.5rem;
    color: #333;
  }

  .toolbar-buttons {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 500;
    transition: all 0.2s;
    text-decoration: none;
    display: inline-block;
  }

  .btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0,0,0,0.2);
  }

  .btn-trigger {
    background: #4CAF50;
    color: white;
  }

  .btn-condition {
    background: #2196F3;
    color: white;
  }

  .btn-action {
    background: #FF9800;
    color: white;
  }

  .btn-output {
    background: #9C27B0;
    color: white;
  }

  .btn-save {
    background: #00BCD4;
    color: white;
  }

  .btn-reset {
    background: #F44336;
    color: white;
  }

  .btn-back {
    background: #757575;
    color: white;
  }

  .save-status {
    margin-left: 1rem;
    font-weight: 500;
  }

  .info-panel {
    background: #fffbea;
    border: 2px solid #ffd700;
    border-radius: 8px;
    padding: 1rem;
    margin: 1rem;
  }

  .info-panel h3 {
    margin: 0 0 0.5rem 0;
    color: #856404;
  }

  .info-panel p {
    margin: 0.5rem 0;
    color: #856404;
  }

  .info-panel ul {
    margin: 0.5rem 0;
    padding-left: 1.5rem;
    color: #856404;
  }

  .info-panel li {
    margin: 0.25rem 0;
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    color: #666;
  }

  .flow-container {
    flex: 1;
    position: relative;
    background: #fafafa;
  }

  :global(.svelte-flow) {
    background: #fafafa;
  }

  :global(.svelte-flow__node) {
    font-family: inherit;
  }

  :global(.svelte-flow__edge-path) {
    stroke: #b1b1b7;
    stroke-width: 2;
  }

  :global(.svelte-flow__edge.animated path) {
    stroke-dasharray: 5;
    animation: dashdraw 0.5s linear infinite;
  }

  @keyframes dashdraw {
    to {
      stroke-dashoffset: -10;
    }
  }
</style>
