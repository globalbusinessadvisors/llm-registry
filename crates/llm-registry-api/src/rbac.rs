//! Role-Based Access Control (RBAC)
//!
//! This module provides a comprehensive RBAC system with roles, permissions,
//! and policy-based access control.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Permission representing a specific action on a resource
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// Resource type (e.g., "asset", "user", "api-key")
    pub resource: String,

    /// Action (e.g., "read", "write", "delete", "admin")
    pub action: String,
}

impl Permission {
    /// Create a new permission
    pub fn new(resource: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            action: action.into(),
        }
    }

    /// Check if this permission matches another (supports wildcards)
    pub fn matches(&self, other: &Permission) -> bool {
        let resource_match = self.resource == "*" || self.resource == other.resource;
        let action_match = self.action == "*" || self.action == other.action;
        resource_match && action_match
    }

    /// Create from string format "resource:action"
    pub fn from_string(s: &str) -> Result<Self, RbacError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(RbacError::InvalidPermissionFormat(s.to_string()));
        }
        Ok(Permission::new(parts[0], parts[1]))
    }

    /// Convert to string format "resource:action"
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.resource, self.action)
    }
}

/// Role with associated permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,

    /// Role description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Permissions granted to this role
    pub permissions: HashSet<Permission>,

    /// Parent roles (for role hierarchy)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inherits_from: Vec<String>,
}

impl Role {
    /// Create a new role
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            permissions: HashSet::new(),
            inherits_from: Vec::new(),
        }
    }

    /// Add a permission to this role
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }

    /// Add multiple permissions
    pub fn add_permissions(&mut self, permissions: Vec<Permission>) {
        self.permissions.extend(permissions);
    }

    /// Add parent role for inheritance
    pub fn add_parent(&mut self, parent_role: impl Into<String>) {
        self.inherits_from.push(parent_role.into());
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Check if role has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.iter().any(|p| p.matches(permission))
    }
}

/// RBAC policy manager
#[derive(Debug, Clone)]
pub struct RbacPolicy {
    /// Map of role name to role definition
    roles: HashMap<String, Role>,

    /// Cached permission sets for roles (including inherited)
    permission_cache: HashMap<String, HashSet<Permission>>,
}

impl Default for RbacPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RbacPolicy {
    /// Create a new RBAC policy
    pub fn new() -> Self {
        let mut policy = Self {
            roles: HashMap::new(),
            permission_cache: HashMap::new(),
        };

        // Add default roles
        policy.add_default_roles();
        policy
    }

    /// Add default system roles
    fn add_default_roles(&mut self) {
        // Super admin role with all permissions
        let mut admin = Role::new("admin");
        admin.description = Some("System administrator with full access".to_string());
        admin.add_permission(Permission::new("*", "*"));
        self.add_role(admin);

        // Developer role
        let mut developer = Role::new("developer");
        developer.description = Some("Developer with asset management and API access".to_string());
        developer.add_permissions(vec![
            Permission::new("asset", "read"),
            Permission::new("asset", "write"),
            Permission::new("asset", "delete"),
            Permission::new("api-key", "create"),
            Permission::new("api-key", "read"),
        ]);
        self.add_role(developer);

        // Viewer role
        let mut viewer = Role::new("viewer");
        viewer.description = Some("Read-only access to assets".to_string());
        viewer.add_permissions(vec![
            Permission::new("asset", "read"),
            Permission::new("dependency", "read"),
        ]);
        self.add_role(viewer);

        // User role
        let mut user = Role::new("user");
        user.description = Some("Regular user with basic permissions".to_string());
        user.add_permissions(vec![
            Permission::new("asset", "read"),
            Permission::new("asset", "write"),
        ]);
        self.add_role(user);
    }

    /// Add a role to the policy
    pub fn add_role(&mut self, role: Role) {
        let role_name = role.name.clone();
        self.roles.insert(role_name.clone(), role);
        self.invalidate_cache(&role_name);
    }

    /// Get a role by name
    pub fn get_role(&self, name: &str) -> Option<&Role> {
        self.roles.get(name)
    }

    /// Get all roles
    pub fn list_roles(&self) -> Vec<&Role> {
        self.roles.values().collect()
    }

    /// Remove a role
    pub fn remove_role(&mut self, name: &str) -> Option<Role> {
        self.invalidate_cache(name);
        self.roles.remove(name)
    }

    /// Invalidate permission cache for a role
    fn invalidate_cache(&mut self, role_name: &str) {
        self.permission_cache.remove(role_name);
        // Also invalidate cache for roles that inherit from this role
        let dependent_roles: Vec<String> = self
            .roles
            .iter()
            .filter(|(_, role)| role.inherits_from.contains(&role_name.to_string()))
            .map(|(name, _)| name.clone())
            .collect();

        for role in dependent_roles {
            self.permission_cache.remove(&role);
        }
    }

    /// Get all permissions for a role (including inherited)
    pub fn get_role_permissions(&mut self, role_name: &str) -> Option<HashSet<Permission>> {
        // Check cache first
        if let Some(cached) = self.permission_cache.get(role_name) {
            return Some(cached.clone());
        }

        // Get role and collect parent role names
        let (direct_permissions, parent_roles) = {
            let role = self.roles.get(role_name)?;
            (role.permissions.clone(), role.inherits_from.clone())
        };

        // Start with direct permissions
        let mut permissions = direct_permissions;

        // Add inherited permissions
        for parent_role_name in &parent_roles {
            if let Some(parent_permissions) = self.get_role_permissions(parent_role_name) {
                permissions.extend(parent_permissions);
            }
        }

        // Cache the result
        self.permission_cache
            .insert(role_name.to_string(), permissions.clone());

        Some(permissions)
    }

