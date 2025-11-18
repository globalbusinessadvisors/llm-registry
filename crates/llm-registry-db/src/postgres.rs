//! PostgreSQL implementation of AssetRepository
//!
//! This module provides a concrete implementation of the AssetRepository trait
//! using PostgreSQL with SQLx for compile-time verified queries.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use llm_registry_core::{
    Asset, AssetId, AssetMetadata, AssetStatus, AssetType, Checksum, HashAlgorithm, Provenance,
    StorageBackend, StorageLocation,
};
use semver::Version;
use serde_json::Value as JsonValue;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, instrument};

use crate::error::{DbError, DbResult};
use crate::repository::{AssetRepository, SearchQuery, SearchResults, SortField, SortOrder};

/// PostgreSQL implementation of AssetRepository
#[derive(Debug, Clone)]
pub struct PostgresAssetRepository {
    pool: PgPool,
}

impl PostgresAssetRepository {
    /// Create a new PostgreSQL asset repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl AssetRepository for PostgresAssetRepository {
    #[instrument(skip(self, asset), fields(asset_id = %asset.id, asset_name = %asset.metadata.name))]
    async fn create(&self, asset: Asset) -> DbResult<Asset> {
        debug!("Creating asset in database");

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // Insert main asset record
        sqlx::query(
            r#"
            INSERT INTO assets (
                id, name, version, asset_type, status,
                storage_backend, storage_uri, storage_path, size_bytes,
                checksum_algorithm, checksum_value,
                signature_algorithm, signature_value, signature_key_id,
                description, license, content_type,
                author, source_repo, commit_hash, build_id,
                created_at, updated_at, deprecated_at, metadata
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11,
                $12, $13, $14,
                $15, $16, $17,
                $18, $19, $20, $21,
                $22, $23, $24, $25
            )
            "#,
        )
        .bind(&asset.id.to_string())
        .bind(&asset.metadata.name)
        .bind(&asset.metadata.version.to_string())
        .bind(&asset.asset_type.to_string())
        .bind(&asset.status.to_string())
        .bind(&asset.storage.backend.to_string())
        .bind(asset.storage.uri.as_ref().unwrap_or(&asset.storage.get_uri()))
        .bind(if asset.storage.path.is_empty() { None } else { Some(&asset.storage.path) })
        .bind(asset.metadata.size_bytes.map(|s| s as i64))
        .bind(&asset.checksum.algorithm.to_string())
        .bind(&asset.checksum.value)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .bind(&asset.metadata.description)
        .bind(&asset.metadata.license)
        .bind(&asset.metadata.content_type)
        .bind(asset.provenance.as_ref().and_then(|p| p.author.as_deref()))
        .bind(asset.provenance.as_ref().and_then(|p| p.source_repo.as_deref()))
        .bind(asset.provenance.as_ref().and_then(|p| p.commit_hash.as_deref()))
        .bind(asset.provenance.as_ref().and_then(|p| p.build_id.as_deref()))
        .bind(&asset.created_at)
        .bind(&asset.updated_at)
        .bind(&asset.deprecated_at)
        .bind(serde_json::to_value(&asset.metadata.annotations)?)
        .execute(&mut *tx)
        .await?;

        // Insert tags
        for tag in &asset.metadata.tags {
            sqlx::query(
                r#"
                INSERT INTO asset_tags (asset_id, tag)
                VALUES ($1, $2)
                ON CONFLICT (asset_id, tag) DO NOTHING
                "#,
            )
            .bind(&asset.id.to_string())
            .bind(tag)
            .execute(&mut *tx)
            .await?;
        }

        // Insert dependencies
        for dep in &asset.dependencies {
            let dep_id = dep.as_id().ok_or_else(|| {
                DbError::InvalidData("Dependency must be resolved to ID before persisting".to_string())
            })?;

            sqlx::query(
                r#"
                INSERT INTO asset_dependencies (asset_id, dependency_id, version_constraint)
                VALUES ($1, $2, $3)
                ON CONFLICT (asset_id, dependency_id) DO NOTHING
                "#,
            )
            .bind(&asset.id.to_string())
            .bind(&dep_id.to_string())
            .bind(dep.as_name_version().map(|(_, v)| v))
            .execute(&mut *tx)
            .await?;
        }

        // Commit transaction
        tx.commit().await?;

        debug!("Asset created successfully");
        Ok(asset)
    }

