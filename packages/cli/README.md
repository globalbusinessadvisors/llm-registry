# @llm-dev-ops/llm-registry

Command-line interface for the LLM Registry - A secure, production-ready registry for Large Language Models.

## Installation

```bash
npm install -g @llm-dev-ops/llm-registry
```

## Configuration

Set up your LLM Registry connection:

```bash
# Using the config command
llm-registry config --url http://localhost:8080 --token your-api-token

# Or use environment variables
export LLM_REGISTRY_URL=http://localhost:8080
export LLM_REGISTRY_TOKEN=your-api-token
```

Configuration is saved to `~/.llm-registry.json`.

## Usage

### Health Check

```bash
llm-registry health
```

### Model Management

#### List Models

```bash
# List all models
llm-registry models list

# Filter by provider
llm-registry models list --provider openai

# Filter by tags
llm-registry models list --tags gpt,chat

# Limit results
llm-registry models list --limit 20
```

#### Get Model Details

```bash
llm-registry models get <model-id>
```

#### Create a Model

```bash
llm-registry models create \
  --name my-model \
  --version 1.0.0 \
  --description "My custom model" \
  --provider openai \
  --tags gpt,chat
```

#### Delete a Model

```bash
llm-registry models delete <model-id>
```

### Asset Management

#### List Assets

```bash
llm-registry assets list <model-id>
```

#### Upload an Asset

```bash
llm-registry assets upload <model-id> ./model.safetensors \
  --name weights \
  --version 1.0.0 \
  --content-type application/octet-stream
```

#### Download an Asset

```bash
llm-registry assets download <model-id> <asset-id> ./output.safetensors
```

## Commands

### `llm-registry config`

Configure the CLI connection settings.

**Options:**
- `-u, --url <url>` - Set the base URL
- `-t, --token <token>` - Set the API token

### `llm-registry models list`

List all models with optional filtering.

**Options:**
- `-p, --provider <provider>` - Filter by provider
- `-t, --tags <tags>` - Filter by tags (comma-separated)
- `-l, --limit <limit>` - Limit results (default: 10)

### `llm-registry models get <id>`

Get detailed information about a specific model.

### `llm-registry models create`

Create a new model.

**Options:**
- `-n, --name <name>` - Model name (required)
- `-v, --version <version>` - Model version (required)
- `-d, --description <description>` - Model description
- `-p, --provider <provider>` - Model provider
- `-t, --tags <tags>` - Tags (comma-separated)

### `llm-registry models delete <id>`

Delete a model.

### `llm-registry assets list <model-id>`

List all assets for a model.

### `llm-registry assets upload <model-id> <file>`

Upload an asset to a model.

**Options:**
- `-n, --name <name>` - Asset name (required)
- `-v, --version <version>` - Asset version (required)
- `-t, --content-type <type>` - Content type (default: application/octet-stream)

### `llm-registry assets download <model-id> <asset-id> <output>`

Download an asset from a model.

### `llm-registry health`

Check the health status of the LLM Registry API.

## Environment Variables

- `LLM_REGISTRY_URL` - Base URL of the LLM Registry API
- `LLM_REGISTRY_TOKEN` - API authentication token

## Examples

```bash
# Configure the CLI
llm-registry config --url https://registry.example.com

# Create a new model
llm-registry models create \
  --name llama-2-7b \
  --version 1.0.0 \
  --provider meta \
  --tags llama,chat,7b

# Upload model weights
llm-registry assets upload abc123 ./llama-2-7b.safetensors \
  --name weights \
  --version 1.0.0

# List all chat models
llm-registry models list --tags chat

# Download a model
llm-registry assets download abc123 def456 ./downloaded-model.safetensors

# Check API health
llm-registry health
```

## License

Apache-2.0 OR MIT

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/globalbusinessadvisors/llm-registry) for guidelines.
