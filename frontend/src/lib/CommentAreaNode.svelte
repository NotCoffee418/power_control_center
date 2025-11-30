<script>
  import { NodeResizer } from '@xyflow/svelte';

  // Props passed by SvelteFlow
  let { data, id, selected } = $props();

  // Initialize the label/title with a default value
  let label = $state(data?.label || 'Comment');
  
  // Comment/description text
  let description = $state(data?.description || '');
  
  // Color for the comment area (customizable)
  let color = $state(data?.color || '#4a5568');
  
  // Sync state changes back to node data for persistence
  $effect(() => {
    if (data) {
      if (data.label !== label) {
        data.label = label;
      }
      if (data.description !== description) {
        data.description = description;
      }
      if (data.color !== color) {
        data.color = color;
      }
    }
  });

  // Handle label input change
  function handleLabelChange(event) {
    label = event.target.value;
  }

  // Handle description input change
  function handleDescriptionChange(event) {
    description = event.target.value;
  }

  // Available colors for the comment area
  const colorOptions = [
    { value: '#4a5568', label: 'Gray' },
    { value: '#2d3748', label: 'Dark Gray' },
    { value: '#744210', label: 'Brown' },
    { value: '#22543d', label: 'Green' },
    { value: '#234e52', label: 'Teal' },
    { value: '#2a4365', label: 'Blue' },
    { value: '#44337a', label: 'Purple' },
    { value: '#702459', label: 'Pink' },
    { value: '#742a2a', label: 'Red' },
    { value: '#7b341e', label: 'Orange' }
  ];

  // Handle color change
  function handleColorChange(event) {
    color = event.target.value;
  }
</script>

<NodeResizer 
  minWidth={200} 
  minHeight={100}
  isVisible={selected}
/>

<div 
  class="comment-area-node"
  class:selected={selected}
  style="background-color: {color};"
>
  <div class="comment-header">
    <input
      type="text"
      class="comment-title"
      value={label}
      oninput={handleLabelChange}
      placeholder="Comment title..."
      title="Comment area title"
    />
    <select 
      class="color-picker"
      value={color}
      onchange={handleColorChange}
      title="Change color"
    >
      {#each colorOptions as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  </div>
  
  <textarea
    class="comment-description"
    value={description}
    oninput={handleDescriptionChange}
    placeholder="Add description or notes here..."
    title="Comment description"
  ></textarea>
</div>

<style>
  .comment-area-node {
    width: 100%;
    height: 100%;
    min-width: 200px;
    min-height: 100px;
    border-radius: 8px;
    border: 2px dashed rgba(255, 255, 255, 0.3);
    padding: 10px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition: border-color 0.2s;
  }

  .comment-area-node.selected {
    border-color: #FFD700;
    border-style: solid;
    box-shadow: 0 0 10px rgba(255, 215, 0, 0.3);
  }

  .comment-header {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .comment-title {
    flex: 1;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    padding: 6px 10px;
    color: white;
    font-size: 14px;
    font-weight: 600;
  }

  .comment-title:focus {
    outline: none;
    border-color: rgba(255, 255, 255, 0.4);
  }

  .comment-title::placeholder {
    color: rgba(255, 255, 255, 0.5);
  }

  .color-picker {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    padding: 4px 8px;
    color: white;
    font-size: 12px;
    cursor: pointer;
    min-width: 80px;
  }

  .color-picker:focus {
    outline: none;
    border-color: rgba(255, 255, 255, 0.4);
  }

  .color-picker option {
    background: #2d2d2d;
    color: white;
  }

  .comment-description {
    flex: 1;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    padding: 8px;
    color: rgba(255, 255, 255, 0.9);
    font-size: 12px;
    resize: none;
    min-height: 40px;
  }

  .comment-description:focus {
    outline: none;
    border-color: rgba(255, 255, 255, 0.3);
  }

  .comment-description::placeholder {
    color: rgba(255, 255, 255, 0.4);
    font-style: italic;
  }

  /* Style for the NodeResizer handles */
  :global(.svelte-flow__resize-control) {
    background: #FFD700 !important;
    border-color: #FFD700 !important;
  }

  :global(.svelte-flow__resize-control.line) {
    border-color: rgba(255, 215, 0, 0.5) !important;
  }
</style>
