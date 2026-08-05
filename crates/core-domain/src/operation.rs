//! iRedAdmin bulk operation types: target × operation kind.
//!
//! MailGrit works only with open-source iRedAdmin (OSE, HTML forms). Targets are
//! limited to the entities that OSE can create/edit/delete through forms: domain,
//! user, administrator. The pair
//! ([`OperationTarget`], [`BulkOperationKind`]) uniquely identifies an operation.

/// Bulk operation target — the iRedAdmin entity the operation acts upon.
/// The pair (OperationTarget, BulkOperationKind) uniquely identifies an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationTarget {
    /// Mail domain.
    Domain,
    /// Mailbox / user.
    User,
    /// iRedAdmin administrator (global or domain-scoped).
    Admin,
}

/// Kind of bulk operation on an iRedAdmin entity.
///
/// MailGrit supports only the three operation kinds that OSE iRedAdmin can perform
/// through HTML forms: create, edit, and delete. Other kinds (disabling/enabling
/// accounts, bulk attribute updates) are not implemented — the UI and webview JS
/// never construct them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BulkOperationKind {
    /// Entity creation (POST to the creation endpoint).
    Create,
    /// Edit of an existing entity (POST/PUT to its profile).
    Edit,
    /// Entity deletion (fail-closed: requires confirmation in the UI).
    Delete,
}

impl BulkOperationKind {
    /// Returns the string representation of the operation kind (for audit log/UI).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "CREATE",
            Self::Edit => "EDIT",
            Self::Delete => "DELETE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_as_str() {
        assert_eq!(BulkOperationKind::Create.as_str(), "CREATE");
        assert_eq!(BulkOperationKind::Edit.as_str(), "EDIT");
        assert_eq!(BulkOperationKind::Delete.as_str(), "DELETE");
    }

    #[test]
    fn kind_is_send_sync_copy() {
        fn assert_props<T: Send + Sync + Copy + 'static>() {}
        assert_props::<BulkOperationKind>();
    }

    // MailGrit supports only OSE entities (Domain/User/Admin).
    #[test]
    fn target_all_variants_are_distinct() {
        let all: Vec<OperationTarget> = vec![
            OperationTarget::Domain,
            OperationTarget::User,
            OperationTarget::Admin,
        ];
        // All variants are distinct: collect into a HashSet and compare the element count.
        // OperationTarget: Eq + Hash (no Ord), hence a HashSet instead of sort + dedup.
        let unique_count: std::collections::HashSet<OperationTarget> =
            all.iter().copied().collect();
        assert_eq!(
            unique_count.len(),
            all.len(),
            "duplicates among OperationTarget"
        );
    }

    #[test]
    fn target_is_send_sync_copy() {
        fn assert_props<T: Send + Sync + Copy + 'static>() {}
        assert_props::<OperationTarget>();
    }
}
