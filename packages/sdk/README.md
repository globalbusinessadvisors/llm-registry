# @llm-dev-ops/llm-registry-sdk

TypeScript SDK for the LLM Registry - A secure, production-ready registry for Large Language Models.

## Installation

```bash
npm install @llm-dev-ops/llm-registry-sdk
```

## Usage

### Initialize the Client

```typescript
import { LLMRegistryClient } from '@llm-dev-ops/llm-registry-sdk';

const client = new LLMRegistryClient({
  baseURL: 'http://localhost:8080',
  apiToken: 'your-api-token', // Optional
  timeout: 30000 // Optional, default: 30000ms
});
```

### Working with Models

```typescript
// List all models
const models = await client.listModels();

// Get a specific model
const model = await client.getModel('model-id');

// Create a new model
const newModel = await client.createModel({
  name: 'my-model',
  version: '1.0.0',
  description: 'My custom model',
  provider: 'openai',
  tags: ['gpt', 'chat'],
  metadata: {
    parameters: 1000000000
  }
});

// Update a model
const updated = await client.updateModel('model-id', {
  description: 'Updated description'
});

// Delete a model
await client.deleteModel('model-id');

// Search models
const results = await client.searchModels({
  query: 'gpt',
  provider: 'openai',
  tags: ['chat'],
  limit: 10,
  offset: 0
});
```

### Working with Assets

```typescript
// List assets for a model
const assets = await client.listAssets('model-id');

// Get a specific asset
const asset = await client.getAsset('model-id', 'asset-id');

// Upload an asset
const file = fs.readFileSync('./model.safetensors');
const newAsset = await client.uploadAsset({
  model_id: 'model-id',
  name: 'weights',
  version: '1.0.0',
  content_type: 'application/octet-stream',
  file: file,
  metadata: {
    format: 'safetensors'
  }
});

// Download an asset
const data = await client.downloadAsset('model-id', 'asset-id');
fs.writeFileSync('./downloaded-model.safetensors', Buffer.from(data));

// Delete an asset
await client.deleteAsset('model-id', 'asset-id');
```

### Health Checks

```typescript
// Check API health
const health = await client.health();
console.log(health.status); // "ok"

// Get API version
const version = await client.version();
console.log(version.version); // "0.1.0"
```

## API Reference

### `LLMRegistryClient`

The main client class for interacting with the LLM Registry API.

#### Constructor

```typescript
new LLMRegistryClient(config: LLMRegistryConfig)
```

#### Configuration Options

- `baseURL` (string, required): Base URL of the LLM Registry API
- `apiToken` (string, optional): API token for authentication
- `timeout` (number, optional): Request timeout in milliseconds (default: 30000)
- `axiosConfig` (AxiosRequestConfig, optional): Additional axios configuration

#### Methods

##### Models

- `listModels(filters?: SearchFilters): Promise<Model[]>`
- `getModel(modelId: string): Promise<Model>`
- `createModel(request: CreateModelRequest): Promise<Model>`
- `updateModel(modelId: string, updates: Partial<CreateModelRequest>): Promise<Model>`
- `deleteModel(modelId: string): Promise<void>`
- `searchModels(filters: SearchFilters): Promise<Model[]>`

##### Assets

- `listAssets(modelId: string): Promise<Asset[]>`
- `getAsset(modelId: string, assetId: string): Promise<Asset>`
- `uploadAsset(request: UploadAssetRequest): Promise<Asset>`
- `downloadAsset(modelId: string, assetId: string): Promise<ArrayBuffer>`
- `deleteAsset(modelId: string, assetId: string): Promise<void>`

##### Health & Status

- `health(): Promise<{ status: string; version?: string }>`
- `version(): Promise<{ version: string }>`

## TypeScript Support

This package is written in TypeScript and includes full type definitions out of the box.

## License

Apache-2.0 OR MIT

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/globalbusinessadvisors/llm-registry) for guidelines.