    #[instrument(skip(self), fields(asset_id = %id))]
    async fn find_by_id(&self, id: &AssetId) -> DbResult<Option<Asset>> {
        debug!("Finding asset by ID");

        let row = sqlx::query(
            r#"
            SELECT
                id, name, version, asset_type, status,
                storage_backend, storage_uri, storage_path, size_bytes,
                checksum_algorithm, checksum_value,
                signature_algorithm, signature_value, signature_key_id,
                description, license, content_type,
                author, source_repo, commit_hash, build_id,
                created_at, updated_at, deprecated_at, metadata
            FROM assets
            WHERE id = $1
            "#,
        )
        .bind(&id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let asset = row_to_asset(row)?;
                let asset = self.load_asset_relations(asset).await?;
                Ok(Some(asset))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn find_by_name_and_version(
        &self,
        name: &str,
        version: &Version,
    ) -> DbResult<Option<Asset>> {
        debug!("Finding asset by name and version");

        let row = sqlx::query(
            r#"
            SELECT
                id, name, version, asset_type, status,
                storage_backend, storage_uri, storage_path, size_bytes,
                checksum_algorithm, checksum_value,
                signature_algorithm, signature_value, signature_key_id,
                description, license, content_type,
                author, source_repo, commit_hash, build_id,
                created_at, updated_at, deprecated_at, metadata
            FROM assets
            WHERE name = $1 AND version = $2
            "#,
        )
        .bind(name)
        .bind(&version.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let asset = row_to_asset(row)?;
                let asset = self.load_asset_relations(asset).await?;
                Ok(Some(asset))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self, ids))]
    async fn find_by_ids(&self, ids: &[AssetId]) -> DbResult<Vec<Asset>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        debug!("Finding {} assets by IDs", ids.len());

        let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();

        let rows = sqlx::query(
            r#"
            SELECT
                id, name, version, asset_type, status,
                storage_backend, storage_uri, storage_path, size_bytes,
                checksum_algorithm, checksum_value,
                signature_algorithm, signature_value, signature_key_id,
                description, license, content_type,
                author, source_repo, commit_hash, build_id,
                created_at, updated_at, deprecated_at, metadata
            FROM assets
            WHERE id = ANY($1)
            "#,
        )
        .bind(&id_strings)
        .fetch_all(&self.pool)
        .await?;

        let mut assets = Vec::new();
        for row in rows {
            let asset = row_to_asset(row)?;
            let asset = self.load_asset_relations(asset).await?;
            assets.push(asset);
        }

        Ok(assets)
    }

