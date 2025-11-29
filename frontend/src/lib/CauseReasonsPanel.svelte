<script>
  // Props
  let { onCauseReasonsChanged = () => {} } = $props();

  // Constants
  const DESCRIPTION_TRUNCATE_LENGTH = 100;

  // State
  let causeReasons = $state([]);
  let loading = $state(true);
  let showHidden = $state(false);
  let editingId = $state(null);
  let editLabel = $state('');
  let editDescription = $state('');
  let showAddForm = $state(false);
  let newLabel = $state('');
  let newDescription = $state('');
  let statusMessage = $state('');

  // Load cause reasons from API
  async function loadCauseReasons() {
    try {
      const endpoint = showHidden ? '/api/cause-reasons/all' : '/api/cause-reasons';
      const response = await fetch(endpoint);
      const result = await response.json();
      
      if (result.success && result.data) {
        causeReasons = result.data;
      } else {
        console.error('Failed to load cause reasons:', result.error);
      }
    } catch (e) {
      console.error('Error loading cause reasons:', e);
    } finally {
      loading = false;
    }
  }

  // Toggle hidden status
  async function toggleHidden(id, currentlyHidden) {
    if (id === 0) {
      statusMessage = '⚠ Cannot hide the Undefined cause reason';
      setTimeout(() => statusMessage = '', 3000);
      return;
    }
    
    try {
      const response = await fetch(`/api/cause-reasons/${id}/hidden`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ is_hidden: !currentlyHidden })
      });
      
      const result = await response.json();
      
      if (result.success) {
        await loadCauseReasons();
        onCauseReasonsChanged();
        statusMessage = currentlyHidden ? '✓ Shown' : '✓ Hidden';
        setTimeout(() => statusMessage = '', 2000);
      } else {
        statusMessage = '✗ ' + (result.error || 'Failed to update');
        setTimeout(() => statusMessage = '', 3000);
      }
    } catch (e) {
      console.error('Error toggling hidden status:', e);
      statusMessage = '✗ Error updating status';
      setTimeout(() => statusMessage = '', 3000);
    }
  }

  // Start editing a cause reason
  function startEditing(reason) {
    editingId = reason.id;
    editLabel = reason.label;
    editDescription = reason.description;
  }

  // Cancel editing
  function cancelEditing() {
    editingId = null;
    editLabel = '';
    editDescription = '';
  }

  // Save edited cause reason
  async function saveEditing() {
    if (!editLabel.trim() || !editDescription.trim()) {
      statusMessage = '⚠ Label and description are required';
      setTimeout(() => statusMessage = '', 3000);
      return;
    }
    
    try {
      const response = await fetch(`/api/cause-reasons/${editingId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ label: editLabel, description: editDescription })
      });
      
      const result = await response.json();
      
      if (result.success) {
        await loadCauseReasons();
        onCauseReasonsChanged();
        cancelEditing();
        statusMessage = '✓ Saved';
        setTimeout(() => statusMessage = '', 2000);
      } else {
        statusMessage = '✗ ' + (result.error || 'Failed to save');
        setTimeout(() => statusMessage = '', 3000);
      }
    } catch (e) {
      console.error('Error saving cause reason:', e);
      statusMessage = '✗ Error saving';
      setTimeout(() => statusMessage = '', 3000);
    }
  }

  // Create new cause reason
  async function createCauseReason() {
    if (!newLabel.trim() || !newDescription.trim()) {
      statusMessage = '⚠ Label and description are required';
      setTimeout(() => statusMessage = '', 3000);
      return;
    }
    
    try {
      const response = await fetch('/api/cause-reasons', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ label: newLabel, description: newDescription })
      });
      
      const result = await response.json();
      
      if (result.success) {
        await loadCauseReasons();
        onCauseReasonsChanged();
        newLabel = '';
        newDescription = '';
        showAddForm = false;
        statusMessage = '✓ Created';
        setTimeout(() => statusMessage = '', 2000);
      } else {
        statusMessage = '✗ ' + (result.error || 'Failed to create');
        setTimeout(() => statusMessage = '', 3000);
      }
    } catch (e) {
      console.error('Error creating cause reason:', e);
      statusMessage = '✗ Error creating';
      setTimeout(() => statusMessage = '', 3000);
    }
  }

  // Delete cause reason
  async function deleteCauseReason(id) {
    if (id === 0) {
      statusMessage = '⚠ Cannot delete the Undefined cause reason';
      setTimeout(() => statusMessage = '', 3000);
      return;
    }
    
    if (!confirm('Delete this cause reason? This cannot be undone.')) {
      return;
    }
    
    try {
      const response = await fetch(`/api/cause-reasons/${id}`, {
        method: 'DELETE'
      });
      
      const result = await response.json();
      
      if (result.success) {
        await loadCauseReasons();
        onCauseReasonsChanged();
        statusMessage = '✓ Deleted';
        setTimeout(() => statusMessage = '', 2000);
      } else {
        statusMessage = '✗ ' + (result.error || 'Failed to delete');
        setTimeout(() => statusMessage = '', 3000);
      }
    } catch (e) {
      console.error('Error deleting cause reason:', e);
      statusMessage = '✗ Error deleting';
      setTimeout(() => statusMessage = '', 3000);
    }
  }

  // React to showHidden changes - loads cause reasons on mount and when showHidden changes
  $effect(() => {
    // Access showHidden to create a dependency
    const _ = showHidden;
    loadCauseReasons();
  });
</script>

<div class="cause-reasons-panel">
  <div class="panel-header">
    <h3>📋 Cause Reasons</h3>
    <button 
      class="btn-add" 
      onclick={() => showAddForm = !showAddForm}
      title="Add new cause reason"
    >
      {showAddForm ? '✕' : '+'}
    </button>
  </div>
  
  <div class="panel-controls">
    <label class="show-hidden-toggle">
      <input type="checkbox" bind:checked={showHidden} />
      <span>Show Hidden</span>
    </label>
  </div>
  
  {#if statusMessage}
    <div class="status-message">{statusMessage}</div>
  {/if}
  
  {#if showAddForm}
    <div class="add-form">
      <input
        type="text"
        placeholder="Label"
        bind:value={newLabel}
        class="form-input"
      />
      <textarea
        placeholder="Description"
        bind:value={newDescription}
        class="form-textarea"
        rows="3"
      ></textarea>
      <div class="form-buttons">
        <button class="btn-cancel" onclick={() => { showAddForm = false; newLabel = ''; newDescription = ''; }}>
          Cancel
        </button>
        <button class="btn-create" onclick={createCauseReason}>
          Create
        </button>
      </div>
    </div>
  {/if}
  
  <div class="cause-reasons-list">
    {#if loading}
      <p class="loading-text">Loading...</p>
    {:else if causeReasons.length === 0}
      <p class="empty-text">No cause reasons found</p>
    {:else}
      {#each causeReasons as reason}
        <div class="cause-reason-item" class:hidden={reason.is_hidden}>
          {#if editingId === reason.id}
            <div class="edit-form">
              <input
                type="text"
                bind:value={editLabel}
                class="form-input"
                placeholder="Label"
              />
              <textarea
                bind:value={editDescription}
                class="form-textarea"
                rows="3"
                placeholder="Description"
              ></textarea>
              <div class="form-buttons">
                <button class="btn-cancel" onclick={cancelEditing}>Cancel</button>
                <button class="btn-save" onclick={saveEditing}>Save</button>
              </div>
            </div>
          {:else}
            <div class="reason-header">
              <span class="reason-id">#{reason.id}</span>
              <span class="reason-label">{reason.label}</span>
            </div>
            <div class="reason-description" title={reason.description}>
              {reason.description.length > DESCRIPTION_TRUNCATE_LENGTH ? reason.description.substring(0, DESCRIPTION_TRUNCATE_LENGTH) + '...' : reason.description}
            </div>
            <div class="reason-actions">
              <button
                class="btn-action btn-edit"
                onclick={() => startEditing(reason)}
                title="Edit"
              >
                ✏️
              </button>
              <button
                class="btn-action btn-hide"
                onclick={() => toggleHidden(reason.id, reason.is_hidden)}
                title={reason.is_hidden ? 'Show' : 'Hide'}
                disabled={reason.id === 0}
              >
                {reason.is_hidden ? '👁️' : '🙈'}
              </button>
              <button
                class="btn-action btn-delete"
                onclick={() => deleteCauseReason(reason.id)}
                title="Delete"
                disabled={reason.id === 0}
              >
                🗑️
              </button>
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .cause-reasons-panel {
    width: 280px;
    background: #2d2d2d;
    border-left: 2px solid #404040;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: #252525;
    border-bottom: 1px solid #404040;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 1rem;
    color: #e0e0e0;
  }

  .btn-add {
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 50%;
    background: #4CAF50;
    color: white;
    font-size: 1.2rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }

  .btn-add:hover {
    background: #45a049;
    transform: scale(1.1);
  }

  .panel-controls {
    padding: 0.5rem 1rem;
    border-bottom: 1px solid #404040;
  }

  .show-hidden-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    color: #aaa;
    cursor: pointer;
  }

  .show-hidden-toggle input {
    cursor: pointer;
  }

  .status-message {
    padding: 0.5rem 1rem;
    font-size: 0.85rem;
    color: #e0e0e0;
    background: rgba(0, 0, 0, 0.2);
  }

  .add-form, .edit-form {
    padding: 0.75rem;
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid #404040;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-input {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #404040;
    border-radius: 4px;
    background: #1a1a1a;
    color: #e0e0e0;
    font-size: 0.85rem;
    box-sizing: border-box;
  }

  .form-textarea {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #404040;
    border-radius: 4px;
    background: #1a1a1a;
    color: #e0e0e0;
    font-size: 0.85rem;
    resize: vertical;
    min-height: 60px;
    box-sizing: border-box;
  }

  .form-input:focus, .form-textarea:focus {
    outline: none;
    border-color: #00BCD4;
  }

  .form-buttons {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }

  .btn-cancel, .btn-create, .btn-save {
    padding: 0.4rem 0.75rem;
    border: none;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-cancel {
    background: #666;
    color: white;
  }

  .btn-cancel:hover {
    background: #777;
  }

  .btn-create, .btn-save {
    background: #4CAF50;
    color: white;
  }

  .btn-create:hover, .btn-save:hover {
    background: #45a049;
  }

  .cause-reasons-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .loading-text, .empty-text {
    text-align: center;
    color: #888;
    padding: 1rem;
    font-size: 0.9rem;
  }

  .cause-reason-item {
    background: #1a1a1a;
    border: 1px solid #404040;
    border-radius: 6px;
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    transition: all 0.2s;
  }

  .cause-reason-item:hover {
    border-color: #555;
  }

  .cause-reason-item.hidden {
    opacity: 0.5;
    background: #222;
  }

  .reason-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .reason-id {
    font-size: 0.7rem;
    color: #666;
    background: #333;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
  }

  .reason-label {
    font-weight: 600;
    color: #e0e0e0;
    font-size: 0.9rem;
  }

  .reason-description {
    font-size: 0.75rem;
    color: #aaa;
    margin-bottom: 0.5rem;
    line-height: 1.4;
  }

  .reason-actions {
    display: flex;
    gap: 0.25rem;
    justify-content: flex-end;
  }

  .btn-action {
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .btn-action:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
  }

  .btn-action:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .btn-edit:hover:not(:disabled) {
    background: rgba(33, 150, 243, 0.3);
  }

  .btn-hide:hover:not(:disabled) {
    background: rgba(255, 152, 0, 0.3);
  }

  .btn-delete:hover:not(:disabled) {
    background: rgba(244, 67, 54, 0.3);
  }
</style>
