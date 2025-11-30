<script>
  import Dashboard from './lib/Dashboard.svelte'
  import NodeEditor from './lib/NodeEditor.svelte'
  
  // Simple client-side routing
  let currentRoute = $state(window.location.pathname);
  
  // Listen for navigation events
  function navigate(path) {
    window.history.pushState({}, '', path);
    currentRoute = path;
  }
  
  // Handle browser back/forward buttons
  window.addEventListener('popstate', () => {
    currentRoute = window.location.pathname;
  });
  
  // Intercept link clicks for SPA navigation
  window.addEventListener('click', (e) => {
    if (e.target.tagName === 'A' && e.target.href && e.target.href.startsWith(window.location.origin)) {
      e.preventDefault();
      navigate(new URL(e.target.href).pathname);
    }
  });
</script>

<main>
  {#if currentRoute === '/nodes'}
    <NodeEditor />
  {:else}
    <Dashboard />
  {/if}
</main>

<style>
  main {
    width: 100%;
  }
</style>