    /// Check if a set of roles has a specific permission
    pub fn has_permission(&mut self, roles: &[String], permission: &Permission) -> bool {
        for role_name in roles {
            if let Some(role_permissions) = self.get_role_permissions(role_name) {
                if role_permissions.iter().any(|p| p.matches(permission)) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a set of roles has ANY of the specified permissions
    pub fn has_any_permission(
        &mut self,
        roles: &[String],
        permissions: &[Permission],
    ) -> bool {
        permissions
            .iter()
            .any(|p| self.has_permission(roles, p))
    }

    /// Check if a set of roles has ALL of the specified permissions
    pub fn has_all_permissions(
        &mut self,
        roles: &[String],
        permissions: &[Permission],
    ) -> bool {
        permissions
            .iter()
            .all(|p| self.has_permission(roles, p))
    }
}

/// RBAC errors
#[derive(Debug, thiserror::Error)]
pub enum RbacError {
    #[error("Invalid permission format: {0}. Expected format: resource:action")]
    InvalidPermissionFormat(String),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Circular role inheritance detected")]
    CircularInheritance,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let perm = Permission::new("asset", "read");
        assert_eq!(perm.resource, "asset");
        assert_eq!(perm.action, "read");
        assert_eq!(perm.to_string(), "asset:read");
    }

    #[test]
    fn test_permission_from_string() {
        let perm = Permission::from_string("asset:write").unwrap();
        assert_eq!(perm.resource, "asset");
        assert_eq!(perm.action, "write");

        assert!(Permission::from_string("invalid").is_err());
    }

    #[test]
    fn test_permission_wildcard_matching() {
        let perm1 = Permission::new("*", "*");
        let perm2 = Permission::new("asset", "read");

        assert!(perm1.matches(&perm2));
        assert!(!perm2.matches(&perm1));
    }

    #[test]
    fn test_role_creation() {
        let mut role = Role::new("developer");
        role.add_permission(Permission::new("asset", "read"));
        role.add_permission(Permission::new("asset", "write"));

        assert_eq!(role.name, "developer");
        assert_eq!(role.permissions.len(), 2);
    }

    #[test]
    fn test_rbac_policy() {
        let mut policy = RbacPolicy::new();

        // Test default roles
        assert!(policy.get_role("admin").is_some());
        assert!(policy.get_role("developer").is_some());
        assert!(policy.get_role("viewer").is_some());
    }

    #[test]
    fn test_permission_checking() {
        let mut policy = RbacPolicy::new();

        let admin_roles = vec!["admin".to_string()];
        let viewer_roles = vec!["viewer".to_string()];

        let read_perm = Permission::new("asset", "read");
        let delete_perm = Permission::new("asset", "delete");

        // Admin should have all permissions
        assert!(policy.has_permission(&admin_roles, &read_perm));
        assert!(policy.has_permission(&admin_roles, &delete_perm));

        // Viewer should only have read permission
        assert!(policy.has_permission(&viewer_roles, &read_perm));
        assert!(!policy.has_permission(&viewer_roles, &delete_perm));
    }

    #[test]
    fn test_role_inheritance() {
        let mut policy = RbacPolicy::new();

        // Create a moderator role that inherits from viewer
        let mut moderator = Role::new("moderator");
        moderator.add_parent("viewer");
        moderator.add_permission(Permission::new("asset", "delete"));
        policy.add_role(moderator);

        let moderator_roles = vec!["moderator".to_string()];

        // Should have permissions from both moderator and viewer
        assert!(policy.has_permission(
            &moderator_roles,
            &Permission::new("asset", "read")
        ));
        assert!(policy.has_permission(
            &moderator_roles,
            &Permission::new("asset", "delete")
        ));
    }

    #[test]
    fn test_has_any_permission() {
        let mut policy = RbacPolicy::new();

        let developer_roles = vec!["developer".to_string()];
        let permissions = vec![
            Permission::new("asset", "delete"),
            Permission::new("user", "admin"),
        ];

        // Developer should have at least one of these permissions
        assert!(policy.has_any_permission(&developer_roles, &permissions));
    }

    #[test]
    fn test_has_all_permissions() {
        let mut policy = RbacPolicy::new();

        let admin_roles = vec!["admin".to_string()];
        let permissions = vec![
            Permission::new("asset", "read"),
            Permission::new("asset", "write"),
            Permission::new("asset", "delete"),
        ];

        // Admin should have all permissions
        assert!(policy.has_all_permissions(&admin_roles, &permissions));
    }

    #[test]
    fn test_cache_invalidation() {
        let mut policy = RbacPolicy::new();

        // Get permissions to populate cache
        let _ = policy.get_role_permissions("developer");
        assert!(policy.permission_cache.contains_key("developer"));

        // Modify role - should invalidate cache
        if let Some(role) = policy.roles.get_mut("developer") {
            role.add_permission(Permission::new("new", "permission"));
        }
        policy.invalidate_cache("developer");

        assert!(!policy.permission_cache.contains_key("developer"));
    }
}
