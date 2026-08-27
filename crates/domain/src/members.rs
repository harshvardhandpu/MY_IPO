//! CoreMember and FriendAccount domain types.
//!
//! Profiles carry only the *masked* PAN; the full PAN lives exclusively in the
//! encrypted identity envelope (see `sanket-identity-security`). Archive, never
//! delete: `MemberStatus` has no deleted variant and these types expose no
//! removal method.

use serde::{Deserialize, Serialize};

use crate::money::BasisPoints;
use sanket_identity_security::MaskedPan;

/// Lifecycle status. Deliberately only two states — there is no delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MemberStatus {
    Active,
    Archived,
}

/// A core group member (owner or participant).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreMember {
    id: String,
    display_name: String,
    role: crate::Role,
    masked_pan: MaskedPan,
    sensitive_record_id: String,
    primary_account_id: Option<String>,
    status: MemberStatus,
}

impl CoreMember {
    /// Onboard a member. PAN is mandatory: the masked form must be produced
    /// from a validated `Pan` before this call (the full PAN never enters the
    /// profile).
    pub fn onboard(
        id: impl Into<String>,
        display_name: impl Into<String>,
        role: crate::Role,
        masked_pan: MaskedPan,
        sensitive_record_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            role,
            masked_pan,
            sensitive_record_id: sensitive_record_id.into(),
            primary_account_id: None,
            status: MemberStatus::Active,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn role(&self) -> crate::Role {
        self.role
    }

    pub fn masked_pan(&self) -> &MaskedPan {
        &self.masked_pan
    }

    pub fn sensitive_record_id(&self) -> &str {
        &self.sensitive_record_id
    }

    pub fn primary_account_id(&self) -> Option<&str> {
        self.primary_account_id.as_deref()
    }

    pub fn status(&self) -> MemberStatus {
        self.status
    }

    /// Designate (or re-designate) the primary account. Exactly one primary.
    pub fn designate_primary_account(&mut self, account_id: impl Into<String>) {
        self.primary_account_id = Some(account_id.into());
    }

    /// Archive, never delete. Archived members keep their history.
    pub fn archive(&mut self) {
        self.status = MemberStatus::Archived;
    }
}

/// A friend account invested through a core member.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendAccount {
    id: String,
    owner_member_id: String,
    label: String,
    masked_pan: MaskedPan,
    sensitive_record_id: String,
    share_basis_points: BasisPoints,
    status: MemberStatus,
}

/// Errors for friend-account mutations.
#[derive(Debug, thiserror::Error)]
#[error("friend share basis points out of range: {0}")]
pub struct FriendShareError(pub i64);

impl FriendAccount {
    /// Create a friend account. PAN mandatory; share defaults to 10%.
    pub fn create(
        id: impl Into<String>,
        owner_member_id: impl Into<String>,
        label: impl Into<String>,
        masked_pan: MaskedPan,
        sensitive_record_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            owner_member_id: owner_member_id.into(),
            label: label.into(),
            masked_pan,
            sensitive_record_id: sensitive_record_id.into(),
            share_basis_points: BasisPoints::friend_share_default(),
            status: MemberStatus::Active,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn owner_member_id(&self) -> &str {
        &self.owner_member_id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn masked_pan(&self) -> &MaskedPan {
        &self.masked_pan
    }

    pub fn sensitive_record_id(&self) -> &str {
        &self.sensitive_record_id
    }

    pub fn share_basis_points(&self) -> BasisPoints {
        self.share_basis_points
    }

    pub fn status(&self) -> MemberStatus {
        self.status
    }

    /// Update the friend's profit share. Range is enforced by `BasisPoints`.
    pub fn set_share_basis_points(&mut self, bp: BasisPoints) -> Result<(), FriendShareError> {
        BasisPoints::try_new(bp.value()).map_err(|e| FriendShareError(e.0))?;
        self.share_basis_points = bp;
        Ok(())
    }

    /// Only active friends are investable.
    pub fn is_investable(&self) -> bool {
        self.status == MemberStatus::Active
    }

    /// Archive, never delete.
    pub fn archive(&mut self) {
        self.status = MemberStatus::Archived;
    }
}