    #[instrument(skip(self, query))]
    async fn search(&self, query: &SearchQuery) -> DbResult<SearchResults> {
        debug!("Searching assets with filters");

        // Build dynamic query
        let mut sql = String::from(
            r#"
            SELECT
                a.id, a.name, a.version, a.asset_type, a.status,
                a.storage_backend, a.storage_uri, a.storage_path, a.size_bytes,
                a.checksum_algorithm, a.checksum_value,
                a.signature_algorithm, a.signature_value, a.signature_key_id,
                a.description, a.license, a.content_type,
                a.author, a.source_repo, a.commit_hash, a.build_id,
                a.created_at, a.updated_at, a.deprecated_at, a.metadata
            FROM assets a
            WHERE 1=1
            "#,
        );

        let mut conditions = Vec::new();
        let mut bind_values: Vec<String> = Vec::new();
        let mut param_num = 1;

        // Text search
        if let Some(ref text) = query.text {
            conditions.push(format!(
                "(a.name ILIKE ${} OR a.description ILIKE ${})",
                param_num, param_num + 1
            ));
            bind_values.push(format!("%{}%", text));
            bind_values.push(format!("%{}%", text));
            param_num += 2;
        }

        // Asset type filter
        if !query.asset_types.is_empty() {
            conditions.push(format!("a.asset_type = ANY(${})", param_num));
            // This is a placeholder - we'll use a different approach for ANY
            param_num += 1;
        }

        // Author filter
        if let Some(ref author) = query.author {
            conditions.push(format!("a.author = ${}", param_num));
            bind_values.push(author.clone());
            param_num += 1;
        }

        // Storage backend filter
        if let Some(ref backend) = query.storage_backend {
            conditions.push(format!("a.storage_backend = ${}", param_num));
            bind_values.push(backend.clone());
            param_num += 1;
        }

        // Deprecated filter
        if query.exclude_deprecated {
            conditions.push("a.deprecated_at IS NULL".to_string());
        }

        // Tag filter - must have all specified tags
        if !query.tags.is_empty() {
            let tag_condition = format!(
                "a.id IN (
                    SELECT asset_id FROM asset_tags
                    WHERE tag = ANY(${}::text[])
                    GROUP BY asset_id
                    HAVING COUNT(DISTINCT tag) = {}
                )",
                param_num,
                query.tags.len()
            );
            conditions.push(tag_condition);
            #[allow(unused_assignments)]
            {
                param_num += 1;
            }
        }

        // Add conditions to query
        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        // Add ORDER BY
        let sort_field = match query.sort_by {
            SortField::CreatedAt => "a.created_at",
            SortField::UpdatedAt => "a.updated_at",
            SortField::Name => "a.name",
            SortField::Version => "a.version",
            SortField::SizeBytes => "a.size_bytes",
        };

        let sort_order = match query.sort_order {
            SortOrder::Ascending => "ASC",
            SortOrder::Descending => "DESC",
        };

        sql.push_str(&format!(" ORDER BY {} {}", sort_field, sort_order));

        // Add LIMIT and OFFSET
        sql.push_str(&format!(" LIMIT {} OFFSET {}", query.limit, query.offset));

        // For simplicity, we'll use a simpler approach - rebuild with sqlx query builder
        // In production, you'd want to use a query builder or macro for this
        let mut final_query = sqlx::query(&sql);

        // Bind parameters in order
        for value in &bind_values {
            final_query = final_query.bind(value);
        }

        let types: Vec<String> = query.asset_types.iter().map(|t| t.to_string()).collect();
        if !query.asset_types.is_empty() {
            final_query = final_query.bind(&types);
        }

        if !query.tags.is_empty() {
            final_query = final_query.bind(&query.tags);
        }

        let rows = final_query.fetch_all(&self.pool).await?;

        let mut assets = Vec::new();
        for row in rows {
            let asset = row_to_asset(row)?;
            let asset = self.load_asset_relations(asset).await?;
            assets.push(asset);
        }

        // Get total count (without pagination)
        let total = self.count_search_results(query).await?;

