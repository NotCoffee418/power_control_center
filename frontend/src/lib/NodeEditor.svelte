<script>
  import { onMount, onDestroy } from 'svelte';
  import { 
    SvelteFlow, 
    Controls, 
    Background, 
    MiniMap
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import CustomNode from './CustomNode.svelte';

  // Node and edge state
  let nodes = $state([]);
  let edges = $state([]);
  let nodeDefinitions = $state([]);
  let loading = $state(true);
  let saveStatus = $state('');
  let searchQuery = $state('');
  let nodeIdCounter = 100;
  
  // Context menu state
  let contextMenu = $state({ visible: false, x: 0, y: 0, nodeId: null });
  const CONTEXT_MENU_HIDDEN = { visible: false, x: 0, y: 0, nodeId: null };

  // Helper functions
  function getNodeDisplayName(node) {
    return node?.data?.definition?.name || node?.id || 'Unknown';
  }

  function resetContextMenu() {
    contextMenu = { ...CONTEXT_MENU_HIDDEN };
  }

  // Custom node types
  const nodeTypes = {
    custom: CustomNode
  };

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
    return {
      id: id,
      type: 'custom',
      position: position,
      data: {
        label: definition.name,
        description: definition.description,
        definition: definition,
        isDefault: isDefault, // OnEvaluate is default and cannot be deleted
      },
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

  // Handle keyboard events for node deletion
  function handleKeyDown(event) {
    if (event.key === 'Delete' || event.key === 'Backspace') {
      // Find selected nodes
      const selectedNodes = nodes.filter(n => n.selected);
      
      if (selectedNodes.length === 0) {
        return;
      }

      // Check if any selected nodes are default nodes
      const defaultNodes = selectedNodes.filter(n => n.data?.isDefault);
      if (defaultNodes.length > 0) {
        const defaultNodeNames = defaultNodes.map(getNodeDisplayName).join(', ');
        alert(`Cannot delete default nodes: ${defaultNodeNames}`);
        return;
      }

      // Confirm deletion
      const nodeNames = selectedNodes.map(getNodeDisplayName).join(', ');
      if (confirm(`Delete ${selectedNodes.length} node(s)?\n\n${nodeNames}`)) {
        // Delete the selected nodes
        const selectedIds = selectedNodes.map(n => n.id);
        nodes = nodes.filter(n => !selectedIds.includes(n.id));
        
        // Also remove edges connected to deleted nodes
        edges = edges.filter(e => 
          !selectedIds.includes(e.source) && !selectedIds.includes(e.target)
        );
      }
    }
  }

  // Handle node context menu (right-click)
  function handleNodeContextMenu(event) {
    event.preventDefault();
    
    const nodeId = event.detail?.node?.id;
    if (!nodeId) return;
    
    contextMenu = {
      visible: true,
      x: event.detail.event.clientX,
      y: event.detail.event.clientY,
      nodeId: nodeId
    };
  }

  // Delete node from context menu
  function deleteNodeFromMenu() {
    const node = nodes.find(n => n.id === contextMenu.nodeId);
    
    if (!node) {
      resetContextMenu();
      return;
    }

    // Check if it's a default node
    if (node.data?.isDefault) {
      alert(`Cannot delete default node: ${getNodeDisplayName(node)}`);
      resetContextMenu();
      return;
    }

    // Confirm deletion
    if (confirm(`Delete node "${getNodeDisplayName(node)}"?`)) {
      // Delete the node
      nodes = nodes.filter(n => n.id !== contextMenu.nodeId);
      
      // Also remove edges connected to deleted node
      edges = edges.filter(e => 
        e.source !== contextMenu.nodeId && e.target !== contextMenu.nodeId
      );
    }
    
    resetContextMenu();
  }

  // Close context menu when clicking elsewhere
  function closeContextMenu() {
    if (contextMenu.visible) {
      resetContextMenu();
    }
  }

  onMount(async () => {
    await loadNodeDefinitions();
    await loadConfiguration();
    
    // Add keyboard event listener
    window.addEventListener('keydown', handleKeyDown);
  });

  onDestroy(() => {
    // Clean up keyboard event listener
    window.removeEventListener('keydown', handleKeyDown);
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
          saveStatus = '⚠ Cannot delete default node';
          setTimeout(() => saveStatus = '', 3000);
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

  // Helper function to get connection details
  function getConnectionDetails(connection) {
    const sourceNode = nodes.find(n => n.id === connection.source);
    const targetNode = nodes.find(n => n.id === connection.target);
    
    if (!sourceNode || !targetNode) {
      return null;
    }

    const sourceOutput = sourceNode.data.definition.outputs.find(
      o => o.id === connection.sourceHandle
    );
    const targetInput = targetNode.data.definition.inputs.find(
      i => i.id === connection.targetHandle
    );

    if (!sourceOutput || !targetInput) {
      return null;
    }

    return { sourceNode, targetNode, sourceOutput, targetInput };
  }

  // Check if a connection is valid based on type compatibility
  function isValidConnection(connection) {
    const details = getConnectionDetails(connection);
    if (!details) return false;

    const { sourceOutput, targetInput } = details;

    // Check if types are compatible
    const sourceType = sourceOutput.value_type?.type;
    const targetType = targetInput.value_type?.type;
    
    if (!sourceType || !targetType) {
      return false;
    }
    
    // Allow Object type to connect to anything (it's a complex/generic type)
    if (sourceType !== targetType && sourceType !== 'Object' && targetType !== 'Object') {
      return false;
    }

    return true;
  }

  function onConnect(connection) {
    // Get connection details once
    const details = getConnectionDetails(connection);
    if (!details) return;

    const { sourceOutput, targetInput } = details;

    // Validate connection types
    const sourceType = sourceOutput.value_type?.type;
    const targetType = targetInput.value_type?.type;
    
    if (!sourceType || !targetType) return;
    
    // Check compatibility
    if (sourceType !== targetType && sourceType !== 'Object' && targetType !== 'Object') {
      saveStatus = `⚠ Type mismatch: ${sourceType} → ${targetType}`;
      setTimeout(() => saveStatus = '', 3000);
      return;
    }

    // Create the edge
    edges = [...edges, { 
      ...connection, 
      id: `e${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}`,
      animated: true,
      style: `stroke: ${sourceOutput.color}; stroke-width: 2px;`
    }];
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
                style="border-left: 4px solid {def.color}"
              >
                <div class="node-item-header">
                  <span class="node-name">{def.name}</span>
                  <span class="node-category">{def.category}</span>
                </div>
                <div class="node-item-desc">{def.description}</div>
                <div class="node-item-ports">
                  {#if def.inputs.length > 0}
                    <div class="port-list">
                      <span class="port-list-title">⬅ Inputs:</span>
                      {#each def.inputs as input}
                        <span class="port-badge" style="background: {input.color};" title={input.description}>
                          {input.label}
                        </span>
                      {/each}
                    </div>
                  {/if}
                  {#if def.outputs.length > 0}
                    <div class="port-list">
                      <span class="port-list-title">Outputs ➡</span>
                      {#each def.outputs as output}
                        <span class="port-badge" style="background: {output.color};" title={output.description}>
                          {output.label}
                        </span>
                      {/each}
                    </div>
                  {/if}
                  {#if def.inputs.length === 0 && def.outputs.length === 0}
                    <span class="port-info">No ports</span>
                  {/if}
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
      <div class="flow-container" onclick={closeContextMenu} onkeydown={closeContextMenu} role="presentation">
        <SvelteFlow 
          {nodes} 
          {edges}
          {nodeTypes}
          onnodeschange={onNodesChange}
          onedgeschange={onEdgesChange}
          onconnect={onConnect}
          isValidConnection={isValidConnection}
          onnodecontextmenu={handleNodeContextMenu}
          fitView
        >
          <Controls />
          <Background />
          <MiniMap />
        </SvelteFlow>
        
        <!-- Context menu -->
        {#if contextMenu.visible}
          <div 
            class="context-menu" 
            style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
            role="menu"
            tabindex="-1"
          >
            <button onclick={deleteNodeFromMenu} class="context-menu-item">
              🗑️ Delete Node
            </button>
          </div>
        {/if}
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
    background: #1a1a1a;
  }

  .toolbar {
    background: #2d2d2d;
    padding: 1rem;
    border-bottom: 2px solid #404040;
    box-shadow: 0 2px 4px rgba(0,0,0,0.3);
  }

  .toolbar h1 {
    margin: 0 0 1rem 0;
    font-size: 1.5rem;
    color: #e0e0e0;
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
    color: #e0e0e0;
  }

  .main-content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .sidebar {
    width: 300px;
    background: #2d2d2d;
    border-right: 2px solid #404040;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar h3 {
    margin: 0;
    padding: 1rem;
    background: #252525;
    border-bottom: 1px solid #404040;
    color: #e0e0e0;
  }

  .search-input {
    width: calc(100% - 2rem);
    margin: 0.5rem 1rem;
    padding: 0.5rem;
    border: 1px solid #404040;
    border-radius: 4px;
    font-size: 0.9rem;
    background: #1a1a1a;
    color: #e0e0e0;
  }

  .search-input::placeholder {
    color: #888;
  }

  .search-input:focus {
    outline: none;
    border-color: #00BCD4;
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
    background: #1a1a1a;
    border: 1px solid #404040;
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    transition: all 0.2s;
  }

  .node-add-btn:hover {
    background: #333;
    border-color: #555;
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
    color: #e0e0e0;
  }

  .node-category {
    font-size: 0.75rem;
    color: #aaa;
    background: #404040;
    padding: 0.125rem 0.5rem;
    border-radius: 3px;
  }

  .node-item-desc {
    font-size: 0.8rem;
    color: #aaa;
    margin-bottom: 0.5rem;
  }

  .node-item-ports {
    font-size: 0.75rem;
    color: #888;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .port-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    align-items: center;
  }

  .port-list-title {
    font-weight: 600;
    margin-right: 0.25rem;
    color: #bbb;
  }

  .port-badge {
    display: inline-block;
    padding: 0.125rem 0.5rem;
    border-radius: 3px;
    font-size: 0.7rem;
    color: white;
    font-weight: 500;
    white-space: nowrap;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
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
    color: #aaa;
  }

  .loading-text {
    text-align: center;
    color: #aaa;
    padding: 1rem;
  }

  .no-results {
    text-align: center;
    color: #888;
    padding: 1rem;
    font-style: italic;
  }

  .flow-container {
    flex: 1;
    position: relative;
    background: #1a1a1a;
  }

  :global(.svelte-flow) {
    background: #1a1a1a;
  }

  :global(.svelte-flow__background) {
    background-color: #1a1a1a;
  }

  :global(.svelte-flow__edge-path) {
    stroke: #555;
  }

  :global(.svelte-flow__controls) {
    background: #2d2d2d;
    border: 1px solid #404040;
  }

  :global(.svelte-flow__controls button) {
    background: #2d2d2d;
    border-bottom: 1px solid #404040;
  }

  :global(.svelte-flow__controls button:hover) {
    background: #3d3d3d;
  }

  :global(.svelte-flow__minimap) {
    background: #2d2d2d;
    border: 1px solid #404040;
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

  /* Style for invalid connection lines being drawn */
  :global(.svelte-flow__connectionline.invalid) {
    stroke: #ff0000 !important;
    stroke-width: 3px !important;
  }

  /* Context menu */
  .context-menu {
    position: fixed;
    background: #2d2d2d;
    border: 1px solid #404040;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    z-index: 1000;
    min-width: 150px;
    overflow: hidden;
  }

  .context-menu-item {
    width: 100%;
    padding: 0.75rem 1rem;
    background: transparent;
    border: none;
    color: #e0e0e0;
    text-align: left;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.2s;
  }

  .context-menu-item:hover {
    background: #3d3d3d;
  }

  @keyframes dashdraw {
    to {
      stroke-dashoffset: -10;
    }
  }
</style>
