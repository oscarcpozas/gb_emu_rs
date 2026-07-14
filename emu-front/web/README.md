# Web build

Build the WASM package from the repository root:

```bash
rustup target add wasm32-unknown-unknown
cargo build -p gb-wasm --target wasm32-unknown-unknown --release
cargo install wasm-bindgen-cli
wasm-bindgen --target web --out-dir emu-front/web/pkg target/wasm32-unknown-unknown/release/gb_wasm.wasm
```

Install web dependencies:

```bash
cd emu-front/web
npm install
```

Serve the web directory locally:

```bash
npm run dev
```

Open the URL printed by `serve` and drop a local `.gb` or `.gbc` ROM into the page.
Audio is played through WebAudio after a ROM is loaded.

## Cloudflare Pages

The GitHub Actions workflow `.github/workflows/cloudflare-pages.yml` builds `gb-wasm`, generates `emu-front/web/pkg`, and deploys the `emu-front/web` directory to Cloudflare Pages.

Required GitHub secrets:

```text
CLOUDFLARE_API_TOKEN
CLOUDFLARE_ACCOUNT_ID
```

The workflow uses `gb-emulator` as the Cloudflare Pages project name. Change `CLOUDFLARE_PROJECT_NAME` in the workflow if your Pages project has a different name.
