# n8n-workflow-sync

`n8n-workflow-sync` is a command line tool to list and create workflows on an n8n instance. It aims to let you version control workflows locally and sync them via the n8n REST API.

## Usage

Set the following environment variables so the CLI can authenticate with your n8n server:

```bash
export N8N_HOST=https://your.n8n.instance/
export N8N_API_KEY=your-api-key
```

Then run the binary with one of the available subcommands:

```bash
# List workflows
cargo run -- list

# Create a new workflow
cargo run -- new "My Flow"

# Download an existing workflow
cargo run -- pull 123 workflow.json

# Upload changes back to n8n
cargo run -- push 123 workflow.json
```

## Development

This project is written in Rust and uses `cargo` for building and testing:

```bash
cargo build
cargo test
```

## License

This project is licensed under the MIT License. See [LICENSE-MIT](LICENSE-MIT) for details.