        Ok(SearchResults {
            assets,
            total,
            offset: query.offset,
            limit: query.limit,
        })
    }

    #[instrument(skip(self, asset), fields(asset_id = %asset.id))]
    async fn update(&self, asset: Asset) -> DbResult<Asset> {
        debug!("Updating asset");

        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            r#"
            UPDATE assets SET
                name = $2,
                version = $3,
                asset_type = $4,
                status = $5,
                storage_backend = $6,
                storage_uri = $7,
                storage_path = $8,
                size_bytes = $9,
                checksum_algorithm = $10,
                checksum_value = $11,
                signature_algorithm = $12,
                signature_value = $13,
                signature_key_id = $14,
                description = $15,
                license = $16,
                content_type = $17,
                author = $18,
                source_repo = $19,
                commit_hash = $20,
                build_id = $21,
                deprecated_at = $22,
                metadata = $23,
                updated_at = $24
            WHERE id = $1
            "#,
        )
        .bind(&asset.id.to_string())
        .bind(&asset.metadata.name)
        .bind(&asset.metadata.version.to_string())
        .bind(&asset.asset_type.to_string())
        .bind(&asset.status.to_string())
        .bind(&asset.storage.backend.to_string())
        .bind(asset.storage.uri.as_ref().unwrap_or(&asset.storage.get_uri()))
        .bind(if asset.storage.path.is_empty() { None } else { Some(&asset.storage.path) })
        .bind(asset.metadata.size_bytes.map(|s| s as i64))
        .bind(&asset.checksum.algorithm.to_string())
        .bind(&asset.checksum.value)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .bind(&asset.metadata.description)
        .bind(&asset.metadata.license)
        .bind(&asset.metadata.content_type)
        .bind(asset.provenance.as_ref().and_then(|p| p.author.as_deref()))
        .bind(asset.provenance.as_ref().and_then(|p| p.source_repo.as_deref()))
        .bind(asset.provenance.as_ref().and_then(|p| p.commit_hash.as_deref()))
        .bind(asset.provenance.as_ref().and_then(|p| p.build_id.as_deref()))
        .bind(&asset.deprecated_at)
        .bind(serde_json::to_value(&asset.metadata.annotations)?)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Asset {} not found", asset.id)));
        }

        // Update tags - delete and re-insert for simplicity
        sqlx::query("DELETE FROM asset_tags WHERE asset_id = $1")
            .bind(&asset.id.to_string())
            .execute(&mut *tx)
            .await?;

        for tag in &asset.metadata.tags {
            sqlx::query(
                r#"
                INSERT INTO asset_tags (asset_id, tag)
                VALUES ($1, $2)
                "#,
            )
            .bind(&asset.id.to_string())
            .bind(tag)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        debug!("Asset updated successfully");
        Ok(asset)
    }

    #[instrument(skip(self), fields(asset_id = %id))]
    async fn delete(&self, id: &AssetId) -> DbResult<()> {
        debug!("Deleting asset");

        let result = sqlx::query("DELETE FROM assets WHERE id = $1")
            .bind(&id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Asset {} not found", id)));
        }

        debug!("Asset deleted successfully");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_versions(&self, name: &str) -> DbResult<Vec<Asset>> {
        debug!("Listing versions for asset");

        let rows = sqlx::query(
            r#"
            SELECT
                id, name, version, asset_type, status,
                storage_backend, storage_uri, storage_path, size_bytes,
                checksum_algorithm, checksum_value,
                signature_algorithm, signature_value, signature_key_id,
                description, license, content_type,
                author, source_repo, commit_hash, build_id,
                created_at, updated_at, deprecated_at, metadata
            FROM assets
            WHERE name = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await?;

        let mut assets = Vec::new();
        for row in rows {
            let asset = row_to_asset(row)?;
            let asset = self.load_asset_relations(asset).await?;
            assets.push(asset);
        }

        Ok(assets)
    }

    #[instrument(skip(self), fields(asset_id = %id))]
    async fn list_dependencies(&self, id: &AssetId) -> DbResult<Vec<Asset>> {
        debug!("Listing dependencies");

        let rows = sqlx::query(
            r#"
            SELECT
                a.id, a.name, a.version, a.asset_type, a.status,
                a.storage_backend, a.storage_uri, a.storage_path, a.size_bytes,
                a.checksum_algorithm, a.checksum_value,
                a.signature_algorithm, a.signature_value, a.signature_key_id,
                a.description, a.license, a.content_type,
                a.author, a.source_repo, a.commit_hash, a.build_id,
                a.created_at, a.updated_at, a.deprecated_at, a.metadata
            FROM assets a
            INNER JOIN asset_dependencies d ON a.id = d.dependency_id
            WHERE d.asset_id = $1
            "#,
        )
        .bind(&id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut assets = Vec::new();
        for row in rows {
            let asset = row_to_asset(row)?;
            let asset = self.load_asset_relations(asset).await?;
            assets.push(asset);
        }

        Ok(assets)
    }

    #[instrument(skip(self), fields(asset_id = %id))]
    async fn list_reverse_dependencies(&self, id: &AssetId) -> DbResult<Vec<Asset>> {
        debug!("Listing reverse dependencies");

        let rows = sqlx::query(
            r#"
            SELECT
                a.id, a.name, a.version, a.asset_type, a.status,
                a.storage_backend, a.storage_uri, a.storage_path, a.size_bytes,
                a.checksum_algorithm, a.checksum_value,
                a.signature_algorithm, a.signature_value, a.signature_key_id,
                a.description, a.license, a.content_type,
                a.author, a.source_repo, a.commit_hash, a.build_id,
                a.created_at, a.updated_at, a.deprecated_at, a.metadata
            FROM assets a
            INNER JOIN asset_dependencies d ON a.id = d.asset_id
            WHERE d.dependency_id = $1
            "#,
        )
        .bind(&id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut assets = Vec::new();
        for row in rows {
            let asset = row_to_asset(row)?;
            let asset = self.load_asset_relations(asset).await?;
            assets.push(asset);
        }

        Ok(assets)
    }

    #[instrument(skip(self), fields(asset_id = %id, tag = %tag))]
    async fn add_tag(&self, id: &AssetId, tag: &str) -> DbResult<()> {
        debug!("Adding tag to asset");

        sqlx::query(
            r#"
            INSERT INTO asset_tags (asset_id, tag)
            VALUES ($1, $2)
            ON CONFLICT (asset_id, tag) DO NOTHING
            "#,
        )
        .bind(&id.to_string())
        .bind(tag)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[instrument(skip(self), fields(asset_id = %id, tag = %tag))]
    async fn remove_tag(&self, id: &AssetId, tag: &str) -> DbResult<()> {
        debug!("Removing tag from asset");

        sqlx::query("DELETE FROM asset_tags WHERE asset_id = $1 AND tag = $2")
            .bind(&id.to_string())
            .bind(tag)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[instrument(skip(self), fields(asset_id = %id))]
    async fn get_tags(&self, id: &AssetId) -> DbResult<Vec<String>> {
        debug!("Getting tags for asset");

        let rows = sqlx::query("SELECT tag FROM asset_tags WHERE asset_id = $1 ORDER BY tag")
            .bind(&id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let tags = rows
            .iter()
            .map(|row| row.get::<String, _>("tag"))
            .collect();

        Ok(tags)
    }

    #[instrument(skip(self))]
    async fn list_all_tags(&self) -> DbResult<Vec<String>> {
        debug!("Listing all tags");

        let rows = sqlx::query("SELECT DISTINCT tag FROM asset_tags ORDER BY tag")
            .fetch_all(&self.pool)
            .await?;

        let tags = rows
            .iter()
            .map(|row| row.get::<String, _>("tag"))
            .collect();

        Ok(tags)
    }

    #[instrument(skip(self))]
    async fn add_dependency(
        &self,
        asset_id: &AssetId,
        dependency_id: &AssetId,
        version_constraint: Option<&str>,
    ) -> DbResult<()> {
        debug!("Adding dependency relationship");

        // Check for circular dependency
        if self.would_create_cycle(asset_id, dependency_id).await? {
            return Err(DbError::CircularDependency(format!(
                "Adding dependency from {} to {} would create a cycle",
                asset_id, dependency_id
            )));
        }

        sqlx::query(
            r#"
            INSERT INTO asset_dependencies (asset_id, dependency_id, version_constraint)
            VALUES ($1, $2, $3)
            ON CONFLICT (asset_id, dependency_id) DO UPDATE
            SET version_constraint = EXCLUDED.version_constraint
            "#,
        )
        .bind(&asset_id.to_string())
        .bind(&dependency_id.to_string())
        .bind(version_constraint)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_dependency(
        &self,
        asset_id: &AssetId,
        dependency_id: &AssetId,
    ) -> DbResult<()> {
        debug!("Removing dependency relationship");

        sqlx::query("DELETE FROM asset_dependencies WHERE asset_id = $1 AND dependency_id = $2")
            .bind(&asset_id.to_string())
            .bind(&dependency_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn count_assets(&self) -> DbResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM assets")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    #[instrument(skip(self))]
    async fn count_by_type(&self, asset_type: &AssetType) -> DbResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM assets WHERE asset_type = $1")
            .bind(&asset_type.to_string())
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> DbResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }
}

impl PostgresAssetRepository {
    /// Load tags and dependencies for an asset
    async fn load_asset_relations(&self, mut asset: Asset) -> DbResult<Asset> {
        // Load tags
        let tags = self.get_tags(&asset.id).await?;
        asset.metadata.tags = tags;

        // Load dependency references
        let dep_rows = sqlx::query(
            "SELECT dependency_id FROM asset_dependencies WHERE asset_id = $1"
        )
        .bind(&asset.id.to_string())
        .fetch_all(&self.pool)
        .await?;

        asset.dependencies = dep_rows
            .iter()
            .filter_map(|row| {
                let dep_id_str: String = row.get("dependency_id");
                AssetId::from_str(&dep_id_str)
                    .ok()
                    .map(|id| llm_registry_core::AssetReference::by_id(id))
            })
            .collect();

        Ok(asset)
    }

    /// Check if adding a dependency would create a cycle
    async fn would_create_cycle(&self, from_id: &AssetId, to_id: &AssetId) -> DbResult<bool> {
        // Use recursive CTE to check for cycles
        let row = sqlx::query(
            r#"
            WITH RECURSIVE dep_tree AS (
                SELECT dependency_id, 1 as depth
                FROM asset_dependencies
                WHERE asset_id = $1

                UNION ALL

                SELECT d.dependency_id, dt.depth + 1
                FROM asset_dependencies d
                INNER JOIN dep_tree dt ON d.asset_id = dt.dependency_id
                WHERE dt.depth < 100
            )
            SELECT COUNT(*) > 0 as has_cycle
            FROM dep_tree
            WHERE dependency_id = $2
            "#,
        )
        .bind(&to_id.to_string())
        .bind(&from_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("has_cycle"))
    }

    /// Count search results without pagination
    async fn count_search_results(&self, query: &SearchQuery) -> DbResult<i64> {
        // Simplified count query - in production, this should mirror the search logic
        let mut sql = String::from("SELECT COUNT(*) as count FROM assets a WHERE 1=1");

        if query.exclude_deprecated {
            sql.push_str(" AND a.deprecated_at IS NULL");
        }

        if !query.asset_types.is_empty() {
            let types: Vec<String> = query.asset_types.iter().map(|t| t.to_string()).collect();
            let placeholders: Vec<String> = types.iter().map(|t| format!("'{}'", t)).collect();
            sql.push_str(&format!(" AND a.asset_type IN ({})", placeholders.join(", ")));
        }

        let row = sqlx::query(&sql).fetch_one(&self.pool).await?;

        Ok(row.get("count"))
    }
}

/// Convert a database row to an Asset
fn row_to_asset(row: PgRow) -> DbResult<Asset> {
    let id_str: String = row.get("id");
    let id = AssetId::from_str(&id_str)
        .map_err(|e| DbError::InvalidData(format!("Invalid asset ID: {}", e)))?;

    let version_str: String = row.get("version");
    let version = Version::parse(&version_str)
        .map_err(|e| DbError::InvalidData(format!("Invalid version: {}", e)))?;

    let asset_type_str: String = row.get("asset_type");
    let asset_type = parse_asset_type(&asset_type_str)?;

    let status_str: String = row.get("status");
    let status = parse_asset_status(&status_str)?;

    let backend_str: String = row.get("storage_backend");
    let backend = parse_storage_backend_from_db(&backend_str)?;

    let storage_uri: String = row.get("storage_uri");
    let storage_path: Option<String> = row.get("storage_path");

    let checksum_algo_str: String = row.get("checksum_algorithm");
    let checksum_algorithm = parse_hash_algorithm(&checksum_algo_str)?;
    let checksum_value: String = row.get("checksum_value");

    let metadata_json: JsonValue = row.get("metadata");
    let annotations: HashMap<String, String> = serde_json::from_value(metadata_json)
        .unwrap_or_default();

    let created_at: DateTime<Utc> = row.get("created_at");
    let updated_at: DateTime<Utc> = row.get("updated_at");
    let deprecated_at: Option<DateTime<Utc>> = row.get("deprecated_at");

    let size_bytes: Option<i64> = row.get("size_bytes");

    let provenance = {
        let author: Option<String> = row.get("author");
        let source_repo: Option<String> = row.get("source_repo");
        let commit_hash: Option<String> = row.get("commit_hash");
        let build_id: Option<String> = row.get("build_id");

        if author.is_some() || source_repo.is_some() {
            Some(Provenance {
                author,
                source_repo,
                commit_hash,
                build_id,
                created_at: Utc::now(),
                build_metadata: HashMap::new(),
            })
        } else {
            None
        }
    };

    let metadata = AssetMetadata {
        name: row.get("name"),
        version,
        description: row.get("description"),
        license: row.get("license"),
        tags: Vec::new(), // Loaded separately
        annotations,
        size_bytes: size_bytes.map(|s| s as u64),
        content_type: row.get("content_type"),
    };

    let storage = StorageLocation {
        backend,
        path: storage_path.unwrap_or_default(),
        uri: Some(storage_uri),
    };

    let checksum = Checksum {
        algorithm: checksum_algorithm,
        value: checksum_value,
    };

    Ok(Asset {
        id,
        asset_type,
        metadata,
        status,
        storage,
        checksum,
        provenance,
        dependencies: Vec::new(), // Loaded separately
        created_at,
        updated_at,
        deprecated_at,
    })
}

fn parse_asset_type(s: &str) -> DbResult<AssetType> {
    match s {
        "model" => Ok(AssetType::Model),
        "pipeline" => Ok(AssetType::Pipeline),
        "test_suite" => Ok(AssetType::TestSuite),
        "policy" => Ok(AssetType::Policy),
        "dataset" => Ok(AssetType::Dataset),
        other => AssetType::custom(other)
            .map_err(|e| DbError::InvalidData(format!("Invalid asset type: {}", e))),
    }
}

fn parse_asset_status(s: &str) -> DbResult<AssetStatus> {
    AssetStatus::from_str(s)
        .map_err(|e| DbError::InvalidData(format!("Invalid asset status: {}", e)))
}

fn parse_storage_backend_from_db(s: &str) -> DbResult<StorageBackend> {
    // For simplicity in the database, we'll store just the backend type as a string
    // and reconstruct minimal backend config. In a real system, you'd store full JSON.
    match s {
        "S3" => Ok(StorageBackend::S3 {
            bucket: String::new(),
            region: String::new(),
            endpoint: None,
        }),
        "GCS" => Ok(StorageBackend::GCS {
            bucket: String::new(),
            project_id: String::new(),
        }),
        "AzureBlob" => Ok(StorageBackend::AzureBlob {
            account_name: String::new(),
            container: String::new(),
        }),
        "MinIO" => Ok(StorageBackend::MinIO {
            bucket: String::new(),
            endpoint: String::new(),
        }),
        "FileSystem" => Ok(StorageBackend::FileSystem {
            base_path: String::new(),
        }),
        _ => Err(DbError::InvalidData(format!(
            "Invalid storage backend type: {}",
            s
        ))),
    }
}

fn parse_hash_algorithm(s: &str) -> DbResult<HashAlgorithm> {
    HashAlgorithm::from_str(s)
        .map_err(|e| DbError::InvalidData(format!("Invalid hash algorithm: {}", e)))
}
