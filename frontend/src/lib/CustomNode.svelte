<script>
  import { Handle, Position } from '@xyflow/svelte';

  // Props passed by SvelteFlow
  let { data, id, selected } = $props();

  // Initialize constants first (these don't depend on state)
  const definition = data?.definition;
  const outputs = definition?.outputs || [];
  const color = definition?.color || '#757575';
  const isDefault = data?.isDefault || false;
  
  // Get node type safely
  const nodeType = definition?.node_type || '';

  // Determine node behavior flags
  const isDynamicLogicNode = ['logic_and', 'logic_or', 'logic_nand'].includes(nodeType);
  const isPrimitiveNode = ['primitive_float', 'primitive_integer', 'primitive_boolean'].includes(nodeType);
  const isEnumNode = ['device', 'intensity'].includes(nodeType);

  // Get default value based on primitive type
  function getDefaultPrimitiveValue() {
    if (nodeType === 'primitive_boolean') return false;
    return 0;
  }

  // Get default value for enum nodes (first enum value)
  function getDefaultEnumValue() {
    if (isEnumNode && outputs.length > 0) {
      const enumOutput = outputs[0];
      if (enumOutput?.value_type?.type === 'Enum' && enumOutput?.value_type?.value?.length > 0) {
        return enumOutput.value_type.value[0];
      }
    }
    return '';
  }

  // Initialize state variables after functions are defined
  let dynamicInputs = $state(data?.dynamicInputs || definition?.inputs || []);
  let primitiveValue = $state(data?.primitiveValue ?? getDefaultPrimitiveValue());
  let enumValue = $state(data?.enumValue ?? getDefaultEnumValue());
  let isValidInput = $state(true);

  // Sync state changes back to node data for persistence
  $effect(() => {
    if (isDynamicLogicNode && data) {
      data.dynamicInputs = dynamicInputs;
    }
    if (isPrimitiveNode && data) {
      data.primitiveValue = primitiveValue;
    }
    if (isEnumNode && data) {
      data.enumValue = enumValue;
    }
  });

  // Add a new input pin for dynamic logic nodes
  function addInput() {
    const nextIndex = dynamicInputs.length + 1;
    const newInput = {
      id: `input_${nextIndex}`,
      label: `Input ${nextIndex}`,
      description: `Boolean input ${nextIndex}`,
      value_type: { type: 'Boolean' },
      required: true,
      color: '#95E1D3' // Boolean color
    };
    dynamicInputs = [...dynamicInputs, newInput];
  }

  // Remove the last input pin (minimum 2)
  function removeInput() {
    if (dynamicInputs.length > 2) {
      dynamicInputs = dynamicInputs.slice(0, -1);
    }
  }

  // Validate and handle float input
  function handleFloatInput(event) {
    const value = event.target.value;
    const trimmed = value.trim();
    const parsed = parseFloat(trimmed);
    isValidInput = trimmed !== '' && !isNaN(parsed) && isFinite(parsed);
    primitiveValue = isValidInput ? parsed : value;
  }

  // Validate and handle integer input
  function handleIntegerInput(event) {
    const value = event.target.value;
    const trimmed = value.trim();
    const parsed = Number(trimmed);
    isValidInput = trimmed !== '' && Number.isInteger(parsed);
    primitiveValue = isValidInput ? parsed : value;
  }

  // Handle boolean toggle
  function handleBooleanToggle(event) {
    primitiveValue = event.target.checked;
  }

  // Handle enum selection change
  function handleEnumChange(event) {
    enumValue = event.target.value;
  }

  // Get enum options for dropdown
  function getEnumOptions() {
    if (outputs.length > 0) {
      const enumOutput = outputs[0];
      if (enumOutput?.value_type?.type === 'Enum' && enumOutput?.value_type?.value) {
        return enumOutput.value_type.value;
      }
    }
    return [];
  }

  // Get the inputs to display (either dynamic or static)
  function getDisplayInputs() {
    return isDynamicLogicNode ? dynamicInputs : (definition?.inputs || []);
  }
</script>

<div 
  class="custom-node"
  class:selected={selected}
  class:default-node={isDefault}
  style="background: {color};"
