<script>
  import { Handle, Position } from '@xyflow/svelte';

  // Props passed by SvelteFlow
  let { data, id, selected } = $props();

  const definition = data.definition;
  const inputs = definition?.inputs || [];
  const outputs = definition?.outputs || [];
  const color = definition?.color || '#757575';
  const isDefault = data.isDefault || false;

  // No longer needed - handles will align naturally with their port rows

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
  </div>
  
  <div class="node-content">
    <!-- Input handles on the left -->
    {#each inputs as input, i}
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

    <!-- If no inputs or outputs, show a message -->
    {#if inputs.length === 0 && outputs.length === 0}
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
  }

  .node-title {
    font-weight: 600;
    font-size: 14px;
  }

  .default-badge {
    font-size: 12px;
    opacity: 0.8;
  }

  .node-content {
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
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
