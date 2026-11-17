#![forbid(unsafe_code)]
#![allow(dead_code)]

//! Active Directory attack paths trên fixture hoàn toàn tổng hợp.
//! Không có credential thật, cryptography, network/RPC, OS command, process injection
//! hoặc token manipulation. Mọi "artifact" chỉ là nhãn để kiểm tra state machine.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const ENGAGEMENT_ID: &str = "ENG-SYNTHETIC-108";
pub const EVIDENCE_KERBEROS: &str = "ev-kerberos";
pub const EVIDENCE_ASREP: &str = "ev-asrep";
pub const EVIDENCE_DELEGATION: &str = "ev-delegation";
pub const EVIDENCE_TICKET: &str = "ev-ticket";
pub const EVIDENCE_ACL: &str = "ev-acl";
pub const EVIDENCE_REPLICATION: &str = "ev-replication";
pub const EVIDENCE_IDENTITY: &str = "ev-identity";
pub const SYNTHETIC_HASH: &str = "SYNTHETIC-NOT-A-REAL-HASH";
const MAX_PATH_STEPS: usize = 8;

// ============================================================================
// 1. EVIDENCE, SCOPE VÀ CÁC KỸ THUẬT
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Evidence {
    pub id: String,
    pub source: String,
    pub detail: String,
}

impl Evidence {
    pub fn new(id: &str, source: &str, detail: &str) -> Self {
        Self {
            id: id.to_string(),
            source: source.to_string(),
            detail: detail.to_string(),
        }
    }

    fn is_valid(&self) -> bool {
        !self.id.trim().is_empty()
            && self.source.starts_with("synthetic-")
            && !self.detail.trim().is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttackTechnique {
    Kerberoasting,
    AsRepRoasting,
    UnconstrainedDelegation,
    ConstrainedDelegation,
    ResourceBasedConstrainedDelegation,
    GenericAll,
    WriteDacl,
    WriteOwner,
    ForceChangePassword,
    DcSync,
    PassTheHash,
    OverPassTheHash,
}

impl AttackTechnique {
    fn required_evidence(self) -> &'static str {
        match self {
            Self::Kerberoasting => EVIDENCE_KERBEROS,
            Self::AsRepRoasting => EVIDENCE_ASREP,
            Self::UnconstrainedDelegation
            | Self::ConstrainedDelegation
            | Self::ResourceBasedConstrainedDelegation => EVIDENCE_DELEGATION,
            Self::GenericAll | Self::WriteDacl | Self::WriteOwner | Self::ForceChangePassword => {
                EVIDENCE_ACL
            }
            Self::DcSync => EVIDENCE_REPLICATION,
            Self::PassTheHash | Self::OverPassTheHash => EVIDENCE_IDENTITY,
        }
    }

    fn allowed_evidence(self) -> &'static [&'static str] {
        match self {
            Self::Kerberoasting => &[EVIDENCE_KERBEROS],
            Self::AsRepRoasting => &[EVIDENCE_ASREP],
            Self::UnconstrainedDelegation
            | Self::ConstrainedDelegation
            | Self::ResourceBasedConstrainedDelegation => &[EVIDENCE_DELEGATION, EVIDENCE_TICKET],
            Self::GenericAll | Self::WriteDacl | Self::WriteOwner | Self::ForceChangePassword => {
                &[EVIDENCE_ACL]
            }
            Self::DcSync => &[EVIDENCE_REPLICATION],
            Self::PassTheHash | Self::OverPassTheHash => &[EVIDENCE_IDENTITY],
        }
    }

    fn acl_right(self) -> Option<AclRight> {
        match self {
            Self::GenericAll => Some(AclRight::GenericAll),
            Self::WriteDacl => Some(AclRight::WriteDacl),
            Self::WriteOwner => Some(AclRight::WriteOwner),
            Self::ForceChangePassword => Some(AclRight::ForceChangePassword),
            _ => None,
        }
    }

    fn synthetic_event_id(self) -> u16 {
        match self {
            Self::Kerberoasting
            | Self::UnconstrainedDelegation
            | Self::ConstrainedDelegation
            | Self::ResourceBasedConstrainedDelegation => 4769,
            Self::AsRepRoasting | Self::OverPassTheHash => 4768,
            Self::GenericAll | Self::WriteDacl | Self::WriteOwner => 5136,
            Self::ForceChangePassword => 4724,
            Self::DcSync => 4662,
            Self::PassTheHash => 4624,
        }
    }

