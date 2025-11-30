<script>
  import { onMount, onDestroy } from 'svelte';
  import { 
    SvelteFlow, 
    Controls, 
    Background, 
    MiniMap,
    useConnection
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import CustomNode from './CustomNode.svelte';
  import ReconnectableEdge from './ReconnectableEdge.svelte';
  import SimulatorDrawer from './SimulatorDrawer.svelte';
  import CauseReasonsPanel from './CauseReasonsPanel.svelte';
  
  // Get connection state for click-connect feature
  const connection = useConnection();

  // Constants for nodeset IDs
  const NEW_NODESET_ID = -1;
  const DEFAULT_NODESET_ID = 0;

  // Node and edge state - using $state.raw for proper SvelteFlow integration
  // $state.raw prevents deep reactivity, allowing SvelteFlow to manage internal state
  let nodes = $state.raw([]);
  let edges = $state.raw([]);
  let nodeDefinitions = $state([]);
  let loading = $state(true);
  let saveStatus = $state('');
  let searchQuery = $state('');
  
  // Nodeset state
  let nodesets = $state([]);
  let currentNodesetId = $state(NEW_NODESET_ID);  // Currently being edited
  let currentNodesetName = $state('New');
  let selectedNodesetId = $state(NEW_NODESET_ID);
  let hasUnsavedChanges = $state(false);
  
  // Active profile state (the one used for AC logic)
  let activeNodesetId = $state(DEFAULT_NODESET_ID);
  let activeNodesetName = $state('Default');
  
  // Simulator drawer state
  let simulatorOpen = $state(false);
  
  // Constants for node spawn positioning
  const NODE_WIDTH = 220; // Approximate width of a node
  const NODE_SPAWN_MARGIN = 50; // Margin between spawned nodes
  
  // Generate a unique node ID using crypto.randomUUID for guaranteed uniqueness
  // Falls back to timestamp + random for older browsers without crypto.randomUUID
  function generateUniqueNodeId(nodeType) {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      // crypto.randomUUID() provides a RFC 4122 compliant UUID
      return `${nodeType}-${crypto.randomUUID()}`;
    }
    // Fallback for older browsers: timestamp + random string
    const timestamp = Date.now();
    const randomSuffix = Math.random().toString(36).substring(2, 10);
    return `${nodeType}-${timestamp}-${randomSuffix}`;
  }
  
  // Calculate spawn position for new nodes
  // Spawns to the right of the rightmost existing node
  function calculateSpawnPosition() {
    if (nodes.length === 0) {
      // No existing nodes, spawn at a reasonable default position
      return { x: 250, y: 200 };
    }
    
    // Find the highest X position among existing nodes
    // Initialize with first node's position to handle negative X values correctly
    // Using optional chaining for defensive programming
    let maxX = nodes[0]?.position?.x ?? 0;
    let avgY = 0;
    
    for (const node of nodes) {
      const nodeX = node.position?.x ?? 0;
      const nodeY = node.position?.y ?? 0;
      if (nodeX > maxX) {
        maxX = nodeX;
      }
      avgY += nodeY;
    }
    
    // Calculate average Y position for consistent vertical placement
    avgY = avgY / nodes.length;
    
    // Spawn to the right of the rightmost node with margin
    return {
      x: maxX + NODE_WIDTH + NODE_SPAWN_MARGIN,
      y: avgY
    };
  }
  
  // Context menu state
  let contextMenu = $state({ visible: false, x: 0, y: 0, nodeId: null, edgeId: null, type: null });
  const CONTEXT_MENU_HIDDEN = { visible: false, x: 0, y: 0, nodeId: null, edgeId: null, type: null };

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

  // Custom edge types with reconnect anchors
  const edgeTypes = {
    default: ReconnectableEdge
  };

  // Category colors
  const categoryColors = {
    'System': '#4CAF50',
    'Logic': '#9C27B0',
    'Primitives': '#FF9800',
    'Sensors': '#00BCD4',
    'Enums': '#E91E63',
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

  // Handle cause reasons changes - update node definitions and existing nodes
  async function handleCauseReasonsChanged() {
    await loadNodeDefinitions();
    
    // Find the cause_reason node definition to get updated options
    const causeReasonDef = nodeDefinitions.find(d => d.node_type === 'cause_reason');
    if (!causeReasonDef) return;
    
    // Get the new list of available cause reason IDs
    const enumOutput = causeReasonDef.outputs?.[0];
    let availableIds = [];
    let defaultId = '0'; // ID for "Undefined"
    
    // CauseReason type has the same structure as EnumWithIds
    if (enumOutput?.value_type?.type === 'CauseReason' || enumOutput?.value_type?.type === 'EnumWithIds') {
      // Format with ID-label pairs
      availableIds = enumOutput.value_type.value.map(opt => opt.id);
      if (enumOutput.value_type.value.length > 0) {
        defaultId = enumOutput.value_type.value[0].id;
      }
    } else if (enumOutput?.value_type?.type === 'Enum') {
      // Legacy format with just labels
      availableIds = enumOutput.value_type.value;
      if (enumOutput.value_type.value.length > 0) {
        defaultId = enumOutput.value_type.value[0];
      }
    }
    
    // Check if any cause_reason nodes need updating
    const hasCauseReasonNodes = nodes.some(node => node.data?.definition?.node_type === 'cause_reason');
    if (!hasCauseReasonNodes) return;
    
    // Update all cause_reason nodes
    let hasChanges = false;
    const updatedNodes = nodes.map(node => {
      if (node.data?.definition?.node_type === 'cause_reason') {
        const currentEnumValue = node.data.enumValue;
        hasChanges = true; // Definition needs updating
        
        // Update the node's definition with the new options
        const updatedNode = {
          ...node,
          data: {
            ...node.data,
            definition: causeReasonDef
          }
        };
        
        // If the current value (ID) is no longer valid, reset to default (Undefined)
        if (currentEnumValue && !availableIds.includes(currentEnumValue)) {
          updatedNode.data.enumValue = defaultId;
        }
        
        return updatedNode;
      }
      return node;
    });
    
    // Update nodes if there were cause_reason nodes to update
    if (hasChanges) {
      nodes = updatedNodes;
    }
  }

  // Load list of nodesets
  async function loadNodesets() {
    try {
      const response = await fetch('/api/nodes/nodesets');
      const result = await response.json();
      
      if (result.success && result.data) {
        nodesets = result.data;
        console.log('Loaded nodesets:', nodesets);
      } else {
        console.error('Failed to load nodesets:', result.error);
      }
    } catch (e) {
      console.error('Error loading nodesets:', e);
    }
  }

  // Load active nodeset from backend
  async function loadActiveNodeset() {
    try {
      const response = await fetch('/api/nodes/nodesets/active');
      const result = await response.json();
      
      if (result.success && result.data) {
        // Set both the editing context and the active profile info
        currentNodesetId = result.data.id;
        currentNodesetName = result.data.name;
        selectedNodesetId = result.data.id;
        nodes = result.data.nodes || [];
        edges = result.data.edges || [];
        
        // Also store as the active profile
        activeNodesetId = result.data.id;
        activeNodesetName = result.data.name;
        
        // Reset unsaved changes after loading
        hasUnsavedChanges = false;
      }
    } catch (e) {
      console.error('Error loading active nodeset:', e);
      createInitialNodes();
    } finally {
      loading = false;
    }
  }

  // Load a specific nodeset
  async function loadNodeset(id) {
    try {
      const response = await fetch(`/api/nodes/nodesets/${id}`);
      const result = await response.json();
      
      if (result.success && result.data) {
        currentNodesetId = result.data.id;
        currentNodesetName = result.data.name;
        selectedNodesetId = result.data.id;
        nodes = result.data.nodes || [];
        edges = result.data.edges || [];
        
        // Reset unsaved changes after loading
        hasUnsavedChanges = false;
        return true;
      } else {
        console.error('Failed to load nodeset:', result.error);
        return false;
      }
    } catch (e) {
      console.error('Error loading nodeset:', e);
      return false;
    }
  }

  // Create initial nodes (empty canvas by default)
  function createInitialNodes() {
    nodes = [];
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

  // Save configuration to the current nodeset
  async function saveConfiguration() {
    // If we're on a new nodeset, treat as Save As
    if (currentNodesetId === NEW_NODESET_ID) {
      await saveAsNodeset();
      return;
    }
    
    // Prevent saving to the default nodeset
    if (currentNodesetId === DEFAULT_NODESET_ID) {
      saveStatus = '⚠ Cannot modify default profile. Use Save As.';
      setTimeout(() => saveStatus = '', 3000);
      return;
    }
    
    if (!confirm('Save changes to the current profile?')) {
      return;
    }
    
    saveStatus = 'Saving...';
    try {
      const response = await fetch(`/api/nodes/nodesets/${currentNodesetId}`, {
        method: 'PUT',
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
        hasUnsavedChanges = false;
        setTimeout(() => saveStatus = '', 2000);
      } else {
        saveStatus = '✗ ' + (result.error || 'Save failed');
        console.error('Save failed:', result.error);
        setTimeout(() => saveStatus = '', 3000);
      }
    } catch (e) {
      saveStatus = '✗ Save failed';
      console.error('Error saving configuration:', e);
      setTimeout(() => saveStatus = '', 3000);
    }
  }

  // Save As - create a new nodeset with a name
  async function saveAsNodeset() {
    const name = prompt('Enter a name for the new profile:');
    if (!name || name.trim() === '') {
      return;
    }
    
    saveStatus = 'Saving...';
    try {
      const response = await fetch('/api/nodes/nodesets', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: name.trim(),
          nodes: nodes,
          edges: edges
        })
      });
      
      const result = await response.json();
      
      if (result.success) {
        saveStatus = '✓ Saved as "' + result.data.name + '"';
        currentNodesetId = result.data.id;
        currentNodesetName = result.data.name;
        selectedNodesetId = result.data.id;
        hasUnsavedChanges = false;
        await loadNodesets(); // Refresh the list
        setTimeout(() => saveStatus = '', 2000);
      } else {
        saveStatus = '✗ ' + (result.error || 'Save failed');
        console.error('Save failed:', result.error);
        setTimeout(() => saveStatus = '', 3000);
      }
    } catch (e) {
      saveStatus = '✗ Save failed';
      console.error('Error saving configuration:', e);
      setTimeout(() => saveStatus = '', 3000);
    }
  }

  // Activate the selected profile
  async function activateProfile() {
    if (selectedNodesetId === NEW_NODESET_ID) {
      saveStatus = '⚠ Please save the profile first';
      setTimeout(() => saveStatus = '', 3000);
      return;
    }
    
    if (!confirm('Activate this profile? It will be used for the AC logic.')) {
      return;
    }
    
    try {
      const response = await fetch(`/api/nodes/nodesets/active/${selectedNodesetId}`, {
        method: 'PUT'
      });
      
      const result = await response.json();
      
      if (result.success) {
        // Update the active profile display
        activeNodesetId = currentNodesetId;
        activeNodesetName = currentNodesetName;
        saveStatus = '✓ Profile activated';
        setTimeout(() => saveStatus = '', 2000);
      } else {
        saveStatus = '✗ ' + (result.error || 'Activation failed');
        setTimeout(() => saveStatus = '', 3000);
      }
    } catch (e) {
      saveStatus = '✗ Activation failed';
      console.error('Error activating profile:', e);
      setTimeout(() => saveStatus = '', 3000);
    }
  }

  // Delete the current nodeset
  async function deleteNodeset() {
    if (currentNodesetId === NEW_NODESET_ID) {
      // Just reset to new state
      createNewNodeset();
      return;
    }
    
    if (currentNodesetId === DEFAULT_NODESET_ID) {
      saveStatus = '⚠ Cannot delete the default profile';
      setTimeout(() => saveStatus = '', 3000);
      return;
    }
    
    if (!confirm(`Delete profile "${currentNodesetName}"? This cannot be undone.`)) {
      return;
    }
    
    try {
      const response = await fetch(`/api/nodes/nodesets/${currentNodesetId}`, {
        method: 'DELETE'
      });
      
      const result = await response.json();
      
      if (result.success) {
        saveStatus = '✓ Profile deleted';
        await loadNodesets();
        // Switch to default profile
        await loadNodeset(0);
        setTimeout(() => saveStatus = '', 2000);
      } else {
        saveStatus = '✗ ' + (result.error || 'Delete failed');
        setTimeout(() => saveStatus = '', 3000);
      }
    } catch (e) {
      saveStatus = '✗ Delete failed';
      console.error('Error deleting profile:', e);
      setTimeout(() => saveStatus = '', 3000);
    }
  }

  // Create a new nodeset (resets to empty state)
  function createNewNodeset() {
    currentNodesetId = NEW_NODESET_ID;
    currentNodesetName = 'New';
    selectedNodesetId = NEW_NODESET_ID;
    nodes = [];
    edges = [];
    hasUnsavedChanges = false;
  }

  // Handle nodeset selection change
  async function handleNodesetChange(event) {
    const newId = parseInt(event.target.value, 10);
    
    if (newId === currentNodesetId) {
      return;
    }
    
    if (hasUnsavedChanges) {
      if (!confirm('You have unsaved changes. Discard them and switch profiles?')) {
        // Reset the dropdown to current value
        selectedNodesetId = currentNodesetId;
        return;
      }
    }
    
    if (newId === NEW_NODESET_ID) {
      createNewNodeset();
    } else {
      await loadNodeset(newId);
    }
  }

  // Add new node from definition
  function addNodeFromDefinition(definition) {
    const uniqueId = generateUniqueNodeId(definition.node_type);
    const spawnPosition = calculateSpawnPosition();
    
    const newNode = createNodeFromDefinition(
      definition,
      uniqueId,
      spawnPosition,
      false
    );
    
    nodes = [...nodes, newNode];
    hasUnsavedChanges = true;
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

  // Handle node context menu (right-click)
  function handleNodeContextMenu({ node, event }) {
    // Prevent the browser's default context menu
    event.preventDefault();
    event.stopPropagation();
    
    const nodeId = node?.id;
    if (!nodeId) return;
    
    contextMenu = {
      visible: true,
      x: event.clientX,
      y: event.clientY,
      nodeId: nodeId,
      edgeId: null,
      type: 'node'
    };
  }

  // Handle edge context menu (right-click)
  function handleEdgeContextMenu({ edge, event }) {
    // Prevent the browser's default context menu
    event.preventDefault();
    event.stopPropagation();
    
    const edgeId = edge?.id;
    if (!edgeId) return;
    
    contextMenu = {
      visible: true,
      x: event.clientX,
      y: event.clientY,
      nodeId: null,
      edgeId: edgeId,
      type: 'edge'
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
      hasUnsavedChanges = true;
    }
    
    resetContextMenu();
  }

  // Delete edge from context menu
  function deleteEdgeFromMenu() {
    const edgeId = contextMenu.edgeId;
    
    if (!edgeId) {
      resetContextMenu();
      return;
    }

    // Confirm deletion
    if (confirm('Delete this connection?')) {
      edges = edges.filter(e => e.id !== edgeId);
      hasUnsavedChanges = true;
    }
    
    resetContextMenu();
  }

  // Check if there's an active click-connect in progress
  function hasActiveConnection() {
    return connection.current?.inProgress;
  }

  // Handle pane context menu - cancel pending connection on right-click
  // When clickConnect is enabled, clicking a pin starts a connection that follows the cursor.
  // The connection is completed by clicking another compatible pin, or cancelled by:
  // 1. Clicking on the pane (empty space) - SvelteFlow handles this automatically
  // 2. Right-clicking anywhere - we prevent the context menu and let the click cancel the connection
  // 3. Pressing Escape - SvelteFlow handles this automatically
  function handlePaneContextMenu({ event }) {
    // If there's a pending connection (click-connect in progress), cancel it
    // by preventing the default context menu - the click event itself cancels the connection
    if (hasActiveConnection()) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    
    // Otherwise, close any existing context menu
    resetContextMenu();
  }

  // Close context menu when clicking elsewhere
  function closeContextMenu() {
    if (contextMenu.visible) {
      resetContextMenu();
    }
  }

  // Close context menu on Escape key
  function handleContextMenuKeyDown(event) {
    if (event.key === 'Escape' && contextMenu.visible) {
      resetContextMenu();
    }
  }

  // Warn user before leaving page with unsaved changes
  function handleBeforeUnload(event) {
    if (hasUnsavedChanges) {
      event.preventDefault();
      // Modern browsers ignore custom messages, but we still need to set returnValue
      event.returnValue = '';
      return '';
    }
  }

  // Handle back button click with unsaved changes check
  // Stops propagation to prevent App.svelte's global click handler from navigating
  function handleBackClick(event) {
    if (hasUnsavedChanges) {
      if (!confirm('You have unsaved changes. Discard them and go back?')) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }
    }
    // Allow navigation to proceed (App.svelte's global handler will navigate)
  }

  onMount(async () => {
    window.addEventListener('beforeunload', handleBeforeUnload);
    await loadNodeDefinitions();
    await loadNodesets();
    await loadActiveNodeset();
  });

  onDestroy(() => {
    window.removeEventListener('beforeunload', handleBeforeUnload);
  });

  // Handle node changes - with bind:nodes, position and selection are handled automatically
  // We only need to handle removal to protect default nodes
  function onNodesChange(changes) {
    changes.forEach(change => {
      // Mark as having unsaved changes for any meaningful change (position, dimensions, remove, add)
      // Selection changes don't count as unsaved changes
      if (change.type === 'position' || 
          change.type === 'dimensions' || 
          change.type === 'remove' || 
          change.type === 'add') {
        hasUnsavedChanges = true;
      }
      
      if (change.type === 'remove') {
        // Check if node is default (OnEvaluate) - should not be deletable
        const node = nodes.find(n => n.id === change.id);
        if (node?.data?.isDefault) {
          saveStatus = '⚠ Cannot delete default node';
          setTimeout(() => saveStatus = '', 3000);
          // Note: The actual removal is handled by onBeforeDelete which returns false
          // to prevent SvelteFlow's default behavior for default nodes
        }
      }
    });
  }

  // Handle edge changes - with bind:edges, changes are handled automatically
  function onEdgesChange(changes) {
    // Mark as having unsaved changes for meaningful edge modifications
    // Filter out selection-only changes
    changes.forEach(change => {
      if (change.type === 'add' || 
          change.type === 'remove' || 
          change.type === 'reset') {
        hasUnsavedChanges = true;
      }
    });
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
    // For dynamic logic nodes (AND, OR, NAND), also check dynamicInputs for added pins
    let targetInput = targetNode.data.definition.inputs.find(
      i => i.id === connection.targetHandle
    );
    if (!targetInput && targetNode.data.dynamicInputs) {
      targetInput = targetNode.data.dynamicInputs.find(
        i => i.id === connection.targetHandle
      );
    }

    if (!sourceOutput || !targetInput) {
      return null;
    }

    return { sourceNode, targetNode, sourceOutput, targetInput };
  }

  // Helper function to check if two enum types are compatible (same values)
  // Enums are compatible only if they have the exact same set of values
  // Supports both Enum (simple strings) and EnumWithIds (id-label pairs)
  function areEnumsCompatible(sourceValueType, targetValueType) {
    const sourceType = sourceValueType?.type;
    const targetType = targetValueType?.type;
    const sourceValues = sourceValueType?.value || [];
    const targetValues = targetValueType?.value || [];
    
    // Enums without values cannot be matched - they represent incomplete type definitions
    if (sourceValues.length === 0 || targetValues.length === 0) {
      return false;
    }
    
    // Check if they have the same values (same length and all values match)
    if (sourceValues.length !== targetValues.length) {
      return false;
    }
    
    // Handle EnumWithIds: compare by ID rather than by object reference
    if (sourceType === 'EnumWithIds' && targetType === 'EnumWithIds') {
      const sortedSourceIds = [...sourceValues].map(v => v.id).sort();
      const sortedTargetIds = [...targetValues].map(v => v.id).sort();
      return sortedSourceIds.every((id, idx) => id === sortedTargetIds[idx]);
    }
    
    // Handle regular Enum: compare by string value
    const sortedSource = [...sourceValues].sort();
    const sortedTarget = [...targetValues].sort();
    
    return sortedSource.every((val, idx) => val === sortedTarget[idx]);
  }

  // Node types that have bidirectional type constraints between inputs and outputs
  // These nodes require that all Any-typed pins (both input and output) share the same type
  const BIDIRECTIONAL_CONSTRAINT_NODES = ['logic_branch'];

  /**
   * Get the constrained type for a node's "Any" type inputs based on existing connections.
   * When one input with "Any" type is already connected, other "Any" type inputs on the same
   * node should be constrained to match that type.
   * 
   * For nodes with bidirectional constraints (like Branch), this also checks outgoing
   * connections from Any-type outputs to determine the constraint.
   * 
   * @param nodeId - The node ID to check
   * @param excludeHandle - The handle ID to exclude (the one we're connecting to)
   * @param allowedTypes - Optional array of types that are valid for this constraint (e.g., ['Float', 'Integer'] for numeric nodes)
   * @param checkOutputs - Whether to also check outgoing connections from outputs (default: true for bidirectional constraint nodes like Branch)
   * @returns The constrained type from other connected inputs/outputs, or null if no constraint exists
   */
  function getConstrainedTypeFromConnectedInputs(nodeId, excludeHandle, allowedTypes = null, checkOutputs = true) {
    const node = nodes.find(n => n.id === nodeId);
    if (!node) return null;

    const nodeType = node.data.definition?.node_type;
    const hasBidirectionalConstraint = BIDIRECTIONAL_CONSTRAINT_NODES.includes(nodeType);

    // Get all inputs for this node that have "Any" type
    const anyTypeInputs = (node.data.definition?.inputs || [])
      .filter(input => input.value_type?.type === 'Any');

    // Find existing connections to other "Any" type inputs on this node
    for (const input of anyTypeInputs) {
      if (input.id === excludeHandle) continue;

      const existingEdge = edges.find(
        e => e.target === nodeId && e.targetHandle === input.id
      );

      if (existingEdge) {
        // Found a connection to another Any-type input, get the source type
        const sourceNode = nodes.find(n => n.id === existingEdge.source);
        if (sourceNode) {
          const sourceOutput = sourceNode.data.definition?.outputs?.find(
            o => o.id === existingEdge.sourceHandle
          );
          if (sourceOutput?.value_type) {
            let constrainedType = sourceOutput.value_type;
            
            // If source is also Any type, need to resolve its constraint
            if (constrainedType.type === 'Any') {
              // Try to get the constraint from the source node
              const sourceConstraint = getConstrainedOutputType(existingEdge.source, existingEdge.sourceHandle);
              if (sourceConstraint) {
                constrainedType = sourceConstraint;
              } else {
                // Skip this connection if we can't resolve the type
                continue;
              }
            }
            
            // If allowedTypes is specified, verify the constraint is valid
            if (allowedTypes && !allowedTypes.includes(constrainedType.type)) {
              // The connected type is not in the allowed list - this indicates
              // an invalid state (shouldn't happen in normal usage)
              console.warn(`Type constraint violation: ${constrainedType.type} is not in allowed types [${allowedTypes.join(', ')}]`);
              // Return null to allow any connection, as the existing connection is invalid
              return null;
            }
            
            return constrainedType;
          }
        }
      }
    }

    // For nodes with bidirectional constraints (like Branch), also check outgoing output connections
    if (hasBidirectionalConstraint && checkOutputs) {
      const outputConstraint = getConstrainedTypeFromConnectedOutputs(nodeId);
      if (outputConstraint) {
        // If allowedTypes is specified, verify the constraint is valid
        if (allowedTypes && !allowedTypes.includes(outputConstraint.type)) {
          console.warn(`Type constraint violation from output: ${outputConstraint.type} is not in allowed types [${allowedTypes.join(', ')}]`);
          return null;
        }
        return outputConstraint;
      }
    }

    return null; // No constraint from other inputs or outputs
  }

  /**
   * Get the constrained type from connected outputs (outgoing connections from an output pin).
   * For nodes like Branch where inputs and outputs are all constrained together,
   * we need to check outgoing connections from the output to determine the constraint.
   * 
   * @param nodeId - The node ID to check
   * @returns The constrained type from outgoing connections, or null if no constraint exists
   */
  function getConstrainedTypeFromConnectedOutputs(nodeId) {
    const node = nodes.find(n => n.id === nodeId);
    if (!node) return null;

    // Get all outputs for this node that have "Any" type
    const anyTypeOutputs = (node.data.definition?.outputs || [])
      .filter(output => output.value_type?.type === 'Any');

    // Find existing outgoing connections from Any-type outputs on this node
    for (const output of anyTypeOutputs) {
      const existingEdge = edges.find(
        e => e.source === nodeId && e.sourceHandle === output.id
      );

      if (existingEdge) {
        // Found an outgoing connection, get the target type
        const targetNode = nodes.find(n => n.id === existingEdge.target);
        if (targetNode) {
          const targetInput = targetNode.data.definition?.inputs?.find(
            i => i.id === existingEdge.targetHandle
          );
          if (targetInput?.value_type) {
            // If target is also "Any", we need to look at what it's constrained to
            if (targetInput.value_type.type === 'Any') {
              // Check if the target node has constraints from its other connections.
              // Pass checkOutputs=false to prevent infinite recursion. Without this:
              // getConstrainedTypeFromConnectedOutputs(BranchNode) ->
              //   getConstrainedTypeFromConnectedInputs(TargetNode, checkOutputs=true) ->
              //     getConstrainedTypeFromConnectedOutputs(TargetNode) ->
              //       ... potentially back to BranchNode (infinite loop)
              const targetConstraint = getConstrainedTypeFromConnectedInputs(
                existingEdge.target,
                existingEdge.targetHandle,
                null,
                false  // Don't check outputs to avoid infinite recursion
              );
              if (targetConstraint) {
                return targetConstraint;
              }
              // We don't recursively check target's outputs - the constraint should come from concrete types
            } else {
              // Target has a concrete type
              return targetInput.value_type;
            }
          }
        }
      }
    }

    return null; // No constraint from outgoing connections
  }

  /**
   * Get the constrained type for a node's "Any" type output based on connected inputs.
   * For nodes like Branch where the output type depends on the input types,
   * this returns the type that the output should be constrained to.
   *
   * @param nodeId - The node ID to check
   * @param outputHandle - The output handle ID
   * @returns The constrained type from connected inputs, or null if no constraint exists
   */
  function getConstrainedOutputType(nodeId, outputHandle) {
    const node = nodes.find(n => n.id === nodeId);
    if (!node) return null;

    const nodeType = node.data.definition?.node_type;
    
    // Only nodes with bidirectional constraints have constrained outputs based on inputs
    if (!BIDIRECTIONAL_CONSTRAINT_NODES.includes(nodeType)) {
      return null;
    }

    // Check the output type - only constrain "Any" type outputs
    const output = (node.data.definition?.outputs || []).find(o => o.id === outputHandle);
    if (!output || output.value_type?.type !== 'Any') {
      return null;
    }

    // For bidirectional constraint nodes (like Branch), the output type is constrained by the inputs.
    // Pass checkOutputs=false to prevent infinite recursion:
    // Without this flag, we would have: getConstrainedOutputType -> getConstrainedTypeFromConnectedInputs
    // -> (if checkOutputs=true) getConstrainedTypeFromConnectedOutputs -> getConstrainedOutputType (infinite loop)
    return getConstrainedTypeFromConnectedInputs(nodeId, null, null, false);
  }

  /**
   * Check if a source type is compatible with a target type, considering dynamic constraints.
   * This is a reusable function for type matching that handles:
   * - Execution flow type (must match exactly - only Execution to Execution)
   * - Any type with optional constraints from other connected inputs
   * - Enum compatibility checking
   * - Standard type matching
   * 
   * @param sourceValueType - The value_type object from the source output
   * @param targetValueType - The value_type object from the target input
   * @param constrainedType - Optional type constraint from other connected inputs (for target)
   * @param allowedTypes - Optional array of allowed types (for nodes like Evaluate Number that only accept Float/Integer)
   * @param sourceConstrainedType - Optional type constraint from connected inputs for source output (for nodes like Branch)
   * @returns true if the types are compatible
   */
  function areTypesCompatible(sourceValueType, targetValueType, constrainedType = null, allowedTypes = null, sourceConstrainedType = null) {
    const sourceType = sourceValueType?.type;
    const targetType = targetValueType?.type;

    if (!sourceType || !targetType) {
      return false;
    }

    // Execution type must match exactly - can only connect Execution to Execution
    if (sourceType === 'Execution' || targetType === 'Execution') {
      return sourceType === 'Execution' && targetType === 'Execution';
    }

    // CauseReason type must match exactly - can only connect CauseReason to CauseReason
    // This avoids the need for enum value matching since CauseReason is its own distinct type
    if (sourceType === 'CauseReason' || targetType === 'CauseReason') {
      return sourceType === 'CauseReason' && targetType === 'CauseReason';
    }

    // Handle "Any" target type with potential constraints
    if (targetType === 'Any') {
      // If there's a constraint from other connected inputs, source must match it
      if (constrainedType) {
        // If source is also Any with a constraint, compare constraints
        if (sourceType === 'Any' && sourceConstrainedType) {
          if ((constrainedType.type === 'Enum' || constrainedType.type === 'EnumWithIds') && 
              (sourceConstrainedType.type === 'Enum' || sourceConstrainedType.type === 'EnumWithIds')) {
            return areEnumsCompatible(sourceConstrainedType, constrainedType);
          }
          return sourceConstrainedType.type === constrainedType.type;
        }
        // If source is Any without constraint, it's compatible
        if (sourceType === 'Any') {
          return true;
        }
        if (constrainedType.type === 'Enum' || constrainedType.type === 'EnumWithIds') {
          return (sourceType === 'Enum' || sourceType === 'EnumWithIds') && areEnumsCompatible(sourceValueType, constrainedType);
        }
        return sourceType === constrainedType.type;
      }
      
      // If there are allowedTypes specified, source must be one of them
      if (allowedTypes) {
        // If source is Any with a constraint, check constraint against allowed types
        if (sourceType === 'Any' && sourceConstrainedType) {
          return allowedTypes.includes(sourceConstrainedType.type);
        }
        // If source is Any without constraint, it's compatible (will be constrained by first connection)
        if (sourceType === 'Any') {
          return true;
        }
        return allowedTypes.includes(sourceType);
      }
      
      // No constraint - accept anything (except Execution and CauseReason which are handled above)
      return true;
    }

    // Handle "Any" source type with potential constraint
    if (sourceType === 'Any') {
      // If source has a constraint from its inputs, use that constraint
      if (sourceConstrainedType) {
        if (targetType === 'Enum' || targetType === 'EnumWithIds') {
          return (sourceConstrainedType.type === 'Enum' || sourceConstrainedType.type === 'EnumWithIds') && areEnumsCompatible(sourceConstrainedType, targetValueType);
        }
        return sourceConstrainedType.type === targetType;
      }
      // No constraint on source - it can connect to anything
      return true;
    }

    // Handle Object type - can connect to anything
    if (sourceType === 'Object' || targetType === 'Object') {
      return true;
    }

    // Types must match exactly
    if (sourceType !== targetType) {
      return false;
    }

    // For Enum and EnumWithIds types, additionally check that enum values are compatible
    if (sourceType === 'Enum' || sourceType === 'EnumWithIds') {
      return areEnumsCompatible(sourceValueType, targetValueType);
    }

    return true;
  }

  /**
   * Get allowed types for a node's "Any" type inputs based on node type.
   * Some nodes (like Evaluate Number) only accept specific types even though
   * they use "Any" type for dynamic matching.
   * 
   * @param nodeType - The node type identifier
   * @returns Array of allowed type strings, or null if all types are allowed
   */
  function getAllowedTypesForNode(nodeType) {
    switch (nodeType) {
      case 'logic_evaluate_number':
        // Evaluate Number only accepts Float or Integer
        return ['Float', 'Integer'];
      case 'logic_equals':
        // Equals accepts all types
        return null;
      case 'logic_branch':
        // Branch accepts all types for its Any-typed inputs (True/False).
        // Type constraints between inputs and output are handled by getConstrainedOutputType.
        return null;
      default:
        return null;
    }
  }

  // Check if a connection is valid based on type compatibility
  function isValidConnection(connection) {
    const details = getConnectionDetails(connection);
    if (!details) return false;

    const { sourceNode, sourceOutput, targetInput, targetNode } = details;

    // Check if the target handle already has a connection
    // Execution type inputs can have multiple connections (multiple paths can trigger the same action)
    // Data inputs can only have one connection
    const existingConnection = edges.find(
      e => e.target === connection.target && e.targetHandle === connection.targetHandle
    );
    if (existingConnection && targetInput.value_type?.type !== 'Execution') {
      return false;
    }

    // Check if types are compatible using the unified type matching function
    const targetType = targetInput.value_type?.type;
    const sourceType = sourceOutput.value_type?.type;
    
    // For nodes with "Any" type inputs, get the constrained type from other connected inputs
    let constrainedType = null;
    let allowedTypes = null;
    
    if (targetType === 'Any') {
      const nodeType = targetNode.data.definition?.node_type;
      allowedTypes = getAllowedTypesForNode(nodeType);
      constrainedType = getConstrainedTypeFromConnectedInputs(
        connection.target, 
        connection.targetHandle,
        allowedTypes
      );
    }

    // For nodes with "Any" type outputs (like Branch), get the constrained type from connected inputs
    let sourceConstrainedType = null;
    if (sourceType === 'Any') {
      sourceConstrainedType = getConstrainedOutputType(
        connection.source,
        connection.sourceHandle
      );
    }

    return areTypesCompatible(
      sourceOutput.value_type, 
      targetInput.value_type, 
      constrainedType,
      allowedTypes,
      sourceConstrainedType
    );
  }

  /**
   * Generate an error message for an invalid connection based on type constraints.
   * 
   * @param connection - The connection being attempted
   * @returns Error message string describing why the connection is invalid
   */
  function getConnectionErrorMessage(connection) {
    const details = getConnectionDetails(connection);
    if (!details) return null;
    
    const { sourceOutput, targetInput, targetNode } = details;
    const sourceType = sourceOutput.value_type?.type;
    const targetType = targetInput.value_type?.type;
    
    // Check if source output is constrained by its inputs (for nodes like Branch)
    if (sourceType === 'Any') {
      const sourceConstrainedType = getConstrainedOutputType(
        connection.source,
        connection.sourceHandle
      );
      if (sourceConstrainedType && targetType !== 'Any' && sourceConstrainedType.type !== targetType) {
        return `⚠ Output constrained to ${sourceConstrainedType.type} by input connections`;
      }
    }
    
    // Check if this is a type constraint violation for "Any" type inputs
    if (targetType === 'Any') {
      const nodeType = targetNode.data.definition?.node_type;
      const allowedTypes = getAllowedTypesForNode(nodeType);
      const constrainedType = getConstrainedTypeFromConnectedInputs(
        connection.target, 
        connection.targetHandle,
        allowedTypes
      );
      
      if (constrainedType) {
        return `⚠ Input constrained to ${constrainedType.type} by other connection`;
      } else if (allowedTypes && !allowedTypes.includes(sourceType)) {
        return `⚠ Only ${allowedTypes.join(' or ')} types allowed`;
      }
    }
    
    // Check for enum type mismatch (both Enum and EnumWithIds)
    if ((sourceType === 'Enum' || sourceType === 'EnumWithIds') && 
        (targetType === 'Enum' || targetType === 'EnumWithIds')) {
      return `⚠ Incompatible enum types`;
    }
    
    // Default type mismatch message
    return `⚠ Type mismatch: ${sourceType} → ${targetType}`;
  }

  function onConnect(connection) {
    // Validate connection using isValidConnection
    if (!isValidConnection(connection)) {
      const errorMessage = getConnectionErrorMessage(connection);
      if (errorMessage) {
        saveStatus = errorMessage;
        setTimeout(() => saveStatus = '', 3000);
      }
      return;
    }

    // Get connection details for edge creation
    const details = getConnectionDetails(connection);
    if (!details) return;

    const { sourceOutput } = details;

    // Create the edge with reconnectable enabled
    edges = [...edges, { 
      ...connection, 
      id: `e${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}`,
      animated: true,
      reconnectable: true,
      type: 'default',
      style: `stroke: ${sourceOutput.color}; stroke-width: 2px;`
    }];
    hasUnsavedChanges = true;
  }

  // Handle edge reconnection
  function onReconnect(oldEdge, newConnection) {
    // Validate the new connection
    if (!isValidConnection(newConnection)) {
      const errorMessage = getConnectionErrorMessage(newConnection);
      if (errorMessage) {
        saveStatus = errorMessage;
        setTimeout(() => saveStatus = '', 3000);
      }
      return;
    }

    // Get connection details for edge styling
    const details = getConnectionDetails(newConnection);
    if (!details) return;

    const { sourceOutput } = details;

    // Update edges: remove old edge, add new one
    edges = edges.filter(e => e.id !== oldEdge.id);
    edges = [...edges, {
      ...newConnection,
      id: `e${newConnection.source}-${newConnection.sourceHandle}-${newConnection.target}-${newConnection.targetHandle}`,
      animated: true,
      reconnectable: true,
      type: 'default',
      style: `stroke: ${sourceOutput.color}; stroke-width: 2px;`
    }];
    hasUnsavedChanges = true;
  }

  // Handle reconnection end - if dropped without a valid target, remove the edge
  function onReconnectEnd(event, edge, handleType) {
    // Check if the edge still has valid connections
    // If the user right-clicked or dropped without connecting, we should remove the edge
    // The edge will be removed if it was being reconnected and dropped on empty space
    
    // We use a small timeout to check if a new connection was made
    setTimeout(() => {
      // Check if the original edge still exists (wasn't replaced by onReconnect)
      const originalEdge = edges.find(e => e.id === edge.id);
      if (originalEdge) {
        // Edge still exists, check if user canceled (right-click usually cancels)
        // For now we keep the edge - the user can use context menu to delete
      }
    }, 100);
  }

  // Handle deletion validation - called before delete to confirm or block
  async function onBeforeDelete({ nodes: nodesToDelete, edges: edgesToDelete }) {
    // Check if any nodes to delete are default nodes
    const defaultNodes = nodesToDelete.filter(n => n.data?.isDefault);
    if (defaultNodes.length > 0) {
      const defaultNodeNames = defaultNodes.map(getNodeDisplayName).join(', ');
      alert(`Cannot delete default nodes: ${defaultNodeNames}`);
      return false; // Block deletion
    }

    // Build confirmation message
    let message = '';
    if (nodesToDelete.length > 0) {
      const nodeNames = nodesToDelete.map(getNodeDisplayName).join(', ');
      message += `Delete ${nodesToDelete.length} node(s)?\n\n${nodeNames}`;
    }
    if (edgesToDelete.length > 0) {
      if (message) message += '\n\n';
      message += `Delete ${edgesToDelete.length} connection(s)?`;
    }

    // Return true to allow deletion, false to block
    return message ? confirm(message) : true;
  }
