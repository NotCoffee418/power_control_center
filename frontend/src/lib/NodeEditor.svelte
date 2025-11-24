<script>
  import { onMount } from 'svelte';
  import { 
    SvelteFlow, 
    Controls, 
    Background, 
    MiniMap
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  // Node and edge state
  let nodes = $state([]);
  let edges = $state([]);
  let nodeDefinitions = $state([]);
  let loading = $state(true);
  let saveStatus = $state('');
  let searchQuery = $state('');
  let nodeIdCounter = 100;

  // Category colors
  const categoryColors = {
    'System': '#4CAF50',
    'AC Controller': '#2196F3',
    'default': '#757575'
  };

  // Load node definitions from backend
  async function loadNodeDefinitions() {
    try {
      const response = await fetch('/api/nodes/definitions');
      const result = await response.json();
      
      if (result.success && result.data) {
        nodeDefinitions = result.data;
        console.log('Loaded node definitions:', nodeDefinitions);
      } else {
        console.error('Failed to load node definitions:', result.error);
      }
    } catch (e) {
      console.error('Error loading node definitions:', e);
    }
  }

  // Load configuration from backend
  async function loadConfiguration() {
    try {
      const response = await fetch('/api/nodes/configuration');
      const result = await response.json();
      
      if (result.success && result.data) {
        nodes = result.data.nodes || [];
        edges = result.data.edges || [];
        
        // If empty, create initial nodes with OnEvaluate
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

  // Create initial nodes with OnEvaluate node
  function createInitialNodes() {
    const onEvaluateDef = nodeDefinitions.find(def => def.node_type === 'on_evaluate');
    if (onEvaluateDef) {
      nodes = [
        createNodeFromDefinition(onEvaluateDef, 'on-evaluate-1', { x: 100, y: 100 }, true)
      ];
    }
  }

  // Create a node from a definition
  function createNodeFromDefinition(definition, id, position, isDefault = false) {
    const color = categoryColors[definition.category] || categoryColors['default'];
    
    return {
      id: id,
      type: 'default',
      position: position,
      data: {
        label: definition.name,
        description: definition.description,
        definition: definition,
        isDefault: isDefault, // OnEvaluate is default and cannot be deleted
      },
      style: `background: ${color}; color: white; padding: 10px; border-radius: 5px; min-width: 200px;`,
      sourcePosition: 'right', // Outputs on the right
      targetPosition: 'left',  // Inputs on the left
    };
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

  // Add new node from definition
  function addNodeFromDefinition(definition) {
    nodeIdCounter++;
    const newNode = createNodeFromDefinition(
      definition,
      `${definition.node_type}-${nodeIdCounter}`,
      { 
        x: Math.random() * 400 + 200, 
        y: Math.random() * 300 + 200 
      },
      false
    );
    
    nodes = [...nodes, newNode];
  }

  // Reset to initial state
  function resetNodes() {
    if (confirm('Reset to default nodes? This will discard your current configuration.')) {
      createInitialNodes();
      edges = [];
      saveConfiguration();
    }
  }

  // Computed: Filter nodes based on search query
  function getFilteredDefinitions() {
    return nodeDefinitions.filter(def => {
      if (!searchQuery) return true;
      const query = searchQuery.toLowerCase();
      return def.name.toLowerCase().includes(query) ||
             def.description.toLowerCase().includes(query) ||
             def.category.toLowerCase().includes(query);
    });
  }

  onMount(async () => {
    await loadNodeDefinitions();
    await loadConfiguration();
  });

  // Handle node changes (position, selection, removal)
  function onNodesChange(changes) {
    changes.forEach(change => {
      if (change.type === 'position' && change.dragging === false) {
        // Update node position
        const nodeIndex = nodes.findIndex(n => n.id === change.id);
        if (nodeIndex !== -1 && change.position) {
          nodes[nodeIndex].position = change.position;
        }
      } else if (change.type === 'remove') {
        // Check if node is default (OnEvaluate) - should not be deletable
        const node = nodes.find(n => n.id === change.id);
        if (node?.data?.isDefault) {
          console.warn('Cannot delete default node:', node.data.label);
          return;
        }
        // Remove node
        nodes = nodes.filter(n => n.id !== change.id);
      }
    });
    // Trigger reactivity
    nodes = nodes;
  }

  function onEdgesChange(changes) {
    changes.forEach(change => {
      if (change.type === 'remove') {
        // Remove edge
        edges = edges.filter(e => e.id !== change.id);
      }
    });
    // Trigger reactivity
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

  <div class="main-content">
    <div class="sidebar">
      <h3>Available Nodes</h3>
      <input 
        type="text" 
        placeholder="Search nodes..." 
        bind:value={searchQuery}
        class="search-input"
      />
      
      <div class="node-list">
        {#if loading}
          <p class="loading-text">Loading...</p>
        {:else if getFilteredDefinitions().length === 0}
          <p class="no-results">No nodes found</p>
        {:else}
          {#each getFilteredDefinitions() as def}
            <div class="node-item">
              <button 
                onclick={() => addNodeFromDefinition(def)}
                class="node-add-btn"
                style="border-left: 4px solid {categoryColors[def.category] || categoryColors['default']}"
              >
                <div class="node-item-header">
                  <span class="node-name">{def.name}</span>
                  <span class="node-category">{def.category}</span>
                </div>
                <div class="node-item-desc">{def.description}</div>
                <div class="node-item-ports">
                  <span class="port-info">
                    {#if def.inputs.length > 0}
                      ⬅ {def.inputs.length} input{def.inputs.length !== 1 ? 's' : ''}
                    {/if}
                    {#if def.outputs.length > 0}
                      {#if def.inputs.length > 0} • {/if}
                      {def.outputs.length} output{def.outputs.length !== 1 ? 's' : ''} ➡
                    {/if}
                    {#if def.inputs.length === 0 && def.outputs.length === 0}
                      No ports
                    {/if}
                  </span>
                </div>
              </button>
            </div>
          {/each}
        {/if}
      </div>
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

  .main-content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .sidebar {
    width: 300px;
    background: white;
    border-right: 2px solid #ddd;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar h3 {
    margin: 0;
    padding: 1rem;
    background: #f5f5f5;
    border-bottom: 1px solid #ddd;
  }

  .search-input {
    width: calc(100% - 2rem);
    margin: 1rem;
    padding: 0.5rem;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 0.9rem;
  }

  .node-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .node-item {
    margin-bottom: 0.5rem;
  }

  .node-add-btn {
    width: 100%;
    padding: 0.75rem;
    background: white;
    border: 1px solid #ddd;
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    transition: all 0.2s;
  }

  .node-add-btn:hover {
    background: #f5f5f5;
    border-color: #999;
    transform: translateX(2px);
  }

  .node-item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .node-name {
    font-weight: 600;
    color: #333;
  }

  .node-category {
    font-size: 0.75rem;
    color: #666;
    background: #f0f0f0;
    padding: 0.125rem 0.5rem;
    border-radius: 3px;
  }

  .node-item-desc {
    font-size: 0.8rem;
    color: #666;
    margin-bottom: 0.5rem;
  }

  .node-item-ports {
    font-size: 0.75rem;
    color: #999;
  }

  .port-info {
    font-style: italic;
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    color: #666;
  }

  .loading-text {
    text-align: center;
    color: #666;
    padding: 1rem;
  }

  .no-results {
    text-align: center;
    color: #999;
    padding: 1rem;
    font-style: italic;
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
