# Copilot Instructions for Power Control Center

## Build Instructions

This project consists of a Rust backend with an embedded Svelte frontend. **The frontend MUST be built before compiling the Rust project.**

### Frontend Build (Required First)

The frontend is located in the `frontend/` directory and must be built before the Rust compilation:

```bash
cd frontend
npm install
npm run build
```

This creates the `frontend/dist/` directory with compiled assets that are embedded into the Rust binary using `rust-embed`.

### Rust Build

After the frontend is built, you can compile the Rust project:

```bash
cargo build
```

Or for a release build:

```bash
cargo build --release
```

### Common Build Errors

**Error: `folder 'frontend/dist/' does not exist`**
- **Cause**: The frontend hasn't been built yet
- **Solution**: Build the frontend first (see Frontend Build section above)

**Error: `no function or associated item named 'get' found for struct 'Static'`**
- **Cause**: This error appears when the `frontend/dist/` folder doesn't exist
- **Solution**: Build the frontend first

### Complete Build Process

To build the entire project from scratch:

```bash
# 1. Build frontend
cd frontend && npm install && npm run build && cd ..

# 2. Build Rust project
cargo build
```

### Development Workflow

For development with live reload:

1. **Frontend development** (in one terminal):
   ```bash
   cd frontend
   npm run dev
   ```

2. **Backend development** (in another terminal):
   ```bash
   # Build frontend once first
   cd frontend && npm run build && cd ..
   
   # Then run/develop the Rust backend
   cargo run
   ```

## Project Structure

- `frontend/` - Svelte + Vite frontend application
- `frontend/dist/` - Built frontend assets (generated, embedded into Rust binary)
- `src/` - Rust source code
- `src/webserver/router.rs` - Embeds frontend/dist/ using rust-embed