</script>

<div class="node-editor-container">
  <div class="toolbar">
    <div class="toolbar-header">
      <h1>🔧 Node-Based AC Logic Editor</h1>
      <div class="profile-info">
        <span class="profile-label">Active Profile:</span>
        <span class="profile-name">{activeNodesetId === NEW_NODESET_ID ? 'None' : `${activeNodesetId} - ${activeNodesetName}`}</span>
      </div>
    </div>
    <div class="toolbar-controls">
      <div class="profile-selector">
        <label for="nodeset-select">Editing:</label>
        <select 
          id="nodeset-select" 
          bind:value={selectedNodesetId} 
          onchange={handleNodesetChange}
          class="nodeset-dropdown"
        >
          <option value={NEW_NODESET_ID}>New Profile</option>
          {#each nodesets as nodeset}
            <option value={nodeset.id}>{nodeset.id} - {nodeset.name}</option>
          {/each}
        </select>
        {#if hasUnsavedChanges}
          <span class="unsaved-indicator" title="Unsaved changes">●</span>
        {/if}
      </div>
      <div class="toolbar-buttons">
        <button onclick={activateProfile} class="btn btn-activate" disabled={selectedNodesetId === NEW_NODESET_ID}>
          ⚡ Activate
        </button>
        <button onclick={saveConfiguration} class="btn btn-save">
          💾 Save
        </button>
        <button onclick={saveAsNodeset} class="btn btn-saveas">
          📝 Save As
        </button>
        <button onclick={deleteNodeset} class="btn btn-delete" disabled={currentNodesetId < 1}>
          🗑️ Delete
        </button>
        <a href="/" class="btn btn-back" onclick={handleBackClick}>← Back</a>
      </div>
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
      <div class="flow-container" onclick={closeContextMenu} onkeydown={handleContextMenuKeyDown} role="application">
        <SvelteFlow 
          bind:nodes
          bind:edges
          {nodeTypes}
          {edgeTypes}
          onnodeschange={onNodesChange}
          onedgeschange={onEdgesChange}
          onconnect={onConnect}
          onreconnect={onReconnect}
          onreconnectend={onReconnectEnd}
          onbeforedelete={onBeforeDelete}
          isValidConnection={isValidConnection}
          onnodecontextmenu={handleNodeContextMenu}
          onedgecontextmenu={handleEdgeContextMenu}
          onpanecontextmenu={handlePaneContextMenu}
          deleteKeyCode="Delete"
          fitView
          selectionKey="Control"
          selectionMode="partial"
          clickConnect={true}
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
            {#if contextMenu.type === 'node'}
              <button onclick={deleteNodeFromMenu} class="context-menu-item">
                🗑️ Delete Node
              </button>
            {:else if contextMenu.type === 'edge'}
              <button onclick={deleteEdgeFromMenu} class="context-menu-item">
                🗑️ Delete Connection
              </button>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
    
    <!-- Cause Reasons Panel on the right -->
    <CauseReasonsPanel onCauseReasonsChanged={handleCauseReasonsChanged} />
  </div>
  
  <!-- Simulator Drawer -->
  <SimulatorDrawer 
    bind:isOpen={simulatorOpen} 
    currentNodesetId={currentNodesetId}
    nodes={nodes}
    edges={edges}
  />
</div>

<style>
  .node-editor-container {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #1a1a1a;
    overflow: hidden;
  }

  .toolbar {
    background: #2d2d2d;
    padding: 0.75rem 1rem;
    border-bottom: 2px solid #404040;
    box-shadow: 0 2px 4px rgba(0,0,0,0.3);
  }

  .toolbar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .toolbar h1 {
    margin: 0;
    font-size: 1.3rem;
    color: #e0e0e0;
  }

  .profile-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: #1a1a1a;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    border: 1px solid #404040;
  }

  .profile-label {
    font-size: 0.85rem;
    color: #888;
  }

  .profile-name {
    font-size: 0.95rem;
    font-weight: 600;
    color: #00BCD4;
  }

  .unsaved-indicator {
    color: #FF9800;
    font-size: 1.2rem;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .toolbar-controls {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .profile-selector {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .profile-selector label {
    font-size: 0.85rem;
    color: #aaa;
  }

  .nodeset-dropdown {
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
    border: 1px solid #404040;
    background: #1a1a1a;
    color: #e0e0e0;
    font-size: 0.9rem;
    cursor: pointer;
    min-width: 150px;
  }

  .nodeset-dropdown:focus {
    outline: none;
    border-color: #00BCD4;
  }

  .toolbar-buttons {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .btn {
    padding: 0.4rem 0.75rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 500;
    transition: all 0.2s;
    text-decoration: none;
    display: inline-block;
  }

  .btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0,0,0,0.2);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-activate {
    background: #4CAF50;
    color: white;
  }

  .btn-save {
    background: #00BCD4;
    color: white;
  }

  .btn-saveas {
    background: #9C27B0;
    color: white;
  }

  .btn-delete {
    background: #F44336;
    color: white;
  }

  .btn-back {
    background: #757575;
    color: white;
  }

  .save-status {
    display: block;
    margin-top: 0.5rem;
    font-weight: 500;
    color: #e0e0e0;
    font-size: 0.9rem;
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

  /* Ensure edges render behind nodes */
  :global(.svelte-flow__edges) {
    z-index: 0 !important;
  }

  :global(.svelte-flow__nodes) {
    z-index: 1 !important;
  }

  :global(.svelte-flow__edge-path) {
    stroke: #b1b1b7;
    stroke-width: 2;
  }

  /* Selected edge styling - more prominent highlight */
  :global(.svelte-flow__edge.selected .svelte-flow__edge-path) {
    stroke-width: 4 !important;
    filter: drop-shadow(0 0 6px currentColor) drop-shadow(0 0 12px currentColor);
  }

  /* Hover effect for edges */
  :global(.svelte-flow__edge:hover .svelte-flow__edge-path) {
    stroke-width: 3;
    filter: drop-shadow(0 0 4px currentColor);
  }

  :global(.svelte-flow__edge.animated path) {
    stroke-dasharray: 5;
    animation: dashdraw 0.5s linear infinite;
  }

  /* Selection box styling - make it only select nodes visually */
  :global(.svelte-flow__selection) {
    background: rgba(0, 188, 212, 0.1) !important;
    border: 1px dashed rgba(0, 188, 212, 0.6) !important;
  }

  :global(.svelte-flow__nodesselection-rect) {
    background: rgba(0, 188, 212, 0.15) !important;
    border: 2px solid rgba(0, 188, 212, 0.8) !important;
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
