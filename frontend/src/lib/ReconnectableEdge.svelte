<script>
  import { 
    BaseEdge, 
    EdgeReconnectAnchor,
    getBezierPath 
  } from '@xyflow/svelte';

  let {
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    style,
    markerEnd,
    markerStart,
    interactionWidth,
    data,
    selected
  } = $props();

  // Calculate the bezier path
  let [path, labelX, labelY, offsetX, offsetY] = $derived(
    getBezierPath({
      sourceX,
      sourceY,
      targetX,
      targetY,
      sourcePosition,
      targetPosition
    })
  );
  
  // Get edge color from style for endpoint circles
  let edgeColor = $derived(() => {
    if (!style) return '#b1b1b7';
    const match = style.match(/stroke:\s*([^;]+)/);
    return match ? match[1].trim() : '#b1b1b7';
  });
</script>

<BaseEdge 
  {id} 
  {path} 
  {style} 
  {markerEnd}
  {markerStart}
  {interactionWidth}
/>

<!-- Source reconnect anchor -->
<EdgeReconnectAnchor 
  type="source" 
  position={{ x: sourceX, y: sourceY }}
/>

<!-- Target reconnect anchor -->
<EdgeReconnectAnchor 
  type="target" 
  position={{ x: targetX, y: targetY }}
/>

<!-- Show endpoint circles when edge is selected for better visibility -->
{#if selected}
  <circle
    cx={sourceX}
    cy={sourceY}
    r={8}
    class="edge-endpoint edge-endpoint-selected"
    style="fill: {edgeColor()}; stroke: #FFD700;"
  />
  <circle
    cx={targetX}
    cy={targetY}
    r={8}
    class="edge-endpoint edge-endpoint-selected"
    style="fill: {edgeColor()}; stroke: #FFD700;"
  />
{/if}

<style>
  .edge-endpoint {
    pointer-events: none;
  }
  
  .edge-endpoint-selected {
    stroke-width: 3;
    filter: drop-shadow(0 0 4px #FFD700);
  }
</style>
