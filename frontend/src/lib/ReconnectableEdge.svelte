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