>
  <div class="node-header">
    <div class="node-title">{definition?.name || 'Node'}</div>
    {#if isDefault}
      <div class="default-badge">🔒</div>
    {/if}
    {#if isDynamicLogicNode}
      <div class="pin-controls">
        <button 
          class="pin-btn" 
          onclick={removeInput} 
          disabled={dynamicInputs.length <= 2}
          title="Remove input pin"
        >−</button>
        <button 
          class="pin-btn" 
          onclick={addInput}
          title="Add input pin"
        >+</button>
      </div>
    {/if}
  </div>
  
  <div class="node-content">
    <!-- Primitive node input field -->
    {#if isPrimitiveNode}
      <div class="primitive-input">
        {#if nodeType === 'primitive_float'}
          <input
            type="text"
            class="value-input"
            class:invalid={!isValidInput}
            value={primitiveValue}
            oninput={handleFloatInput}
            placeholder="0.0"
            title="Enter a decimal number"
          />
        {:else if nodeType === 'primitive_integer'}
          <input
            type="text"
            class="value-input"
            class:invalid={!isValidInput}
            value={primitiveValue}
            oninput={handleIntegerInput}
            placeholder="0"
            title="Enter a whole number"
          />
        {:else if nodeType === 'primitive_boolean'}
          <label class="checkbox-wrapper">
            <input
              type="checkbox"
              checked={primitiveValue}
              onchange={handleBooleanToggle}
            />
            <span class="checkbox-label">{primitiveValue ? 'True' : 'False'}</span>
          </label>
        {/if}
      </div>
    {/if}

    <!-- Enum node dropdown -->
    {#if isEnumNode}
      <div class="enum-input">
        <select 
          class="enum-select"
          value={enumValue}
          onchange={handleEnumChange}
        >
          {#each getEnumOptions() as option}
            <option value={option}>{option}</option>
          {/each}
        </select>
      </div>
    {/if}

    <!-- Input handles on the left -->
    {#each getDisplayInputs() as input, i}
      <div class="port-row input-port">
        <Handle
          type="target"
          position={Position.Left}
          id={input.id}
          style="background: {input.color}; border-color: {input.color};"
          class="custom-handle"
          title="{input.label} ({input.value_type.type})"
        />
        <div class="port-label" title={input.description}>
          {input.label}
          {#if input.required}
            <span class="required">*</span>
          {/if}
        </div>
      </div>
    {/each}

    <!-- Output handles on the right -->
    {#each outputs as output, i}
      <div class="port-row output-port">
        <div class="port-label" title={output.description}>
          {output.label}
        </div>
        <Handle
          type="source"
          position={Position.Right}
          id={output.id}
          style="background: {output.color}; border-color: {output.color};"
          class="custom-handle"
          title="{output.label} ({output.value_type.type})"
        />
      </div>
    {/each}

    <!-- If no inputs or outputs, show a message (for non-primitive/non-enum nodes) -->
    {#if !isPrimitiveNode && !isEnumNode && getDisplayInputs().length === 0 && outputs.length === 0}
      <div class="no-ports">No ports</div>
    {/if}
  </div>
</div>

<style>
  .custom-node {
    border-radius: 8px;
    color: white;
    min-width: 200px;
    max-width: 280px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    border: 2px solid transparent;
    transition: all 0.2s;
  }

  .custom-node.selected {
    border-color: #FFD700;
    box-shadow: 0 6px 12px rgba(0, 0, 0, 0.2);
  }

  .custom-node.default-node {
    border-style: dashed;
  }

  .node-header {
    padding: 10px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.2);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(0, 0, 0, 0.1);
    border-radius: 6px 6px 0 0;
    gap: 8px;
  }

  .node-title {
    font-weight: 600;
    font-size: 14px;
    flex: 1;
  }

  .default-badge {
    font-size: 12px;
    opacity: 0.8;
  }

  .pin-controls {
    display: flex;
    gap: 4px;
  }

  .pin-btn {
    width: 20px;
    height: 20px;
    border: none;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.2);
    color: white;
    font-size: 14px;
    font-weight: bold;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s;
  }

  .pin-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.4);
  }

  .pin-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .node-content {
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .primitive-input {
    margin-bottom: 4px;
  }

  .value-input {
    width: 100%;
    padding: 6px 8px;
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.2);
    color: white;
    font-size: 13px;
    box-sizing: border-box;
  }

  .value-input:focus {
    outline: none;
    border-color: rgba(255, 255, 255, 0.6);
  }

  .value-input.invalid {
    border-color: #FF6B6B;
    background: rgba(255, 107, 107, 0.2);
  }

  .checkbox-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    padding: 4px 0;
  }

  .checkbox-wrapper input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
    accent-color: #95E1D3;
  }

  .checkbox-label {
    font-size: 13px;
    font-weight: 500;
  }

  .enum-input {
    margin-bottom: 4px;
  }

  .enum-select {
    width: 100%;
    padding: 6px 8px;
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.3);
    color: white;
    font-size: 13px;
    box-sizing: border-box;
    cursor: pointer;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='white' d='M6 8L2 4h8z'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 8px center;
    padding-right: 24px;
  }

  .enum-select:focus {
    outline: none;
    border-color: rgba(255, 255, 255, 0.6);
  }

  .enum-select option {
    background: #2d2d2d;
    color: white;
  }

  .port-row {
    display: flex;
    align-items: center;
    position: relative;
    min-height: 28px;
    height: 28px;
    font-size: 12px;
  }

  .input-port {
    justify-content: flex-start;
    padding-left: 16px;
  }

  .output-port {
    justify-content: flex-end;
    padding-right: 16px;
  }

  .port-label {
    display: flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 20px;
  }

  .required {
    color: #FFD700;
    font-weight: bold;
    margin-left: 2px;
  }

  .no-ports {
    text-align: center;
    padding: 8px;
    font-style: italic;
    opacity: 0.7;
    font-size: 12px;
  }

  /* Fixed size handles that don't change on hover */
  :global(.custom-handle) {
    width: 10px !important;
    height: 10px !important;
    border-width: 2px;
    transition: none;
    cursor: crosshair;
  }

  /* Override default handle positioning to align with text */
  :global(.custom-handle.target) {
    left: -5px;
    transform: translateY(-50%);
    top: 50%;
    position: absolute;
  }

  :global(.custom-handle.source) {
    right: -5px;
    transform: translateY(-50%);
    top: 50%;
    position: absolute;
  }
</style>
