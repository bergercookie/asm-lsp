# VSCode asm-lsp

## Development

### Setup your environment

```console
# Build asm-lsp
cargo build

# Setup JS
cd editors/code
npm install
```

### Debugging

In VSCode, go to the `Run & Debug` sidebar (Ctrl + Shft + D) and click the `Run Extension (Debug Build)`
button. This will open a new VSCode instance with the lsp server installed.

### Adding the binary inside VSIX

Copy the asm-lsp binary into `editors/code/bin/{platform}-{arch}/`.

```console
npm run package
```