    fn telemetry_kind(self) -> TelemetryKind {
        match self {
            Self::Kerberoasting => TelemetryKind::ServiceTicket,
            Self::AsRepRoasting => TelemetryKind::AuthenticationReply,
            Self::UnconstrainedDelegation
            | Self::ConstrainedDelegation
            | Self::ResourceBasedConstrainedDelegation => TelemetryKind::Delegation,
            Self::GenericAll | Self::WriteDacl | Self::WriteOwner | Self::ForceChangePassword => {
                TelemetryKind::DirectoryMutation
            }
            Self::DcSync => TelemetryKind::Replication,
            Self::PassTheHash => TelemetryKind::IdentityLogon,
            Self::OverPassTheHash => TelemetryKind::TicketGranting,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngagementScope {
    pub id: String,
    pub techniques: BTreeSet<AttackTechnique>,
    pub actors: BTreeSet<String>,
    pub targets: BTreeSet<String>,
}

// ============================================================================
// 2. OBJECT, ARTIFACT VÀ DELEGATION TỔNG HỢP
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountKind {
    User,
    ServiceAccount,
    Computer,
    Domain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
    pub spn: Option<String>,
    pub preauthentication_required: bool,
    pub service_ticket_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CredentialUse {
    ServiceTicketVerifier,
    AsRepVerifier,
    NtHashLabel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticCredential {
    pub id: String,
    pub account: String,
    pub intended_use: CredentialUse,
    /// Không phải hash/credential dùng được; validator chỉ chấp nhận nhãn tổng hợp.
    pub artifact: String,
    /// Chuỗi so khớp cho dictionary giả, không phải password hay derived key.
    pub offline_verifier: String,
    pub usable: bool,
    pub evidence_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyntheticTicketKind {
    Tgt,
    Service,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticTicket {
    pub id: String,
    pub kind: SyntheticTicketKind,
    pub realm: String,
    pub client: String,
    pub service: String,
    pub label: String,
    pub evidence_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DelegationKind {
    Unconstrained,
    Constrained,
    ResourceBasedConstrained,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationRule {
    pub kind: DelegationKind,
    /// Account được cấu hình delegation (hoặc được RBCD cho phép).
    pub principal: String,
    /// Với RBCD, đây là resource chứa rule; các loại khác để trống.
    pub resource: Option<String>,
    /// Constrained delegation có allowlist target tường minh.
    pub allowed_targets: BTreeSet<String>,
    pub evidence_id: String,
}

impl DelegationRule {
    pub fn unconstrained(principal: &str, evidence_id: &str) -> Self {
        Self {
            kind: DelegationKind::Unconstrained,
            principal: principal.to_string(),
            resource: None,
            allowed_targets: BTreeSet::new(),
            evidence_id: evidence_id.to_string(),
        }
    }

    pub fn constrained(
        principal: &str,
        allowed_targets: BTreeSet<String>,
        evidence_id: &str,
    ) -> Self {
        Self {
            kind: DelegationKind::Constrained,
            principal: principal.to_string(),
            resource: None,
            allowed_targets,
            evidence_id: evidence_id.to_string(),
        }
    }

    pub fn resource_based(principal: &str, resource: &str, evidence_id: &str) -> Self {
        Self {
            kind: DelegationKind::ResourceBasedConstrained,
            principal: principal.to_string(),
            resource: Some(resource.to_string()),
            allowed_targets: BTreeSet::new(),
            evidence_id: evidence_id.to_string(),
        }
    }
}

// ============================================================================
// 3. ATTEMPT, FINDING, TELEMETRY VÀ REMEDIATION
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackAttempt {
    pub engagement: String,
    pub technique: AttackTechnique,
    pub actor: String,
    pub target: String,
    pub credential_id: Option<String>,
    pub ticket_id: Option<String>,
    pub dictionary: Vec<String>,
    pub evidence_ids: BTreeSet<String>,
}

impl AttackAttempt {
    pub fn new(technique: AttackTechnique, actor: &str, target: &str) -> Self {
        Self {
            engagement: ENGAGEMENT_ID.to_string(),
            technique,
            actor: actor.to_string(),
            target: target.to_string(),
            credential_id: None,
            ticket_id: None,
            dictionary: Vec::new(),
            evidence_ids: BTreeSet::new(),
        }
    }

    pub fn with_credential(mut self, credential_id: &str) -> Self {
        self.credential_id = Some(credential_id.to_string());
        self
    }

    pub fn with_ticket(mut self, ticket_id: &str) -> Self {
        self.ticket_id = Some(ticket_id.to_string());
        self
    }

    pub fn with_dictionary(mut self, candidates: &[&str]) -> Self {
        self.dictionary = candidates
            .iter()
            .map(|candidate| (*candidate).to_string())
            .collect();
        self
    }

    pub fn with_evidence(mut self, evidence_id: &str) -> Self {
        self.evidence_ids.insert(evidence_id.to_string());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingOutcome {
    RoastableMaterial,
    OfflineDictionaryMatch,
    DelegationApproved,
    AclActionRolledBack,
    SyntheticSecretDisclosed,
    IdentityStateChanged,
    TicketStateChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompromiseStatus {
    /// Có material hoặc offline match không chứng minh chiếm account.
    NotEstablished,
    /// Chỉ có transition trong mock, không phải account/host thật bị chiếm.
    SyntheticStateOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityStateTransition {
    pub session: String,
    pub before: String,
    pub after: String,
    pub resource: String,
    pub ticket: Option<SyntheticTicket>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryKind {
    ServiceTicket,
    AuthenticationReply,
    OfflineDictionary,
    Delegation,
    DirectoryMutation,
    Replication,
    IdentityLogon,
    TicketGranting,
    ScopeEnforcement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOutcome {
    Observed,
    Matched,
    Rejected,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionEvent {
    /// Logical sequence, không đọc đồng hồ hệ thống.
    pub sequence: u64,
    /// ID Windows tương ứng để học detection; bản thân event này là dữ liệu giả.
    pub synthetic_event_id: u16,
    pub kind: TelemetryKind,
    pub technique: AttackTechnique,
    pub actor: String,
    pub target: String,
    pub outcome: EventOutcome,
    pub detail: String,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remediation {
    pub technique: AttackTechnique,
    pub actions: Vec<String>,
    pub retest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackFinding {
    pub technique: AttackTechnique,
    pub actor: String,
    pub target: String,
    pub outcome: FindingOutcome,
    pub compromise: CompromiseStatus,
    pub evidence: Vec<Evidence>,
    pub detection: Vec<DetectionEvent>,
    pub remediation: Remediation,
    pub synthetic_secret: Option<String>,
    pub state_transition: Option<IdentityStateTransition>,
}

// ============================================================================
// 4. ACL TRANSACTION VÀ COMPOSED PATH
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AclRight {
    GenericAll,
    WriteDacl,
    WriteOwner,
    ForceChangePassword,
    ReplicationGetChanges,
    ReplicationGetChangesAll,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AclGrant {
    pub actor: String,
    pub target: String,
    pub right: AclRight,
    pub evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AclState {
    pub grants: BTreeSet<AclGrant>,
    pub owners: BTreeMap<String, String>,
    pub controlled_objects: BTreeSet<String>,
    pub password_reset_markers: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AclMutation {
    pub id: String,
    pub technique: AttackTechnique,
    pub actor: String,
    pub target: String,
    pub before: AclState,
    pub after: AclState,
    pub rolled_back: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupRecord {
    pub mutation_id: String,
    pub restored: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedPath {
    pub start: String,
    pub goal: String,
    pub findings: Vec<AttackFinding>,
}

impl ComposedPath {
    pub fn validate(&self) -> bool {
        if self.findings.is_empty() || self.findings.len() > MAX_PATH_STEPS {
            return false;
        }
        let mut visited = BTreeSet::from([self.start.clone()]);
        let mut current = self.start.as_str();
        for finding in &self.findings {
            if finding.actor != current
                || finding.evidence.is_empty()
                || !visited.insert(finding.target.clone())
            {
                return false;
            }
            current = &finding.target;
        }
        current == self.goal
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttackError {
    OutOfScope {
        engagement: String,
        technique: AttackTechnique,
        actor: String,
        target: String,
    },
    UnknownIdentity(String),
    MissingEvidence,
    UnknownEvidence(String),
    UnrelatedEvidence(String),
    WrongEvidence {
        expected: String,
    },
    UnknownCredential(String),
    UnknownTicket(String),
    MissingPrecondition {
        technique: AttackTechnique,
        reason: &'static str,
    },
    WrongSyntheticCandidate,
    EmptyPath,
    PathTooLong {
        actual: usize,
        maximum: usize,
    },
    PathDisconnected {
        step: usize,
    },
    PathCycle {
        step: usize,
        identity: String,
    },
    GoalMismatch {
        expected: String,
        actual: String,
    },
    PathFailed {
        step: usize,
        error: Box<AttackError>,
    },
    NoStateChange {
        technique: AttackTechnique,
        target: String,
    },
    RemediationFailed(AttackTechnique),
}

impl fmt::Display for AttackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfScope {
                engagement,
                technique,
                actor,
                target,
            } => write!(
                f,
                "ngoài engagement {engagement}: {technique:?}, actor={actor}, target={target}"
            ),
            Self::WrongSyntheticCandidate => {
                f.write_str("không candidate tổng hợp nào khớp; không có offline match")
            }
            Self::MissingPrecondition { technique, reason } => {
                write!(f, "{technique:?}: thiếu precondition: {reason}")
            }
            Self::PathFailed { step, error } => {
                write!(f, "path dừng ở step {step}: {error}")
            }
            Self::NoStateChange { technique, target } => {
                write!(
                    f,
                    "{technique:?}: target {target} đã ở trạng thái đích; không có mutation"
                )
            }
            Self::UnrelatedEvidence(id) => {
                write!(f, "evidence không thuộc loại của technique: {id}")
            }
            _ => write!(f, "mock AD attack bị từ chối: {self:?}"),
        }
    }
}

impl Error for AttackError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PathFailed { error, .. } => Some(error),
            _ => None,
        }
    }
}

// ============================================================================
// 5. MOCK ENGINE: VALIDATE TRƯỚC, MUTATE SAU, ACL LUÔN ROLLBACK
// ============================================================================

#[derive(Debug, Clone)]
pub struct MockADAttacks {
    pub scope: EngagementScope,
    pub accounts: BTreeMap<String, Account>,
    pub credentials: BTreeMap<String, SyntheticCredential>,
    pub tickets: BTreeMap<String, SyntheticTicket>,
    pub delegations: Vec<DelegationRule>,
    pub acl: AclState,
    pub evidence: BTreeMap<String, Evidence>,
    // Issuance snapshot tách artifact do fixture phát hành khỏi dữ liệu caller tự thêm.
    // `usable` vẫn là live state để remediation revoke credential.
    issued_credentials: BTreeMap<String, SyntheticCredential>,
    issued_tickets: BTreeMap<String, SyntheticTicket>,
    issued_evidence: BTreeMap<String, Evidence>,
    events: Vec<DetectionEvent>,
    mutations: Vec<AclMutation>,
    cleanup: Vec<CleanupRecord>,
    sessions: BTreeMap<String, IdentityStateTransition>,
}

impl MockADAttacks {
    pub fn standard() -> Self {
        let evidence_rows = vec![
            Evidence::new(
                EVIDENCE_KERBEROS,
                "synthetic-kdc-log",
                "SPN, synthetic service ticket và offline verifier của svc-web",
            ),
            Evidence::new(
                EVIDENCE_ASREP,
                "synthetic-account-policy",
                "legacy@lab.example có pre-authentication=false trong fixture",
            ),
            Evidence::new(
                EVIDENCE_DELEGATION,
                "synthetic-delegation-export",
                "ba rule unconstrained/constrained/RBCD được ghi riêng",
            ),
            Evidence::new(
                EVIDENCE_TICKET,
                "synthetic-ticket-store",
                "ticket labels chỉ phục vụ kiểm tra kind/client/service",
            ),
            Evidence::new(
                EVIDENCE_ACL,
                "synthetic-acl-export",
                "grant ACL được resolve sẵn; không có Windows ACL mutation thật",
            ),
            Evidence::new(
                EVIDENCE_REPLICATION,
                "synthetic-domain-acl",
                "DCSync cần đồng thời ReplicationGetChanges và ReplicationGetChangesAll",
            ),
            Evidence::new(
                EVIDENCE_IDENTITY,
                "synthetic-identity-fixture",
                "PtH/OPtH chỉ đổi typed session state trong bộ nhớ",
            ),
        ];
        let evidence: BTreeMap<String, Evidence> = evidence_rows
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect();

        let account_rows = vec![
            account("analyst@lab.example", AccountKind::User, None, true, false),
            account(
                "svc-web@lab.example",
                AccountKind::ServiceAccount,
                Some("HTTP/web.lab.example"),
                true,
                true,
            ),
            account("legacy@lab.example", AccountKind::User, None, false, false),
            account(
                "svc-front@lab.example",
                AccountKind::ServiceAccount,
                Some("HTTP/front.lab.example"),
                true,
                true,
            ),
            account("helpdesk@lab.example", AccountKind::User, None, true, false),
            account("bob@lab.example", AccountKind::User, None, true, false),
            account(
                "server01.lab.example",
                AccountKind::Computer,
                None,
                true,
                false,
            ),
            account("ws01.lab.example", AccountKind::Computer, None, true, false),
            account("lab.example", AccountKind::Domain, None, true, false),
            // Object tồn tại nhưng cố ý nằm ngoài engagement scope.
            account("outside.example", AccountKind::Computer, None, true, false),
        ];
        let accounts: BTreeMap<String, Account> = account_rows
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect();

        let credential_rows = vec![
            credential(
                "cred-kerb-web",
                "svc-web@lab.example",
                CredentialUse::ServiceTicketVerifier,
                "SYNTHETIC-CANDIDATE-WEB",
                EVIDENCE_KERBEROS,
            ),
            credential(
                "cred-asrep-legacy",
                "legacy@lab.example",
                CredentialUse::AsRepVerifier,
                "SYNTHETIC-CANDIDATE-LEGACY",
                EVIDENCE_ASREP,
            ),
            credential(
                "cred-pth-web",
                "svc-web@lab.example",
                CredentialUse::NtHashLabel,
                "SYNTHETIC-NOT-APPLICABLE",
                EVIDENCE_IDENTITY,
            ),
        ];
        let credentials: BTreeMap<String, SyntheticCredential> = credential_rows
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect();

        let ticket_rows = vec![
            SyntheticTicket {
                id: "ticket-tgt-bob".to_string(),
                kind: SyntheticTicketKind::Tgt,
                realm: "LAB.EXAMPLE".to_string(),
                client: "bob@lab.example".to_string(),
                service: "krbtgt/lab.example".to_string(),
                label: "SYNTHETIC-TICKET-TGT-BOB".to_string(),
                evidence_id: EVIDENCE_TICKET.to_string(),
            },
            SyntheticTicket {
                id: "ticket-service-bob".to_string(),
                kind: SyntheticTicketKind::Service,
                realm: "LAB.EXAMPLE".to_string(),
                client: "bob@lab.example".to_string(),
                service: "svc-front@lab.example".to_string(),
                label: "SYNTHETIC-TICKET-SERVICE-BOB".to_string(),
                evidence_id: EVIDENCE_TICKET.to_string(),
            },
        ];
        let tickets: BTreeMap<String, SyntheticTicket> = ticket_rows
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect();

        let delegations = vec![
            DelegationRule::unconstrained("svc-front@lab.example", EVIDENCE_DELEGATION),
            DelegationRule::constrained(
                "svc-front@lab.example",
                BTreeSet::from(["server01.lab.example".to_string()]),
                EVIDENCE_DELEGATION,
            ),
            DelegationRule::resource_based(
                "svc-front@lab.example",
                "server01.lab.example",
                EVIDENCE_DELEGATION,
            ),
        ];

        let grant_rows = vec![
            grant(
                "analyst@lab.example",
                "bob@lab.example",
                AclRight::GenericAll,
            ),
            grant(
                "helpdesk@lab.example",
                "bob@lab.example",
                AclRight::WriteDacl,
            ),
            grant(
                "helpdesk@lab.example",
                "bob@lab.example",
                AclRight::WriteOwner,
            ),
            grant(
                "helpdesk@lab.example",
                "bob@lab.example",
                AclRight::ForceChangePassword,
            ),
            AclGrant {
                actor: "analyst@lab.example".to_string(),
                target: "lab.example".to_string(),
                right: AclRight::ReplicationGetChanges,
                evidence_id: EVIDENCE_REPLICATION.to_string(),
            },
            AclGrant {
                actor: "analyst@lab.example".to_string(),
                target: "lab.example".to_string(),
                right: AclRight::ReplicationGetChangesAll,
                evidence_id: EVIDENCE_REPLICATION.to_string(),
            },
        ];
        let acl = AclState {
            grants: grant_rows.into_iter().collect(),
            owners: BTreeMap::from([(
                "bob@lab.example".to_string(),
                "analyst@lab.example".to_string(),
            )]),
            controlled_objects: BTreeSet::new(),
            password_reset_markers: BTreeSet::new(),
        };
        let techniques = [
            AttackTechnique::Kerberoasting,
            AttackTechnique::AsRepRoasting,
            AttackTechnique::UnconstrainedDelegation,
            AttackTechnique::ConstrainedDelegation,
            AttackTechnique::ResourceBasedConstrainedDelegation,
            AttackTechnique::GenericAll,
            AttackTechnique::WriteDacl,
            AttackTechnique::WriteOwner,
            AttackTechnique::ForceChangePassword,
            AttackTechnique::DcSync,
            AttackTechnique::PassTheHash,
            AttackTechnique::OverPassTheHash,
        ]
        .into_iter()
        .collect();
        let actors = accounts
            .keys()
            .filter(|id| id.as_str() != "outside.example" && id.as_str() != "lab.example")
            .cloned()
            .collect();
        let targets = accounts
            .keys()
            .filter(|id| id.as_str() != "outside.example")
            .cloned()
            .collect();
        Self {
            scope: EngagementScope {
                id: ENGAGEMENT_ID.to_string(),
                techniques,
                actors,
                targets,
            },
            accounts,
            issued_credentials: credentials.clone(),
            credentials,
            issued_tickets: tickets.clone(),
            tickets,
            delegations,
            acl,
            issued_evidence: evidence.clone(),
            evidence,
            events: Vec::new(),
            mutations: Vec::new(),
            cleanup: Vec::new(),
            sessions: BTreeMap::new(),
        }
    }

    /// Template hợp lệ của fixture; caller vẫn phải qua toàn bộ validation.
    pub fn attempt_template(&self, technique: AttackTechnique) -> AttackAttempt {
        match technique {
            AttackTechnique::Kerberoasting => {
                AttackAttempt::new(technique, "analyst@lab.example", "svc-web@lab.example")
                    .with_credential("cred-kerb-web")
                    .with_evidence(EVIDENCE_KERBEROS)
            }
            AttackTechnique::AsRepRoasting => {
                AttackAttempt::new(technique, "analyst@lab.example", "legacy@lab.example")
                    .with_credential("cred-asrep-legacy")
                    .with_evidence(EVIDENCE_ASREP)
            }
            AttackTechnique::UnconstrainedDelegation => {
                AttackAttempt::new(technique, "svc-front@lab.example", "server01.lab.example")
                    .with_ticket("ticket-tgt-bob")
                    .with_evidence(EVIDENCE_DELEGATION)
                    .with_evidence(EVIDENCE_TICKET)
            }
            AttackTechnique::ConstrainedDelegation
            | AttackTechnique::ResourceBasedConstrainedDelegation => {
                AttackAttempt::new(technique, "svc-front@lab.example", "server01.lab.example")
                    .with_ticket("ticket-service-bob")
                    .with_evidence(EVIDENCE_DELEGATION)
                    .with_evidence(EVIDENCE_TICKET)
            }
            AttackTechnique::GenericAll => {
                AttackAttempt::new(technique, "analyst@lab.example", "bob@lab.example")
                    .with_evidence(EVIDENCE_ACL)
            }
            AttackTechnique::WriteDacl
            | AttackTechnique::WriteOwner
            | AttackTechnique::ForceChangePassword => {
                AttackAttempt::new(technique, "helpdesk@lab.example", "bob@lab.example")
                    .with_evidence(EVIDENCE_ACL)
            }
            AttackTechnique::DcSync => {
                AttackAttempt::new(technique, "analyst@lab.example", "lab.example")
                    .with_evidence(EVIDENCE_REPLICATION)
            }
            AttackTechnique::PassTheHash | AttackTechnique::OverPassTheHash => {
                AttackAttempt::new(technique, "svc-web@lab.example", "server01.lab.example")
                    .with_credential("cred-pth-web")
                    .with_evidence(EVIDENCE_IDENTITY)
            }
        }
    }

    pub fn events(&self) -> &[DetectionEvent] {
        &self.events
    }

    pub fn acl_mutations(&self) -> &[AclMutation] {
        &self.mutations
    }

    pub fn cleanup_records(&self) -> &[CleanupRecord] {
        &self.cleanup
    }

    pub fn sessions(&self) -> &BTreeMap<String, IdentityStateTransition> {
        &self.sessions
    }

    fn precondition(&self, technique: AttackTechnique, reason: &'static str) -> AttackError {
        AttackError::MissingPrecondition { technique, reason }
    }

    fn registered_evidence(&self, id: &str) -> Result<&Evidence, AttackError> {
        match (self.evidence.get(id), self.issued_evidence.get(id)) {
            (Some(evidence), Some(issued)) if evidence == issued && evidence.is_valid() => {
                Ok(evidence)
            }
            _ => Err(AttackError::UnknownEvidence(id.to_string())),
        }
    }

    fn registered_credential(&self, id: &str) -> Result<&SyntheticCredential, AttackError> {
        let unknown = || AttackError::UnknownCredential(id.to_string());
        let credential = self.credentials.get(id).ok_or_else(unknown)?;
        let issued = self
            .issued_credentials
            .get(id)
            .ok_or_else(|| AttackError::UnknownCredential(id.to_string()))?;
        // Chỉ `usable` được thay đổi; không cho caller rebind owner/use/verifier/evidence.
        if credential.id != id
            || credential.account != issued.account
            || credential.intended_use != issued.intended_use
            || credential.artifact != issued.artifact
            || credential.offline_verifier != issued.offline_verifier
            || credential.evidence_id != issued.evidence_id
        {
            return Err(AttackError::UnknownCredential(id.to_string()));
        }
        Ok(credential)
    }

    fn registered_ticket(&self, id: &str) -> Result<&SyntheticTicket, AttackError> {
        match (self.tickets.get(id), self.issued_tickets.get(id)) {
            (Some(ticket), Some(issued)) if ticket == issued => Ok(ticket),
            _ => Err(AttackError::UnknownTicket(id.to_string())),
        }
    }

    fn validate_attempt(&self, attempt: &AttackAttempt) -> Result<(), AttackError> {
        if attempt.engagement != self.scope.id
            || !self.scope.techniques.contains(&attempt.technique)
            || !self.scope.actors.contains(&attempt.actor)
            || !self.scope.targets.contains(&attempt.target)
        {
            return Err(AttackError::OutOfScope {
                engagement: attempt.engagement.clone(),
                technique: attempt.technique,
                actor: attempt.actor.clone(),
                target: attempt.target.clone(),
            });
        }
        for identity in [&attempt.actor, &attempt.target] {
            if !self.accounts.contains_key(identity) {
                return Err(AttackError::UnknownIdentity(identity.clone()));
            }
        }
        if attempt.evidence_ids.is_empty() {
            return Err(AttackError::MissingEvidence);
        }
        for evidence_id in &attempt.evidence_ids {
            self.registered_evidence(evidence_id)?;
        }
        let allowed = attempt.technique.allowed_evidence();
        if let Some(unrelated) = attempt
            .evidence_ids
            .iter()
            .find(|id| !allowed.contains(&id.as_str()))
        {
            return Err(AttackError::UnrelatedEvidence(unrelated.clone()));
        }
        let required = attempt.technique.required_evidence();
        if !attempt.evidence_ids.contains(required) {
            return Err(AttackError::WrongEvidence {
                expected: required.to_string(),
            });
        }
        if let Some(credential_id) = &attempt.credential_id {
            let credential = self.registered_credential(credential_id)?;
            if !attempt.evidence_ids.contains(&credential.evidence_id) {
                return Err(AttackError::WrongEvidence {
                    expected: credential.evidence_id.clone(),
                });
            }
        }
        if let Some(ticket_id) = &attempt.ticket_id {
            let ticket = self.registered_ticket(ticket_id)?;
            if !ticket.label.starts_with("SYNTHETIC-TICKET-") {
                return Err(self.precondition(attempt.technique, "ticket label không tổng hợp"));
            }
            if !attempt.evidence_ids.contains(&ticket.evidence_id) {
                return Err(AttackError::WrongEvidence {
                    expected: ticket.evidence_id.clone(),
                });
            }
        }

        match attempt.technique {
            AttackTechnique::Kerberoasting | AttackTechnique::AsRepRoasting => {
                self.validate_roasting(attempt)
            }
            AttackTechnique::UnconstrainedDelegation
            | AttackTechnique::ConstrainedDelegation
            | AttackTechnique::ResourceBasedConstrainedDelegation => {
                self.validate_delegation(attempt)
            }
            AttackTechnique::GenericAll
            | AttackTechnique::WriteDacl
            | AttackTechnique::WriteOwner
            | AttackTechnique::ForceChangePassword => self.validate_acl(attempt),
            AttackTechnique::DcSync => self.validate_dcsync(attempt),
            AttackTechnique::PassTheHash | AttackTechnique::OverPassTheHash => {
                self.validate_identity_transition(attempt)
            }
        }
    }

    fn validate_roasting(&self, attempt: &AttackAttempt) -> Result<(), AttackError> {
        let target = &self.accounts[&attempt.target];
        match attempt.technique {
            AttackTechnique::Kerberoasting => {
                if target.kind != AccountKind::ServiceAccount
                    || !target
                        .spn
                        .as_deref()
                        .is_some_and(valid_service_principal_name)
                {
                    return Err(self.precondition(
                        attempt.technique,
                        "target phải có SPN service account dạng service/instance",
                    ));
                }
                if !target.service_ticket_available {
                    return Err(self.precondition(
                        attempt.technique,
                        "không có synthetic service ticket khả dụng",
                    ));
                }
            }
            AttackTechnique::AsRepRoasting => {
                if target.kind != AccountKind::User {
                    return Err(self.precondition(attempt.technique, "AS-REP target phải là user"));
                }
                if target.preauthentication_required {
                    return Err(
                        self.precondition(attempt.technique, "pre-authentication đang bắt buộc")
                    );
                }
            }
            _ => unreachable!("chỉ gọi cho roasting"),
        }
        let credential = self.required_credential(attempt)?;
        let expected_use = match attempt.technique {
            AttackTechnique::Kerberoasting => CredentialUse::ServiceTicketVerifier,
            AttackTechnique::AsRepRoasting => CredentialUse::AsRepVerifier,
            _ => unreachable!(),
        };
        self.validate_credential(attempt, credential, &attempt.target, expected_use)
    }

    fn required_credential(
        &self,
        attempt: &AttackAttempt,
    ) -> Result<&SyntheticCredential, AttackError> {
        let id = attempt
            .credential_id
            .as_ref()
            .ok_or_else(|| self.precondition(attempt.technique, "thiếu synthetic credential ID"))?;
        self.registered_credential(id)
    }

    fn validate_credential(
        &self,
        attempt: &AttackAttempt,
        credential: &SyntheticCredential,
        account: &str,
        intended_use: CredentialUse,
    ) -> Result<(), AttackError> {
        if !credential.usable
            || credential.account != account
            || credential.intended_use != intended_use
            || credential.artifact != SYNTHETIC_HASH
            || !credential.offline_verifier.starts_with("SYNTHETIC-")
        {
            return Err(self.precondition(
                attempt.technique,
                "credential sai owner/use, đã revoke hoặc không phải nhãn tổng hợp",
            ));
        }
        Ok(())
    }

    fn validate_delegation(&self, attempt: &AttackAttempt) -> Result<(), AttackError> {
        let target = &self.accounts[&attempt.target];
        if !matches!(
            target.kind,
            AccountKind::Computer | AccountKind::ServiceAccount
        ) {
            return Err(
                self.precondition(attempt.technique, "delegation target không phải resource")
            );
        }
        let kind = match attempt.technique {
            AttackTechnique::UnconstrainedDelegation => DelegationKind::Unconstrained,
            AttackTechnique::ConstrainedDelegation => DelegationKind::Constrained,
            AttackTechnique::ResourceBasedConstrainedDelegation => {
                DelegationKind::ResourceBasedConstrained
            }
            _ => unreachable!(),
        };
        let rule = self
            .delegations
            .iter()
            .find(|rule| rule.kind == kind && rule.principal == attempt.actor)
            .ok_or_else(|| self.precondition(attempt.technique, "không có delegation rule"))?;
        if rule.evidence_id != EVIDENCE_DELEGATION
            || !attempt.evidence_ids.contains(EVIDENCE_DELEGATION)
        {
            return Err(AttackError::WrongEvidence {
                expected: EVIDENCE_DELEGATION.to_string(),
            });
        }
        match kind {
            DelegationKind::Unconstrained => {
                if rule.resource.is_some() || !rule.allowed_targets.is_empty() {
                    return Err(
                        self.precondition(attempt.technique, "unconstrained rule malformed")
                    );
                }
            }
            DelegationKind::Constrained => {
                if rule.resource.is_some() || !rule.allowed_targets.contains(&attempt.target) {
                    return Err(self.precondition(
                        attempt.technique,
                        "target không nằm trong constrained allowlist",
                    ));
                }
            }
            DelegationKind::ResourceBasedConstrained => {
                if rule.resource.as_deref() != Some(attempt.target.as_str())
                    || !rule.allowed_targets.is_empty()
                {
                    return Err(self.precondition(
                        attempt.technique,
                        "RBCD rule không được cấu hình trên target resource",
                    ));
                }
            }
        }
        let ticket_id = attempt
            .ticket_id
            .as_ref()
            .ok_or_else(|| self.precondition(attempt.technique, "thiếu synthetic ticket"))?;
        let ticket = self.registered_ticket(ticket_id)?;
        if ticket.realm != "LAB.EXAMPLE"
            || ticket.client == attempt.actor
            || !self.accounts.contains_key(&ticket.client)
        {
            return Err(self.precondition(attempt.technique, "ticket realm/client không hợp lệ"));
        }
        match kind {
            DelegationKind::Unconstrained => {
                if ticket.kind != SyntheticTicketKind::Tgt || ticket.service != "krbtgt/lab.example"
                {
                    return Err(self.precondition(
                        attempt.technique,
                        "unconstrained cần TGT tổng hợp của identity khác",
                    ));
                }
            }
            DelegationKind::Constrained | DelegationKind::ResourceBasedConstrained => {
                if ticket.kind != SyntheticTicketKind::Service || ticket.service != attempt.actor {
                    return Err(self.precondition(
                        attempt.technique,
                        "constrained/RBCD cần service ticket gắn với principal",
                    ));
                }
            }
        }
        Ok(())
    }

    fn matching_grant<'a>(
        &'a self,
        attempt: &AttackAttempt,
        right: AclRight,
    ) -> Option<&'a AclGrant> {
        self.acl.grants.iter().find(|grant| {
            grant.actor == attempt.actor && grant.target == attempt.target && grant.right == right
        })
    }

    fn validate_acl(&self, attempt: &AttackAttempt) -> Result<(), AttackError> {
        let right = attempt.technique.acl_right().ok_or_else(|| {
            self.precondition(attempt.technique, "technique không phải ACL action")
        })?;
        if right == AclRight::ForceChangePassword
            && !matches!(
                self.accounts[&attempt.target].kind,
                AccountKind::User | AccountKind::ServiceAccount | AccountKind::Computer
            )
        {
            return Err(self.precondition(
                attempt.technique,
                "password reset target phải là user, service account hoặc computer",
            ));
        }
        let grant = self
            .matching_grant(attempt, right)
            .ok_or_else(|| self.precondition(attempt.technique, "không có exact ACL grant"))?;
        if grant.evidence_id != EVIDENCE_ACL || !attempt.evidence_ids.contains(EVIDENCE_ACL) {
            return Err(AttackError::WrongEvidence {
                expected: EVIDENCE_ACL.to_string(),
            });
        }
        Ok(())
    }

    fn validate_dcsync(&self, attempt: &AttackAttempt) -> Result<(), AttackError> {
        if self.accounts[&attempt.target].kind != AccountKind::Domain {
            return Err(self.precondition(attempt.technique, "DCSync target phải là domain root"));
        }
        for right in [
            AclRight::ReplicationGetChanges,
            AclRight::ReplicationGetChangesAll,
        ] {
            let grant = self.matching_grant(attempt, right).ok_or_else(|| {
                self.precondition(
                    attempt.technique,
                    "DCSync cần ĐỒNG THỜI hai replication rights",
                )
            })?;
            if grant.evidence_id != EVIDENCE_REPLICATION
                || !attempt.evidence_ids.contains(EVIDENCE_REPLICATION)
            {
                return Err(AttackError::WrongEvidence {
                    expected: EVIDENCE_REPLICATION.to_string(),
                });
            }
        }
        Ok(())
    }

    fn validate_identity_transition(&self, attempt: &AttackAttempt) -> Result<(), AttackError> {
        let target = &self.accounts[&attempt.target];
        if !matches!(
            target.kind,
            AccountKind::Computer | AccountKind::ServiceAccount
        ) {
            return Err(self.precondition(attempt.technique, "identity target không phải resource"));
        }
        let credential = self.required_credential(attempt)?;
        self.validate_credential(
            attempt,
            credential,
            &attempt.actor,
            CredentialUse::NtHashLabel,
        )
    }

    fn record(
        &mut self,
        attempt: &AttackAttempt,
        kind: TelemetryKind,
        outcome: EventOutcome,
        detail: &str,
    ) {
        let evidence = attempt
            .evidence_ids
            .iter()
            .filter_map(|id| self.evidence.get(id).cloned())
            .collect();
        self.events.push(DetectionEvent {
            sequence: self.events.len() as u64 + 1,
            synthetic_event_id: attempt.technique.synthetic_event_id(),
            kind,
            technique: attempt.technique,
            actor: attempt.actor.clone(),
            target: attempt.target.clone(),
            outcome,
            detail: detail.to_string(),
            evidence,
        });
    }

    fn reject<T>(&mut self, attempt: &AttackAttempt, error: AttackError) -> Result<T, AttackError> {
        let kind = if matches!(error, AttackError::OutOfScope { .. }) {
            TelemetryKind::ScopeEnforcement
        } else {
            attempt.technique.telemetry_kind()
        };
        self.record(attempt, kind, EventOutcome::Rejected, &error.to_string());
        Err(error)
    }

    fn make_finding(
        &self,
        attempt: &AttackAttempt,
        first_event: usize,
        outcome: FindingOutcome,
        synthetic_secret: Option<String>,
        state_transition: Option<IdentityStateTransition>,
    ) -> AttackFinding {
        let compromise = match outcome {
            FindingOutcome::RoastableMaterial | FindingOutcome::OfflineDictionaryMatch => {
                CompromiseStatus::NotEstablished
            }
            FindingOutcome::DelegationApproved
            | FindingOutcome::AclActionRolledBack
            | FindingOutcome::SyntheticSecretDisclosed
            | FindingOutcome::IdentityStateChanged
            | FindingOutcome::TicketStateChanged => CompromiseStatus::SyntheticStateOnly,
        };
        AttackFinding {
            technique: attempt.technique,
            actor: attempt.actor.clone(),
            target: attempt.target.clone(),
            outcome,
            compromise,
            evidence: attempt
                .evidence_ids
                .iter()
                .filter_map(|id| self.evidence.get(id).cloned())
                .collect(),
            detection: self.events[first_event..].to_vec(),
            remediation: self.remediation_for(attempt.technique),
            synthetic_secret,
            state_transition,
        }
    }

    /// Mọi thành công đều đi qua scope + evidence + technique preconditions trước.
    pub fn attempt(&mut self, attempt: &AttackAttempt) -> Result<AttackFinding, AttackError> {
        let first_event = self.events.len();
        if let Err(error) = self.validate_attempt(attempt) {
            return self.reject(attempt, error);
        }
        match attempt.technique {
            AttackTechnique::Kerberoasting | AttackTechnique::AsRepRoasting => {
                self.record(
                    attempt,
                    attempt.technique.telemetry_kind(),
                    EventOutcome::Observed,
                    "đã quan sát roastable material tổng hợp; chưa suy ra account compromise",
                );
                if attempt.dictionary.is_empty() {
                    return Ok(self.make_finding(
                        attempt,
                        first_event,
                        FindingOutcome::RoastableMaterial,
                        None,
                        None,
                    ));
                }
                let credential = self.required_credential(attempt)?;
                if !attempt
                    .dictionary
                    .iter()
                    .any(|candidate| candidate == &credential.offline_verifier)
                {
                    self.record(
                        attempt,
                        TelemetryKind::OfflineDictionary,
                        EventOutcome::Rejected,
                        "wrong synthetic candidate; offline check thất bại",
                    );
                    return Err(AttackError::WrongSyntheticCandidate);
                }
                self.record(
                    attempt,
                    TelemetryKind::OfflineDictionary,
                    EventOutcome::Matched,
                    "offline dictionary match trên verifier giả; vẫn chưa chứng minh account compromise",
                );
                Ok(self.make_finding(
                    attempt,
                    first_event,
                    FindingOutcome::OfflineDictionaryMatch,
                    None,
                    None,
                ))
            }
            AttackTechnique::UnconstrainedDelegation
            | AttackTechnique::ConstrainedDelegation
            | AttackTechnique::ResourceBasedConstrainedDelegation => {
                self.record(
                    attempt,
                    TelemetryKind::Delegation,
                    EventOutcome::Observed,
                    "delegation rule, ticket binding và target allowlist/resource đã khớp",
                );
                Ok(self.make_finding(
                    attempt,
                    first_event,
                    FindingOutcome::DelegationApproved,
                    None,
                    None,
                ))
            }
            AttackTechnique::GenericAll
            | AttackTechnique::WriteDacl
            | AttackTechnique::WriteOwner
            | AttackTechnique::ForceChangePassword => {
                let before = self.acl.clone();
                match attempt.technique {
                    AttackTechnique::GenericAll => {
                        self.acl.controlled_objects.insert(attempt.target.clone());
                    }
                    AttackTechnique::WriteDacl => {
                        self.acl.grants.insert(AclGrant {
                            actor: attempt.actor.clone(),
                            target: attempt.target.clone(),
                            right: AclRight::GenericAll,
                            evidence_id: EVIDENCE_ACL.to_string(),
                        });
                    }
                    AttackTechnique::WriteOwner => {
                        self.acl
                            .owners
                            .insert(attempt.target.clone(), attempt.actor.clone());
                    }
                    AttackTechnique::ForceChangePassword => {
                        self.acl
                            .password_reset_markers
                            .insert(attempt.target.clone());
                    }
                    _ => unreachable!(),
                }
                let after = self.acl.clone();
                if before == after {
                    return self.reject(
                        attempt,
                        AttackError::NoStateChange {
                            technique: attempt.technique,
                            target: attempt.target.clone(),
                        },
                    );
                }
                self.record(
                    attempt,
                    TelemetryKind::DirectoryMutation,
                    EventOutcome::Observed,
                    "ACL action tạo state change trong transaction; sẽ rollback ngay",
                );
                let mutation_id = format!("SYNTHETIC-ACL-{:03}", self.mutations.len() + 1);
                self.mutations.push(AclMutation {
                    id: mutation_id.clone(),
                    technique: attempt.technique,
                    actor: attempt.actor.clone(),
                    target: attempt.target.clone(),
                    before: before.clone(),
                    after,
                    rolled_back: false,
                });
                self.acl = before;
                if let Some(mutation) = self.mutations.last_mut() {
                    mutation.rolled_back = true;
                }
                self.cleanup.push(CleanupRecord {
                    mutation_id,
                    restored: true,
                    detail: "đã khôi phục nguyên vẹn grants/owners/control/reset markers"
                        .to_string(),
                });
                self.record(
                    attempt,
                    TelemetryKind::DirectoryMutation,
                    EventOutcome::RolledBack,
                    "cleanup record xác nhận ACL state đã được khôi phục",
                );
                Ok(self.make_finding(
                    attempt,
                    first_event,
                    FindingOutcome::AclActionRolledBack,
                    None,
                    None,
                ))
            }
            AttackTechnique::DcSync => {
                self.record(
                    attempt,
                    TelemetryKind::Replication,
                    EventOutcome::Observed,
                    "đủ hai replication rights; chỉ trả nhãn secret giả, không gọi replication RPC",
                );
                Ok(self.make_finding(
                    attempt,
                    first_event,
                    FindingOutcome::SyntheticSecretDisclosed,
                    Some(SYNTHETIC_HASH.to_string()),
                    None,
                ))
            }
            AttackTechnique::PassTheHash | AttackTechnique::OverPassTheHash => {
                let is_opth = attempt.technique == AttackTechnique::OverPassTheHash;
                self.record(
                    attempt,
                    attempt.technique.telemetry_kind(),
                    EventOutcome::Observed,
                    if is_opth {
                        "OPtH: chỉ tạo TGT label và typed ticket state"
                    } else {
                        "PtH: chỉ đổi typed identity state; không dump/inject/token manipulation"
                    },
                );
                let ticket = is_opth.then(|| SyntheticTicket {
                    id: "ticket-opth-synthetic".to_string(),
                    kind: SyntheticTicketKind::Tgt,
                    realm: "LAB.EXAMPLE".to_string(),
                    client: attempt.actor.clone(),
                    service: "krbtgt/lab.example".to_string(),
                    label: "SYNTHETIC-TICKET-OPTH".to_string(),
                    evidence_id: EVIDENCE_IDENTITY.to_string(),
                });
                let transition = IdentityStateTransition {
                    session: format!("SYNTHETIC-SESSION-{:03}", self.events.len()),
                    before: "SYNTHETIC-UNAUTHENTICATED".to_string(),
                    after: attempt.actor.clone(),
                    resource: attempt.target.clone(),
                    ticket,
                };
                self.sessions
                    .insert(transition.session.clone(), transition.clone());
                Ok(self.make_finding(
                    attempt,
                    first_event,
                    if is_opth {
                        FindingOutcome::TicketStateChanged
                    } else {
                        FindingOutcome::IdentityStateChanged
                    },
                    None,
                    Some(transition),
                ))
            }
        }
    }

    /// Nối path bằng cách execute lại từng attempt; adjacency một mình không đủ.
    pub fn compose_path(
        &mut self,
        goal: &str,
        attempts: &[AttackAttempt],
    ) -> Result<ComposedPath, AttackError> {
        if attempts.is_empty() {
            return Err(AttackError::EmptyPath);
        }
        if attempts.len() > MAX_PATH_STEPS {
            return Err(AttackError::PathTooLong {
                actual: attempts.len(),
                maximum: MAX_PATH_STEPS,
            });
        }
        let start = attempts[0].actor.clone();
        let mut current = start.clone();
        let mut visited = BTreeSet::from([start.clone()]);
        let mut findings = Vec::with_capacity(attempts.len());
        for (index, attempt) in attempts.iter().enumerate() {
            let step = index + 1;
            if attempt.actor != current {
                return Err(AttackError::PathDisconnected { step });
            }
            if !visited.insert(attempt.target.clone()) {
                return Err(AttackError::PathCycle {
                    step,
                    identity: attempt.target.clone(),
                });
            }
            let finding = self
                .attempt(attempt)
                .map_err(|error| AttackError::PathFailed {
                    step,
                    error: Box::new(error),
                })?;
            findings.push(finding);
            current = attempt.target.clone();
        }
        if current != goal {
            return Err(AttackError::GoalMismatch {
                expected: goal.to_string(),
                actual: current,
            });
        }
        let path = ComposedPath {
            start,
            goal: goal.to_string(),
            findings,
        };
        if path.validate() {
            Ok(path)
        } else {
            Err(AttackError::PathDisconnected { step: 0 })
        }
    }

    pub fn remediation_for(&self, technique: AttackTechnique) -> Remediation {
        let (actions, retest) = match technique {
            AttackTechnique::Kerberoasting => (
                vec![
                    "dùng gMSA/service secret dài, bỏ SPN stale và giảm quyền service account",
                    "ưu tiên AES và alert theo tập SPN/etype bất thường, không chỉ một 4769",
                ],
                "export fixture mới: service_ticket_available=false và cùng attempt phải fail",
            ),
            AttackTechnique::AsRepRoasting => (
                vec![
                    "bật Kerberos pre-authentication cho mọi account không có ngoại lệ được duyệt",
                    "theo dõi thay đổi policy và chuỗi AS-REP bất thường",
                ],
                "preauthentication_required=true; attempt cũ phải fail trước offline check",
            ),
            AttackTechnique::UnconstrainedDelegation => (
                vec![
                    "loại bỏ unconstrained delegation khỏi principal trong fixture",
                    "đặt identity nhạy cảm theo chính sách chống delegation và review tiering",
                ],
                "không còn rule Unconstrained; cùng TGT giả phải bị từ chối",
            ),
            AttackTechnique::ConstrainedDelegation => (
                vec![
                    "xóa rule hoặc thu nhỏ allowed_targets theo nhu cầu nghiệp vụ",
                    "review cả principal, protocol transition và resource được phép",
                ],
                "target không còn trong allowlist; cùng service ticket giả phải fail",
            ),
            AttackTechnique::ResourceBasedConstrainedDelegation => (
                vec![
                    "gỡ principal khỏi RBCD rule trên resource và review ai được sửa resource ACL",
                    "theo dõi thay đổi delegation cùng machine/account quản trị",
                ],
                "resource không còn rule cho principal; attempt cũ phải fail",
            ),
            AttackTechnique::GenericAll => (
                vec![
                    "gỡ ACE GenericAll không cần thiết và khôi phục owner/DACL theo change record",
                ],
                "exact grant GenericAll biến mất; attempt fail và không còn controlled marker",
            ),
            AttackTechnique::WriteDacl => (
                vec!["gỡ WriteDacl, review ACE vừa thêm và giới hạn quyền quản trị DACL"],
                "exact grant WriteDacl biến mất; DACL trước/sau cleanup phải bằng nhau",
            ),
            AttackTechnique::WriteOwner => (
                vec![
                    "gỡ WriteOwner, khôi phục owner hợp lệ và review quyền Take Ownership trong lab",
                ],
                "owner trở về baseline và attempt WriteOwner cũ phải fail",
            ),
            AttackTechnique::ForceChangePassword => (
                vec![
                    "gỡ quyền reset không cần thiết; rotate secret lab nếu có thay đổi được phê duyệt",
                    "review reset events và phiên liên quan thay vì chỉ xóa marker",
                ],
                "grant ForceChangePassword biến mất; reset marker không tồn tại sau rollback",
            ),
            AttackTechnique::DcSync => (
                vec![
                    "gỡ một trong hai replication right không cần thiết và review cả hai trên domain root",
                    "giám sát replication access theo principal/DC được phê duyệt",
                ],
                "thiếu bất kỳ replication right nào cũng làm DCSync fail; không secret output",
            ),
            AttackTechnique::PassTheHash => (
                vec![
                    "revoke synthetic credential, áp dụng tiering/least privilege và giảm reuse credential trong lab thật",
                    "correlate logon type/account/source với baseline; không kết luận từ một event đơn lẻ",
                ],
                "credential usable=false; cùng attempt phải fail và không tạo session mới",
            ),
            AttackTechnique::OverPassTheHash => (
                vec![
                    "revoke synthetic credential và review TGT/logon telemetry theo cùng identity",
                    "áp dụng credential hygiene, tiering và vòng đời secret cho tài khoản dịch vụ",
                ],
                "credential usable=false; không TGT label hay session state mới",
            ),
        };
        Remediation {
            technique,
            actions: actions.into_iter().map(str::to_string).collect(),
            retest: retest.to_string(),
        }
    }

    /// Thay đổi fixture như một control phòng thủ; không phát lệnh cho OS/directory.
    pub fn apply_remediation(&mut self, technique: AttackTechnique) -> Remediation {
        match technique {
            AttackTechnique::Kerberoasting => {
                for account in self.accounts.values_mut() {
                    if account.spn.is_some() {
                        account.service_ticket_available = false;
                    }
                }
                for credential in self.credentials.values_mut() {
                    if credential.intended_use == CredentialUse::ServiceTicketVerifier {
                        credential.usable = false;
                    }
                }
            }
            AttackTechnique::AsRepRoasting => {
                for account in self.accounts.values_mut() {
                    account.preauthentication_required = true;
                }
                for credential in self.credentials.values_mut() {
                    if credential.intended_use == CredentialUse::AsRepVerifier {
                        credential.usable = false;
                    }
                }
            }
            AttackTechnique::UnconstrainedDelegation
            | AttackTechnique::ConstrainedDelegation
            | AttackTechnique::ResourceBasedConstrainedDelegation => {
                let kind = match technique {
                    AttackTechnique::UnconstrainedDelegation => DelegationKind::Unconstrained,
                    AttackTechnique::ConstrainedDelegation => DelegationKind::Constrained,
                    AttackTechnique::ResourceBasedConstrainedDelegation => {
                        DelegationKind::ResourceBasedConstrained
                    }
                    _ => unreachable!(),
                };
                self.delegations.retain(|rule| rule.kind != kind);
            }
            AttackTechnique::GenericAll
            | AttackTechnique::WriteDacl
            | AttackTechnique::WriteOwner
            | AttackTechnique::ForceChangePassword => {
                let right = technique.acl_right().expect("ACL technique");
                self.acl.grants.retain(|grant| grant.right != right);
            }
            AttackTechnique::DcSync => {
                self.acl.grants.retain(|grant| {
                    !matches!(
                        grant.right,
                        AclRight::ReplicationGetChanges | AclRight::ReplicationGetChangesAll
                    )
                });
            }
            AttackTechnique::PassTheHash | AttackTechnique::OverPassTheHash => {
                for credential in self.credentials.values_mut() {
                    if credential.intended_use == CredentialUse::NtHashLabel {
                        credential.usable = false;
                    }
                }
                self.sessions.clear();
            }
        }
        self.remediation_for(technique)
    }

    /// Chỉ remediation cho baseline đã thành công; retest phải fail ở precondition.
    pub fn remediate_and_retest(
        &mut self,
        attempt: &AttackAttempt,
    ) -> Result<Remediation, AttackError> {
        self.attempt(attempt)?;
        let remediation = self.apply_remediation(attempt.technique);
        match self.attempt(attempt) {
            Err(AttackError::MissingPrecondition { .. }) => Ok(remediation),
            Err(error) => Err(error),
            Ok(_) => Err(AttackError::RemediationFailed(attempt.technique)),
        }
    }
}

fn valid_service_principal_name(spn: &str) -> bool {
    let Some((service, instance)) = spn.split_once('/') else {
        return false;
    };
    !service.is_empty()
        && !instance.is_empty()
        && !instance.contains('/')
        && !spn.chars().any(char::is_whitespace)
}

fn account(
    id: &str,
    kind: AccountKind,
    spn: Option<&str>,
    preauthentication_required: bool,
    service_ticket_available: bool,
) -> Account {
    Account {
        id: id.to_string(),
        kind,
        spn: spn.map(str::to_string),
        preauthentication_required,
        service_ticket_available,
    }
}

fn credential(
    id: &str,
    account: &str,
    intended_use: CredentialUse,
    offline_verifier: &str,
    evidence_id: &str,
) -> SyntheticCredential {
    SyntheticCredential {
        id: id.to_string(),
        account: account.to_string(),
        intended_use,
        artifact: SYNTHETIC_HASH.to_string(),
        offline_verifier: offline_verifier.to_string(),
        usable: true,
        evidence_id: evidence_id.to_string(),
    }
}

fn grant(actor: &str, target: &str, right: AclRight) -> AclGrant {
    AclGrant {
        actor: actor.to_string(),
        target: target.to_string(),
        right,
        evidence_id: EVIDENCE_ACL.to_string(),
    }
}

// ============================================================================
// 6. DEMO TÁI LẬP
// ============================================================================

fn main() -> Result<(), Box<dyn Error>> {
    let mut lab = MockADAttacks::standard();
    println!("MockADAttacks — 100% synthetic, std-only, không network/OS/RPC");

    let material = lab.attempt(&lab.attempt_template(AttackTechnique::Kerberoasting))?;
    println!(
        "Kerberoasting material: {:?}; compromise={:?}",
        material.outcome, material.compromise
    );
    let matched = lab.attempt(
        &lab.attempt_template(AttackTechnique::Kerberoasting)
            .with_dictionary(&["SYNTHETIC-CANDIDATE-WRONG", "SYNTHETIC-CANDIDATE-WEB"]),
    )?;
    println!(
        "Offline dictionary: {:?}; compromise={:?}",
        matched.outcome, matched.compromise
    );
    let wrong = lab.attempt(
        &lab.attempt_template(AttackTechnique::AsRepRoasting)
            .with_dictionary(&["SYNTHETIC-CANDIDATE-WRONG"]),
    );
    println!("AS-REP wrong candidate: {}", wrong.expect_err("phải fail"));

    let constrained = lab.attempt(&lab.attempt_template(AttackTechnique::ConstrainedDelegation))?;
    println!(
        "Constrained allowlist: {} -> {} ({:?})",
        constrained.actor, constrained.target, constrained.outcome
    );
    let invalid_target = AttackAttempt::new(
        AttackTechnique::ConstrainedDelegation,
        "svc-front@lab.example",
        "ws01.lab.example",
    )
    .with_ticket("ticket-service-bob")
    .with_evidence(EVIDENCE_DELEGATION)
    .with_evidence(EVIDENCE_TICKET);
    println!(
        "Constrained invalid target: {}",
        lab.attempt(&invalid_target)
            .expect_err("target ngoài allowlist")
    );

    let before = lab.acl.clone();
    let acl = lab.attempt(&lab.attempt_template(AttackTechnique::WriteDacl))?;
    let mutation = lab.acl_mutations().last().expect("ACL mutation record");
    println!(
        "WriteDacl: {:?}; mutation_changed={}; restored={}",
        acl.outcome,
        mutation.before != mutation.after,
        lab.acl == before && lab.cleanup_records().last().is_some_and(|row| row.restored)
    );

    let dcsync = lab.attempt(&lab.attempt_template(AttackTechnique::DcSync))?;
    println!(
        "DCSync: {:?}; output={}",
        dcsync.outcome,
        dcsync.synthetic_secret.expect("chỉ là nhãn giả")
    );
    let opth = lab.attempt(&lab.attempt_template(AttackTechnique::OverPassTheHash))?;
    let state = opth.state_transition.expect("typed OPtH state");
    println!(
        "OPtH state: {} -> {}; ticket_kind={:?}",
        state.before,
        state.after,
        state.ticket.map(|ticket| ticket.kind)
    );

    let mut path_lab = MockADAttacks::standard();
    let roast = path_lab
        .attempt_template(AttackTechnique::Kerberoasting)
        .with_dictionary(&["SYNTHETIC-CANDIDATE-WEB"]);
    let pth = path_lab.attempt_template(AttackTechnique::PassTheHash);
    let path = path_lab.compose_path("server01.lab.example", &[roast, pth])?;
    println!(
        "Composed path: {} -> {}; validated_steps={}",
        path.start,
        path.goal,
        path.findings.len()
    );
    println!(
        "Telemetry events: {}; ACL cleanup records: {}",
        lab.events().len(),
        lab.cleanup_records().len()
    );
    Ok(())
}

// ============================================================================
// 7. TESTS: POSITIVE, NEGATIVE, BOUNDARY, MALFORMED, ROLLBACK, DETERMINISM
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_TECHNIQUES: [AttackTechnique; 12] = [
        AttackTechnique::Kerberoasting,
        AttackTechnique::AsRepRoasting,
        AttackTechnique::UnconstrainedDelegation,
        AttackTechnique::ConstrainedDelegation,
        AttackTechnique::ResourceBasedConstrainedDelegation,
        AttackTechnique::GenericAll,
        AttackTechnique::WriteDacl,
        AttackTechnique::WriteOwner,
        AttackTechnique::ForceChangePassword,
        AttackTechnique::DcSync,
        AttackTechnique::PassTheHash,
        AttackTechnique::OverPassTheHash,
    ];

    #[test]
    fn fixture_is_sorted_synthetic_and_complete() {
        let lab = MockADAttacks::standard();
        assert_eq!(lab.accounts.len(), 10);
        assert_eq!(lab.credentials.len(), 3);
        assert_eq!(lab.delegations.len(), 3);
        assert!(lab.evidence.values().all(Evidence::is_valid));
        assert!(
            lab.credentials
                .values()
                .all(|credential| credential.artifact == SYNTHETIC_HASH)
        );
        assert!(lab.scope.targets.contains("server01.lab.example"));
        assert!(!lab.scope.targets.contains("outside.example"));
    }

    #[test]
    fn engagement_id_technique_actor_and_target_scope_are_enforced() {
        let mut lab = MockADAttacks::standard();
        let mut attempt = lab.attempt_template(AttackTechnique::Kerberoasting);
        attempt.engagement = "ENG-OTHER".to_string();
        assert!(matches!(
            lab.attempt(&attempt),
            Err(AttackError::OutOfScope { .. })
        ));
        let mut attempt = lab.attempt_template(AttackTechnique::Kerberoasting);
        attempt.target = "outside.example".to_string();
        assert!(matches!(
            lab.attempt(&attempt),
            Err(AttackError::OutOfScope { .. })
        ));
        assert_eq!(lab.events().len(), 2);
        assert!(
            lab.events()
                .iter()
                .all(|event| event.kind == TelemetryKind::ScopeEnforcement)
        );
    }

    #[test]
    fn unknown_in_scope_identity_is_an_error_not_a_ghost() {
        let mut lab = MockADAttacks::standard();
        lab.scope.targets.insert("ghost.lab.example".to_string());
        let mut attempt = lab.attempt_template(AttackTechnique::Kerberoasting);
        attempt.target = "ghost.lab.example".to_string();
        assert_eq!(
            lab.attempt(&attempt),
            Err(AttackError::UnknownIdentity(
                "ghost.lab.example".to_string()
            ))
        );
    }

    #[test]
    fn missing_unknown_and_unrelated_evidence_are_rejected() {
        for attempt in [
            AttackAttempt::new(
                AttackTechnique::Kerberoasting,
                "analyst@lab.example",
                "svc-web@lab.example",
            )
            .with_credential("cred-kerb-web"),
            AttackAttempt::new(
                AttackTechnique::Kerberoasting,
                "analyst@lab.example",
                "svc-web@lab.example",
            )
            .with_credential("cred-kerb-web")
            .with_evidence("ev-missing"),
            AttackAttempt::new(
                AttackTechnique::Kerberoasting,
                "analyst@lab.example",
                "svc-web@lab.example",
            )
            .with_credential("cred-kerb-web")
            .with_evidence(EVIDENCE_ACL),
        ] {
            let mut lab = MockADAttacks::standard();
            assert!(lab.attempt(&attempt).is_err());
            assert!(lab.acl_mutations().is_empty());
            assert!(lab.sessions().is_empty());
        }
    }

    #[test]
    fn malformed_registered_evidence_is_not_accepted() {
        let mut lab = MockADAttacks::standard();
        lab.evidence
            .get_mut(EVIDENCE_KERBEROS)
            .expect("evidence")
            .source = "real-production-log".to_string();
        let attempt = lab.attempt_template(AttackTechnique::Kerberoasting);
        assert_eq!(
            lab.attempt(&attempt),
            Err(AttackError::UnknownEvidence(EVIDENCE_KERBEROS.to_string()))
        );
    }

    #[test]
    fn roastable_material_does_not_establish_account_compromise() {
        let mut lab = MockADAttacks::standard();
        for technique in [
            AttackTechnique::Kerberoasting,
            AttackTechnique::AsRepRoasting,
        ] {
            let finding = lab
                .attempt(&lab.attempt_template(technique))
                .expect("material");
            assert_eq!(finding.outcome, FindingOutcome::RoastableMaterial);
            assert_eq!(finding.compromise, CompromiseStatus::NotEstablished);
            assert_eq!(finding.synthetic_secret, None);
        }
    }

    #[test]
    fn kerberoasting_offline_match_and_wrong_candidate_are_distinct() {
        let mut lab = MockADAttacks::standard();
        let matched = lab.attempt(
            &lab.attempt_template(AttackTechnique::Kerberoasting)
                .with_dictionary(&["SYNTHETIC-CANDIDATE-NO", "SYNTHETIC-CANDIDATE-WEB"]),
        );
        let finding = matched.expect("correct synthetic candidate");
        assert_eq!(finding.outcome, FindingOutcome::OfflineDictionaryMatch);
        assert_eq!(finding.compromise, CompromiseStatus::NotEstablished);
        assert_eq!(finding.detection.len(), 2);
        assert_eq!(finding.detection[1].outcome, EventOutcome::Matched);

        let wrong = lab.attempt(
            &lab.attempt_template(AttackTechnique::Kerberoasting)
                .with_dictionary(&["SYNTHETIC-CANDIDATE-NO"]),
        );
        assert_eq!(wrong, Err(AttackError::WrongSyntheticCandidate));
        assert_eq!(
            lab.events().last().map(|event| event.outcome),
            Some(EventOutcome::Rejected)
        );
    }

    #[test]
    fn kerberoasting_requires_service_spn_and_available_ticket() {
        let mut lab = MockADAttacks::standard();
        let mut no_spn = lab.attempt_template(AttackTechnique::Kerberoasting);
        no_spn.target = "bob@lab.example".to_string();
        assert!(matches!(
            lab.attempt(&no_spn),
            Err(AttackError::MissingPrecondition { .. })
        ));
        let valid = lab.attempt_template(AttackTechnique::Kerberoasting);
        lab.accounts
            .get_mut("svc-web@lab.example")
            .expect("service")
            .service_ticket_available = false;
        assert!(lab.attempt(&valid).is_err());
    }

    #[test]
    fn asrep_requires_disabled_preauthentication_and_matching_candidate() {
        let mut lab = MockADAttacks::standard();
        let attempt = lab.attempt_template(AttackTechnique::AsRepRoasting);
        lab.accounts
            .get_mut("legacy@lab.example")
            .expect("legacy")
            .preauthentication_required = true;
        assert!(matches!(
            lab.attempt(&attempt),
            Err(AttackError::MissingPrecondition { .. })
        ));
        lab.accounts
            .get_mut("legacy@lab.example")
            .expect("legacy")
            .preauthentication_required = false;
        assert_eq!(
            lab.attempt(&attempt.clone().with_dictionary(&["SYNTHETIC-WRONG"])),
            Err(AttackError::WrongSyntheticCandidate)
        );
        assert!(
            lab.attempt(&attempt.with_dictionary(&["SYNTHETIC-CANDIDATE-LEGACY"]))
                .is_ok()
        );
    }

    #[test]
    fn unconstrained_delegation_requires_other_client_tgt() {
        let mut lab = MockADAttacks::standard();
        let attempt = lab.attempt_template(AttackTechnique::UnconstrainedDelegation);
        assert!(lab.attempt(&attempt).is_ok());
        let wrong_ticket = attempt.with_ticket("ticket-service-bob");
        assert!(matches!(
            lab.attempt(&wrong_ticket),
            Err(AttackError::MissingPrecondition { .. })
        ));
    }

    #[test]
    fn constrained_delegation_allows_only_explicit_target() {
        let mut lab = MockADAttacks::standard();
        let allowed = lab.attempt_template(AttackTechnique::ConstrainedDelegation);
        assert_eq!(
            lab.attempt(&allowed).expect("allowed target").outcome,
            FindingOutcome::DelegationApproved
        );
        let invalid = AttackAttempt::new(
            AttackTechnique::ConstrainedDelegation,
            "svc-front@lab.example",
            "ws01.lab.example",
        )
        .with_ticket("ticket-service-bob")
        .with_evidence(EVIDENCE_DELEGATION)
        .with_evidence(EVIDENCE_TICKET);
        let error = lab.attempt(&invalid).expect_err("target must be rejected");
        assert!(error.to_string().contains("allowlist"));
    }

    #[test]
    fn rbcd_rule_must_live_on_requested_resource() {
        let mut lab = MockADAttacks::standard();
        assert!(
            lab.attempt(&lab.attempt_template(AttackTechnique::ResourceBasedConstrainedDelegation))
                .is_ok()
        );
        lab.delegations
            .retain(|rule| rule.kind != DelegationKind::ResourceBasedConstrained);
        assert!(
            lab.attempt(&lab.attempt_template(AttackTechnique::ResourceBasedConstrainedDelegation))
                .is_err()
        );
    }

    #[test]
    fn constrained_ticket_must_be_bound_to_delegating_principal() {
        let mut lab = MockADAttacks::standard();
        lab.tickets
            .get_mut("ticket-service-bob")
            .expect("ticket")
            .service = "svc-web@lab.example".to_string();
        assert!(
            lab.attempt(&lab.attempt_template(AttackTechnique::ConstrainedDelegation))
                .is_err()
        );
    }

    #[test]
    fn generic_all_mutation_is_real_then_fully_rolled_back() {
        let mut lab = MockADAttacks::standard();
        let before = lab.acl.clone();
        let finding = lab
            .attempt(&lab.attempt_template(AttackTechnique::GenericAll))
            .expect("grant exists");
        let mutation = lab.acl_mutations().last().expect("mutation");
        assert!(
            mutation
                .after
                .controlled_objects
                .contains("bob@lab.example")
        );
        assert!(
            !mutation
                .before
                .controlled_objects
                .contains("bob@lab.example")
        );
        assert!(mutation.rolled_back);
        assert_eq!(lab.acl, before);
        assert_eq!(finding.outcome, FindingOutcome::AclActionRolledBack);
        assert!(lab.cleanup_records().last().expect("cleanup").restored);
    }

    #[test]
    fn writedacl_mutation_adds_then_restores_exact_grant_set() {
        let mut lab = MockADAttacks::standard();
        lab.acl
            .grants
            .retain(|grant| grant.right != AclRight::GenericAll);
        let before = lab.acl.clone();
        lab.attempt(&lab.attempt_template(AttackTechnique::WriteDacl))
            .expect("WriteDacl does not need GenericAll");
        let mutation = lab.acl_mutations().last().expect("mutation");
        assert!(mutation.after.grants.len() > mutation.before.grants.len());
        assert_eq!(lab.acl.grants, before.grants);
        assert_eq!(lab.acl, before);
    }

    #[test]
    fn writeowner_restores_original_owner() {
        let mut lab = MockADAttacks::standard();
        let original = lab.acl.owners["bob@lab.example"].clone();
        lab.attempt(&lab.attempt_template(AttackTechnique::WriteOwner))
            .expect("WriteOwner");
        let mutation = lab.acl_mutations().last().expect("mutation");
        assert_eq!(
            mutation.after.owners["bob@lab.example"],
            "helpdesk@lab.example"
        );
        assert_eq!(lab.acl.owners["bob@lab.example"], original);
        assert_eq!(lab.acl, mutation.before);
    }

    #[test]
    fn force_change_password_leaves_no_reset_marker_after_cleanup() {
        let mut lab = MockADAttacks::standard();
        let before = lab.acl.clone();
        lab.attempt(&lab.attempt_template(AttackTechnique::ForceChangePassword))
            .expect("reset grant");
        assert!(
            lab.acl_mutations()
                .last()
                .expect("mutation")
                .after
                .password_reset_markers
                .contains("bob@lab.example")
        );
        assert!(lab.acl.password_reset_markers.is_empty());
        assert_eq!(lab.acl, before);
        assert_eq!(lab.cleanup_records().len(), 1);
    }

    #[test]
    fn acl_action_without_exact_grant_does_not_mutate() {
        let mut lab = MockADAttacks::standard();
        lab.acl
            .grants
            .retain(|grant| grant.right != AclRight::WriteDacl);
        let before = lab.acl.clone();
        assert!(
            lab.attempt(&lab.attempt_template(AttackTechnique::WriteDacl))
                .is_err()
        );
        assert_eq!(lab.acl, before);
        assert!(lab.acl_mutations().is_empty());
        assert!(lab.cleanup_records().is_empty());
    }

    #[test]
    fn dcsync_returns_only_synthetic_secret_when_both_rights_exist() {
        let mut lab = MockADAttacks::standard();
        let before = lab.acl.clone();
        let finding = lab
            .attempt(&lab.attempt_template(AttackTechnique::DcSync))
            .expect("both rights");
        assert_eq!(finding.synthetic_secret.as_deref(), Some(SYNTHETIC_HASH));
        assert_eq!(finding.outcome, FindingOutcome::SyntheticSecretDisclosed);
        assert_eq!(lab.acl, before);
        assert!(lab.acl_mutations().is_empty());
    }

    #[test]
    fn dcsync_fails_when_either_replication_right_is_missing() {
        for missing in [
            AclRight::ReplicationGetChanges,
            AclRight::ReplicationGetChangesAll,
        ] {
            let mut lab = MockADAttacks::standard();
            lab.acl.grants.retain(|grant| grant.right != missing);
            assert!(matches!(
                lab.attempt(&lab.attempt_template(AttackTechnique::DcSync)),
                Err(AttackError::MissingPrecondition { .. })
            ));
            assert!(lab.sessions().is_empty());
            assert!(lab.acl_mutations().is_empty());
        }
    }

    #[test]
    fn pth_is_only_an_identity_state_transition_without_ticket() {
        let mut lab = MockADAttacks::standard();
        let finding = lab
            .attempt(&lab.attempt_template(AttackTechnique::PassTheHash))
            .expect("typed PtH");
        let state = finding.state_transition.expect("state");
        assert_eq!(state.before, "SYNTHETIC-UNAUTHENTICATED");
        assert_eq!(state.after, "svc-web@lab.example");
        assert_eq!(state.resource, "server01.lab.example");
        assert_eq!(state.ticket, None);
        assert_eq!(finding.compromise, CompromiseStatus::SyntheticStateOnly);
        assert_eq!(lab.sessions().len(), 1);
    }

    #[test]
    fn opth_creates_only_a_synthetic_tgt_state() {
        let mut lab = MockADAttacks::standard();
        let finding = lab
            .attempt(&lab.attempt_template(AttackTechnique::OverPassTheHash))
            .expect("typed OPtH");
        let ticket = finding
            .state_transition
            .and_then(|state| state.ticket)
            .expect("TGT label");
        assert_eq!(ticket.kind, SyntheticTicketKind::Tgt);
        assert_eq!(ticket.client, "svc-web@lab.example");
        assert!(ticket.label.starts_with("SYNTHETIC-TICKET-"));
        assert!(!lab.tickets.contains_key(&ticket.id));
    }

    #[test]
    fn identity_transition_rejects_wrong_owner_or_unsynthetic_artifact() {
        let mut lab = MockADAttacks::standard();
        lab.credentials
            .get_mut("cred-pth-web")
            .expect("credential")
            .account = "bob@lab.example".to_string();
        assert!(
            lab.attempt(&lab.attempt_template(AttackTechnique::PassTheHash))
                .is_err()
        );
        let mut lab = MockADAttacks::standard();
        lab.credentials
            .get_mut("cred-pth-web")
            .expect("credential")
            .artifact = "0024-not-synthetic".to_string();
        assert!(
            lab.attempt(&lab.attempt_template(AttackTechnique::OverPassTheHash))
                .is_err()
        );
        assert!(lab.sessions().is_empty());
    }

    #[test]
    fn composed_path_rechecks_every_edge_and_preserves_evidence() {
        let mut lab = MockADAttacks::standard();
        let roast = lab
            .attempt_template(AttackTechnique::Kerberoasting)
            .with_dictionary(&["SYNTHETIC-CANDIDATE-WEB"]);
        let pth = lab.attempt_template(AttackTechnique::PassTheHash);
        let path = lab
            .compose_path("server01.lab.example", &[roast, pth])
            .expect("both edges independently valid");
        assert!(path.validate());
        assert_eq!(path.findings.len(), 2);
        assert_eq!(path.start, "analyst@lab.example");
        assert_eq!(path.goal, "server01.lab.example");
        assert!(
            path.findings
                .iter()
                .all(|finding| !finding.evidence.is_empty())
        );
        // Credential PtH là precondition độc lập, không phải output tự suy ra từ roast.
        assert_eq!(
            path.findings[0].compromise,
            CompromiseStatus::NotEstablished
        );
    }

    #[test]
    fn adjacency_cannot_hide_missing_precondition_on_second_edge() {
        let mut lab = MockADAttacks::standard();
        lab.credentials.remove("cred-pth-web");
        let roast = lab
            .attempt_template(AttackTechnique::Kerberoasting)
            .with_dictionary(&["SYNTHETIC-CANDIDATE-WEB"]);
        let pth = lab.attempt_template(AttackTechnique::PassTheHash);
        let result = lab.compose_path("server01.lab.example", &[roast, pth]);
        assert!(matches!(
            result,
            Err(AttackError::PathFailed { step: 2, .. })
        ));
    }

    #[test]
    fn composed_path_rejects_disconnected_and_wrong_goal() {
        let mut lab = MockADAttacks::standard();
        let roast = lab.attempt_template(AttackTechnique::Kerberoasting);
        let mut disconnected = lab.attempt_template(AttackTechnique::PassTheHash);
        disconnected.actor = "bob@lab.example".to_string();
        assert_eq!(
            lab.compose_path("server01.lab.example", &[roast.clone(), disconnected]),
            Err(AttackError::PathDisconnected { step: 2 })
        );
        let pth = lab.attempt_template(AttackTechnique::PassTheHash);
        assert!(matches!(
            lab.compose_path("ws01.lab.example", &[roast, pth]),
            Err(AttackError::GoalMismatch { .. })
        ));
    }

    #[test]
    fn composed_path_rejects_cycles_before_executing_cycle_edge() {
        let mut lab = MockADAttacks::standard();
        let first = lab.attempt_template(AttackTechnique::Kerberoasting);
        let second = AttackAttempt::new(
            AttackTechnique::PassTheHash,
            "svc-web@lab.example",
            "analyst@lab.example",
        )
        .with_credential("cred-pth-web")
        .with_evidence(EVIDENCE_IDENTITY);
        assert_eq!(
            lab.compose_path("analyst@lab.example", &[first, second]),
            Err(AttackError::PathCycle {
                step: 2,
                identity: "analyst@lab.example".to_string()
            })
        );
        assert!(lab.sessions().is_empty());
    }

    #[test]
    fn path_boundaries_empty_long_and_unknown_evidence_are_safe() {
        let mut lab = MockADAttacks::standard();
        assert_eq!(
            lab.compose_path("lab.example", &[]),
            Err(AttackError::EmptyPath)
        );
        let long = vec![lab.attempt_template(AttackTechnique::Kerberoasting); 9];
        assert_eq!(
            lab.compose_path("server01.lab.example", &long),
            Err(AttackError::PathTooLong {
                actual: 9,
                maximum: MAX_PATH_STEPS
            })
        );
        let mut unknown = lab.attempt_template(AttackTechnique::Kerberoasting);
        unknown.evidence_ids.clear();
        unknown.evidence_ids.insert("ev-unknown".to_string());
        assert!(matches!(
            lab.compose_path("svc-web@lab.example", &[unknown]),
            Err(AttackError::PathFailed { step: 1, .. })
        ));
    }

    #[test]
    fn every_success_has_typed_detection_and_concrete_remediation() {
        for technique in ALL_TECHNIQUES {
            let mut lab = MockADAttacks::standard();
            let finding = lab
                .attempt(&lab.attempt_template(technique))
                .expect("standard positive fixture");
            assert!(!finding.detection.is_empty(), "{technique:?}");
            assert_eq!(finding.remediation.technique, technique);
            assert!(!finding.remediation.actions.is_empty());
            assert!(
                !finding.remediation.retest.trim().is_empty(),
                "{technique:?}"
            );
            assert!(finding.detection.iter().all(|event| event.sequence > 0));
            assert!(
                finding
                    .detection
                    .iter()
                    .all(|event| !event.evidence.is_empty())
            );
        }
    }

    #[test]
    fn remediation_retest_blocks_every_standard_technique() {
        for technique in ALL_TECHNIQUES {
            let mut lab = MockADAttacks::standard();
            let attempt = lab.attempt_template(technique);
            assert!(lab.attempt(&attempt).is_ok(), "baseline {technique:?}");
            let remediation = lab
                .remediate_and_retest(&attempt)
                .expect("control must block technique");
            assert_eq!(remediation.technique, technique);
            assert!(
                matches!(
                    lab.events().last().map(|event| event.outcome),
                    Some(EventOutcome::Rejected)
                ),
                "{technique:?}"
            );
        }
    }

    #[test]
    fn remediation_does_not_count_scope_or_evidence_failure_as_success() {
        let mut lab = MockADAttacks::standard();
        let mut attempt = lab.attempt_template(AttackTechnique::Kerberoasting);
        attempt.engagement = "ENG-OTHER".to_string();
        assert!(matches!(
            lab.remediate_and_retest(&attempt),
            Err(AttackError::OutOfScope { .. })
        ));
    }

    #[test]
    fn caller_inserted_or_rebound_credentials_are_not_fixture_artifacts() {
        let mut lab = MockADAttacks::standard();
        lab.credentials.insert(
            "cred-caller".to_string(),
            credential(
                "cred-caller",
                "svc-web@lab.example",
                CredentialUse::ServiceTicketVerifier,
                "SYNTHETIC-CANDIDATE-CALLER",
                EVIDENCE_KERBEROS,
            ),
        );
        let inserted = lab
            .attempt_template(AttackTechnique::Kerberoasting)
            .with_credential("cred-caller")
            .with_dictionary(&["SYNTHETIC-CANDIDATE-CALLER"]);
        assert!(lab.attempt(&inserted).is_err());

        let mut lab = MockADAttacks::standard();
        lab.credentials
            .get_mut("cred-kerb-web")
            .expect("credential")
            .offline_verifier = "SYNTHETIC-CANDIDATE-REBOUND".to_string();
        let rebound = lab
            .attempt_template(AttackTechnique::Kerberoasting)
            .with_dictionary(&["SYNTHETIC-CANDIDATE-REBOUND"]);
        assert!(lab.attempt(&rebound).is_err());
    }

    #[test]
    fn caller_inserted_tickets_or_replaced_evidence_are_not_trusted() {
        let mut lab = MockADAttacks::standard();
        lab.tickets.insert(
            "ticket-caller".to_string(),
            SyntheticTicket {
                id: "ticket-caller".to_string(),
                kind: SyntheticTicketKind::Service,
                realm: "LAB.EXAMPLE".to_string(),
                client: "bob@lab.example".to_string(),
                service: "svc-front@lab.example".to_string(),
                label: "SYNTHETIC-TICKET-CALLER".to_string(),
                evidence_id: EVIDENCE_TICKET.to_string(),
            },
        );
        let attempt = lab
            .attempt_template(AttackTechnique::ConstrainedDelegation)
            .with_ticket("ticket-caller");
        assert!(lab.attempt(&attempt).is_err());

        let mut lab = MockADAttacks::standard();
        lab.evidence.insert(
            EVIDENCE_KERBEROS.to_string(),
            Evidence::new(
                EVIDENCE_KERBEROS,
                "synthetic-caller-claim",
                "caller tự tuyên bố đã quan sát ticket",
            ),
        );
        assert!(
            lab.attempt(&lab.attempt_template(AttackTechnique::Kerberoasting))
                .is_err()
        );
    }

    #[test]
    fn malformed_spn_is_not_roastable_material() {
        for spn in [
            "",
            "HTTP",
            "/web.lab.example",
            "HTTP/",
            "HTTP /web.lab.example",
            "HTTP/web lab.example",
            "HTTP/web.lab.example/extra",
        ] {
            let mut lab = MockADAttacks::standard();
            lab.accounts
                .get_mut("svc-web@lab.example")
                .expect("service")
                .spn = Some(spn.to_string());
            assert!(
                lab.attempt(&lab.attempt_template(AttackTechnique::Kerberoasting))
                    .is_err(),
                "SPN phải bị từ chối: {spn:?}"
            );
        }
    }

    #[test]
    fn acl_noop_is_not_a_successful_mutation_or_cleanup() {
        for technique in [
            AttackTechnique::GenericAll,
            AttackTechnique::WriteDacl,
            AttackTechnique::WriteOwner,
            AttackTechnique::ForceChangePassword,
        ] {
            let mut lab = MockADAttacks::standard();
            let target = "bob@lab.example";
            match technique {
                AttackTechnique::GenericAll => {
                    lab.acl.controlled_objects.insert(target.to_string());
                }
                AttackTechnique::WriteDacl => {
                    lab.acl.grants.insert(AclGrant {
                        actor: "helpdesk@lab.example".to_string(),
                        target: target.to_string(),
                        right: AclRight::GenericAll,
                        evidence_id: EVIDENCE_ACL.to_string(),
                    });
                }
                AttackTechnique::WriteOwner => {
                    lab.acl
                        .owners
                        .insert(target.to_string(), "helpdesk@lab.example".to_string());
                }
                AttackTechnique::ForceChangePassword => {
                    lab.acl.password_reset_markers.insert(target.to_string());
                }
                _ => unreachable!(),
            }
            let before = lab.acl.clone();
            assert_eq!(
                lab.attempt(&lab.attempt_template(technique)),
                Err(AttackError::NoStateChange {
                    technique,
                    target: target.to_string(),
                })
            );
            assert_eq!(lab.acl, before);
            assert!(lab.acl_mutations().is_empty());
            assert!(lab.cleanup_records().is_empty());
        }
    }

    #[test]
    fn invalid_remediation_baseline_does_not_change_controls() {
        let mut lab = MockADAttacks::standard();
        let mut attempt = lab.attempt_template(AttackTechnique::Kerberoasting);
        attempt.engagement = "ENG-OTHER".to_string();
        let accounts = lab.accounts.clone();
        let credentials = lab.credentials.clone();
        assert!(matches!(
            lab.remediate_and_retest(&attempt),
            Err(AttackError::OutOfScope { .. })
        ));
        assert_eq!(lab.accounts, accounts);
        assert_eq!(lab.credentials, credentials);

        let mut lab = MockADAttacks::standard();
        lab.credentials.remove("cred-pth-web");
        let credentials = lab.credentials.clone();
        let attempt = lab.attempt_template(AttackTechnique::PassTheHash);
        assert_eq!(
            lab.remediate_and_retest(&attempt),
            Err(AttackError::UnknownCredential("cred-pth-web".to_string()))
        );
        assert_eq!(lab.credentials, credentials);
        assert!(lab.sessions().is_empty());
    }

    #[test]
    fn configuration_evidence_cannot_be_rebound_or_diluted() {
        let mut lab = MockADAttacks::standard();
        lab.delegations
            .iter_mut()
            .find(|rule| rule.kind == DelegationKind::Constrained)
            .expect("rule")
            .evidence_id = EVIDENCE_ACL.to_string();
        let attempt = lab
            .attempt_template(AttackTechnique::ConstrainedDelegation)
            .with_evidence(EVIDENCE_ACL);
        assert_eq!(
            lab.attempt(&attempt),
            Err(AttackError::UnrelatedEvidence(EVIDENCE_ACL.to_string()))
        );
        let attempt = lab.attempt_template(AttackTechnique::ConstrainedDelegation);
        assert_eq!(
            lab.attempt(&attempt),
            Err(AttackError::WrongEvidence {
                expected: EVIDENCE_DELEGATION.to_string()
            })
        );

        let mut lab = MockADAttacks::standard();
        let mut grant = lab
            .acl
            .grants
            .iter()
            .find(|grant| grant.right == AclRight::ReplicationGetChanges)
            .expect("grant")
            .clone();
        lab.acl.grants.remove(&grant);
        grant.evidence_id = EVIDENCE_KERBEROS.to_string();
        lab.acl.grants.insert(grant);
        let attempt = lab
            .attempt_template(AttackTechnique::DcSync)
            .with_evidence(EVIDENCE_KERBEROS);
        assert_eq!(
            lab.attempt(&attempt),
            Err(AttackError::UnrelatedEvidence(
                EVIDENCE_KERBEROS.to_string()
            ))
        );
        let attempt = lab.attempt_template(AttackTechnique::DcSync);
        assert_eq!(
            lab.attempt(&attempt),
            Err(AttackError::WrongEvidence {
                expected: EVIDENCE_REPLICATION.to_string()
            })
        );

        let mut lab = MockADAttacks::standard();
        let attempt = lab
            .attempt_template(AttackTechnique::Kerberoasting)
            .with_evidence(EVIDENCE_ACL);
        assert_eq!(
            lab.attempt(&attempt),
            Err(AttackError::UnrelatedEvidence(EVIDENCE_ACL.to_string()))
        );
    }

    #[test]
    fn password_reset_rejects_domain_root() {
        let mut lab = MockADAttacks::standard();
        lab.acl.grants.insert(AclGrant {
            actor: "helpdesk@lab.example".to_string(),
            target: "lab.example".to_string(),
            right: AclRight::ForceChangePassword,
            evidence_id: EVIDENCE_ACL.to_string(),
        });
        let attempt = AttackAttempt::new(
            AttackTechnique::ForceChangePassword,
            "helpdesk@lab.example",
            "lab.example",
        )
        .with_evidence(EVIDENCE_ACL);
        assert!(matches!(
            lab.attempt(&attempt),
            Err(AttackError::MissingPrecondition { .. })
        ));
        assert!(lab.acl.password_reset_markers.is_empty());
        assert!(lab.acl_mutations().is_empty());
        assert!(lab.cleanup_records().is_empty());
    }

    #[test]
    fn identity_sessions_are_unique_per_successful_attempt() {
        let mut lab = MockADAttacks::standard();
        lab.attempt(&lab.attempt_template(AttackTechnique::PassTheHash))
            .expect("PtH server01");
        lab.attempt(&lab.attempt_template(AttackTechnique::OverPassTheHash))
            .expect("OPtH server01");
        let other_resource = AttackAttempt::new(
            AttackTechnique::OverPassTheHash,
            "svc-web@lab.example",
            "ws01.lab.example",
        )
        .with_credential("cred-pth-web")
        .with_evidence(EVIDENCE_IDENTITY);
        lab.attempt(&other_resource).expect("OPtH ws01");
        assert_eq!(lab.sessions().len(), 3);
        let resources = lab
            .sessions()
            .values()
            .map(|transition| transition.resource.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            resources,
            BTreeSet::from(["server01.lab.example", "ws01.lab.example"])
        );
    }

    #[test]
    fn repeated_runs_are_deterministic_including_events_and_cleanup() {
        #[derive(Debug, PartialEq, Eq)]
        struct RunOutput {
            findings: Vec<Result<AttackFinding, AttackError>>,
            events: Vec<DetectionEvent>,
            mutations: Vec<AclMutation>,
            cleanup: Vec<CleanupRecord>,
        }

        fn run() -> RunOutput {
            let mut lab = MockADAttacks::standard();
            let attempts = [
                lab.attempt_template(AttackTechnique::Kerberoasting)
                    .with_dictionary(&["SYNTHETIC-CANDIDATE-WEB"]),
                lab.attempt_template(AttackTechnique::ConstrainedDelegation),
                lab.attempt_template(AttackTechnique::WriteOwner),
                lab.attempt_template(AttackTechnique::DcSync),
                lab.attempt_template(AttackTechnique::OverPassTheHash),
                lab.attempt_template(AttackTechnique::AsRepRoasting)
                    .with_dictionary(&["SYNTHETIC-WRONG"]),
            ];
            let findings = attempts
                .iter()
                .map(|attempt| lab.attempt(attempt))
                .collect();
            RunOutput {
                findings,
                events: lab.events.clone(),
                mutations: lab.mutations.clone(),
                cleanup: lab.cleanup.clone(),
            }
        }
        assert_eq!(run(), run());
    }

    #[test]
    fn public_composed_path_validation_rejects_fabricated_success() {
        let mut lab = MockADAttacks::standard();
        let attempt = lab.attempt_template(AttackTechnique::Kerberoasting);
        let finding = lab.attempt(&attempt).expect("finding");
        let disconnected = ComposedPath {
            start: "bob@lab.example".to_string(),
            goal: "svc-web@lab.example".to_string(),
            findings: vec![finding.clone()],
        };
        assert!(!disconnected.validate());
        let cyclic = ComposedPath {
            start: "analyst@lab.example".to_string(),
            goal: "svc-web@lab.example".to_string(),
            findings: vec![finding.clone(), finding],
        };
        assert!(!cyclic.validate());
    }
}
