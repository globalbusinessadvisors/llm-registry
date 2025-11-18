import axios, { AxiosInstance, AxiosRequestConfig } from 'axios';

/**
 * Configuration options for the LLM Registry client
 */
export interface LLMRegistryConfig {
  /** Base URL of the LLM Registry API */
  baseURL: string;
  /** API token for authentication (optional) */
  apiToken?: string;
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;
  /** Additional axios configuration */
  axiosConfig?: AxiosRequestConfig;
}

/**
 * Model information
 */
export interface Model {
  id: string;
  name: string;
  version: string;
  description?: string;
  provider?: string;
  created_at: string;
  updated_at: string;
  tags?: string[];
  metadata?: Record<string, any>;
}

/**
 * Asset information
 */
export interface Asset {
  id: string;
  model_id: string;
  name: string;
  version: string;
  content_type: string;
  size: number;
  checksum: string;
  storage_path: string;
  created_at: string;
  metadata?: Record<string, any>;
}

/**
 * Model creation request
 */
export interface CreateModelRequest {
  name: string;
  version: string;
  description?: string;
  provider?: string;
  tags?: string[];
  metadata?: Record<string, any>;
}

/**
 * Asset upload request
 */
export interface UploadAssetRequest {
  model_id: string;
  name: string;
  version: string;
  content_type: string;
  file: Buffer | Blob;
  metadata?: Record<string, any>;
}

/**
 * Search filters
 */
export interface SearchFilters {
  query?: string;
  provider?: string;
  tags?: string[];
  limit?: number;
  offset?: number;
}

/**
 * LLM Registry SDK Client
 *
 * A TypeScript client for interacting with the LLM Registry API
 *
 * @example
 * ```typescript
 * const client = new LLMRegistryClient({
 *   baseURL: 'http://localhost:8080',
 *   apiToken: 'your-api-token'
 * });
 *
 * // List models
 * const models = await client.listModels();
 *
 * // Get a specific model
 * const model = await client.getModel('model-id');
 *
 * // Create a new model
 * const newModel = await client.createModel({
 *   name: 'my-model',
 *   version: '1.0.0',
 *   description: 'My custom model'
 * });
 * ```
 */
export class LLMRegistryClient {
  private client: AxiosInstance;

  constructor(config: LLMRegistryConfig) {
    const { baseURL, apiToken, timeout = 30000, axiosConfig = {} } = config;

    this.client = axios.create({
      baseURL,
      timeout,
      headers: {
        'Content-Type': 'application/json',
        ...(apiToken && { Authorization: `Bearer ${apiToken}` }),
        ...axiosConfig.headers,
      },
      ...axiosConfig,
    });
  }

  // Models API

  /**
   * List all models
   */
  async listModels(filters?: SearchFilters): Promise<Model[]> {
    const response = await this.client.get<Model[]>('/api/v1/models', {
      params: filters,
    });
    return response.data;
  }

  /**
   * Get a specific model by ID
   */
  async getModel(modelId: string): Promise<Model> {
    const response = await this.client.get<Model>(`/api/v1/models/${modelId}`);
    return response.data;
  }

  /**
   * Create a new model
   */
  async createModel(request: CreateModelRequest): Promise<Model> {
    const response = await this.client.post<Model>('/api/v1/models', request);
    return response.data;
  }

  /**
   * Update a model
   */
  async updateModel(modelId: string, updates: Partial<CreateModelRequest>): Promise<Model> {
    const response = await this.client.patch<Model>(`/api/v1/models/${modelId}`, updates);
    return response.data;
  }

  /**
   * Delete a model
   */
  async deleteModel(modelId: string): Promise<void> {
    await this.client.delete(`/api/v1/models/${modelId}`);
  }

  /**
   * Search models
   */
  async searchModels(filters: SearchFilters): Promise<Model[]> {
    const response = await this.client.get<Model[]>('/api/v1/models/search', {
      params: filters,
    });
    return response.data;
  }

  // Assets API

  /**
   * List assets for a model
   */
  async listAssets(modelId: string): Promise<Asset[]> {
    const response = await this.client.get<Asset[]>(`/api/v1/models/${modelId}/assets`);
    return response.data;
  }

  /**
   * Get a specific asset
   */
  async getAsset(modelId: string, assetId: string): Promise<Asset> {
    const response = await this.client.get<Asset>(`/api/v1/models/${modelId}/assets/${assetId}`);
    return response.data;
  }

  /**
   * Upload an asset
   */
  async uploadAsset(request: UploadAssetRequest): Promise<Asset> {
    const formData = new FormData();
    formData.append('name', request.name);
    formData.append('version', request.version);
    formData.append('content_type', request.content_type);
    formData.append('file', request.file as any);

    if (request.metadata) {
      formData.append('metadata', JSON.stringify(request.metadata));
    }

    const response = await this.client.post<Asset>(
      `/api/v1/models/${request.model_id}/assets`,
      formData,
      {
        headers: {
          'Content-Type': 'multipart/form-data',
        },
      }
    );
    return response.data;
  }

  /**
   * Download an asset
   */
  async downloadAsset(modelId: string, assetId: string): Promise<ArrayBuffer> {
    const response = await this.client.get<ArrayBuffer>(
      `/api/v1/models/${modelId}/assets/${assetId}/download`,
      {
        responseType: 'arraybuffer',
      }
    );
    return response.data;
  }

  /**
   * Delete an asset
   */
  async deleteAsset(modelId: string, assetId: string): Promise<void> {
    await this.client.delete(`/api/v1/models/${modelId}/assets/${assetId}`);
  }

  // Health & Status

  /**
   * Check API health status
   */
  async health(): Promise<{ status: string; version?: string }> {
    const response = await this.client.get('/health');
    return response.data;
  }

  /**
   * Get API version
   */
  async version(): Promise<{ version: string }> {
    const response = await this.client.get('/version');
    return response.data;
  }
}

export default LLMRegistryClient;
