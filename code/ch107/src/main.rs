#![forbid(unsafe_code)]
#![allow(dead_code)]

//! Active Directory fundamentals trên một snapshot hoàn toàn tổng hợp.
//! Không có LDAP, RPC, SMB, Kerberos network call hoặc lệnh hệ điều hành.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

// ============================================================================
// 1. ĐỊNH DANH, DN VÀ OBJECT
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(String);

impl ObjectId {
    pub fn new(value: &str) -> Result<Self, DirectoryError> {
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(DirectoryError::InvalidObjectId(value.to_string()));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DnKey {
    CommonName,
    OrganizationalUnit,
    DomainComponent,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DnComponent {
    pub key: DnKey,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DistinguishedName {
    pub components: Vec<DnComponent>,
}

impl DistinguishedName {
    /// Parser subset CN/OU/DC, không hỗ trợ RDN nhiều giá trị hoặc escape của RFC 4514.
    pub fn parse(input: &str) -> Result<Self, DirectoryError> {
        if input.is_empty() || input.trim() != input {
            return Err(DirectoryError::InvalidDistinguishedName(input.to_string()));
        }
        let raw: Vec<&str> = input.split(',').collect();
        let mut components = Vec::with_capacity(raw.len());
        let mut previous_rank = 0;
        let mut domain_components = 0;
        for (index, part) in raw.iter().enumerate() {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| DirectoryError::InvalidDistinguishedName(input.to_string()))?;
            if value.is_empty()
                || value.trim() != value
                || value.contains(['\\', '+', '"', '<', '>', ';', '\0'])
            {
                return Err(DirectoryError::InvalidDistinguishedName(input.to_string()));
            }
            let (key, rank) = match key.to_ascii_uppercase().as_str() {
                "CN" => (DnKey::CommonName, 0),
                "OU" => (DnKey::OrganizationalUnit, 1),
                "DC" => (DnKey::DomainComponent, 2),
                _ => return Err(DirectoryError::InvalidDistinguishedName(input.to_string())),
            };
            if (index == 0 && key == DnKey::OrganizationalUnit && rank < previous_rank)
                || rank < previous_rank
                || (key == DnKey::CommonName && index != 0)
            {
                return Err(DirectoryError::InvalidDistinguishedName(input.to_string()));
            }
            if key == DnKey::DomainComponent {
                domain_components += 1;
            }
            previous_rank = rank;
            components.push(DnComponent {
                key,
                value: value.to_string(),
            });
        }
        if domain_components == 0 {
            return Err(DirectoryError::InvalidDistinguishedName(input.to_string()));
        }
        Ok(Self { components })
    }

    pub fn value(&self, key: DnKey) -> Option<&str> {
        self.components
            .iter()
            .find(|component| component.key == key)
            .map(|component| component.value.as_str())
    }

    pub fn domain(&self) -> String {
        self.components
            .iter()
            .filter(|component| component.key == DnKey::DomainComponent)
            .map(|component| component.value.as_str())
            .collect::<Vec<_>>()
            .join(".")
            .to_ascii_lowercase()
    }

    pub fn parent(&self) -> Option<Self> {
        (self.components.len() > 1).then(|| Self {
            components: self.components[1..].to_vec(),
        })
    }

    /// So sánh toàn bộ RDN theo cùng quy tắc với `within`.
    pub fn eq_ignore_ascii_case(&self, other: &Self) -> bool {
        self.components.len() == other.components.len() && self.within(other)
    }

    /// Kiểm tra suffix DN theo ASCII case-insensitive, không suy diễn quyền từ cây OU.
    pub fn within(&self, container: &Self) -> bool {
        self.components.len() >= container.components.len()
            && self.components[self.components.len() - container.components.len()..]
                .iter()
                .zip(&container.components)
                .all(|(left, right)| {
                    left.key == right.key && left.value.eq_ignore_ascii_case(&right.value)
                })
    }
}

impl fmt::Display for DistinguishedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self
            .components
            .iter()
            .map(|component| {
                let key = match component.key {
                    DnKey::CommonName => "CN",
                    DnKey::OrganizationalUnit => "OU",
                    DnKey::DomainComponent => "DC",
                };
                format!("{key}={}", component.value)
            })
            .collect::<Vec<_>>()
            .join(",");
        f.write_str(&text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectKind {
    Domain,
    OrganizationalUnit,
    User,
    Group,
    Computer,
    ServiceAccount,
    DomainController,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupScope {
    DomainLocal,
    Global,
    Universal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub source: String,
    pub detail: String,
}

impl Evidence {
    pub fn new(source: &str, detail: &str) -> Self {
        Self {
            source: source.to_string(),
            detail: detail.to_string(),
        }
    }

    fn is_valid(&self) -> bool {
        !self.source.trim().is_empty() && !self.detail.trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdObject {
    pub id: ObjectId,
    pub kind: ObjectKind,
    pub name: String,
    pub dn: DistinguishedName,
    pub owner: ObjectId,
    pub guid: Option<String>,
    pub group_scope: Option<GroupScope>,
    pub spns: Vec<String>,
    pub site: Option<String>,
}

impl AdObject {
    pub fn new(
        id: ObjectId,
        kind: ObjectKind,
        name: &str,
        dn: DistinguishedName,
        owner: ObjectId,
    ) -> Self {
        Self {
            id,
            kind,
            name: name.to_string(),
            dn,
            owner,
            guid: None,
            group_scope: None,
            spns: Vec::new(),
            site: None,
        }
    }

    pub fn attributes(&self, snapshot: &DirectorySnapshot) -> BTreeMap<String, Vec<String>> {
        let class = match self.kind {
            ObjectKind::User | ObjectKind::ServiceAccount => "user",
            ObjectKind::Group => "group",
            ObjectKind::Computer | ObjectKind::DomainController => "computer",
            ObjectKind::Domain => "domain",
            ObjectKind::OrganizationalUnit => "organizationalUnit",
        };
        let mut attributes = BTreeMap::from([
            ("objectclass".to_string(), vec![class.to_string()]),
            ("samaccountname".to_string(), vec![self.name.clone()]),
            ("distinguishedname".to_string(), vec![self.dn.to_string()]),
        ]);
        if !self.spns.is_empty() {
            attributes.insert("serviceprincipalname".to_string(), self.spns.clone());
        }
        let groups = snapshot
            .memberships
            .iter()
            .filter(|membership| membership.member == self.id)
            .filter_map(|membership| snapshot.object(&membership.group))
            .map(|group| group.dn.to_string())
            .collect::<Vec<_>>();
        if !groups.is_empty() {
            attributes.insert("memberof".to_string(), groups);
        }
        attributes
    }
}

// ============================================================================
// 2. GROUP, TRUST, GPO VÀ ACL
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMembership {
    pub member: ObjectId,
    pub group: ObjectId,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustDirection {
    /// source tin target: identity phía target có thể xác thực vào source.
    Outbound,
    /// target tin source: identity phía source có thể xác thực vào target.
    Inbound,
    Bidirectional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trust {
    pub source_domain: String,
    pub target_domain: String,
    pub direction: TrustDirection,
    pub transitive: bool,
    pub evidence: Evidence,
}

impl Trust {
    pub fn permits_authentication(&self, identity_domain: &str, resource_domain: &str) -> bool {
        let outbound = self.source_domain.eq_ignore_ascii_case(resource_domain)
            && self.target_domain.eq_ignore_ascii_case(identity_domain)
            && matches!(
                self.direction,
                TrustDirection::Outbound | TrustDirection::Bidirectional
            );
        let inbound = self.source_domain.eq_ignore_ascii_case(identity_domain)
            && self.target_domain.eq_ignore_ascii_case(resource_domain)
            && matches!(
                self.direction,
                TrustDirection::Inbound | TrustDirection::Bidirectional
            );
        outbound || inbound
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpoLink {
    pub ou: DistinguishedName,
    pub enabled: bool,
    pub enforced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gpo {
    pub id: ObjectId,
    pub name: String,
    pub links: Vec<GpoLink>,
    /// Snapshot đã resolve security filtering thành object ID rõ ràng.
    pub security_filter: Vec<ObjectId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessRight {
    GenericRead,
    GenericAll,
    WriteDacl,
    WriteOwner,
    ForceChangePassword,
    ReplicationGetChanges,
    ReplicationGetChangesAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AceEffect {
    Allow,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AclEntry {
    pub principal: ObjectId,
    pub target: ObjectId,
    pub right: AccessRight,
    pub effect: AceEffect,
    /// `true` nghĩa là snapshot đã nhận ACE này từ container cha.
    pub inherited: bool,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessStatus {
    Allowed,
    Denied,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessDecision {
    pub status: AccessStatus,
    pub evidence: Vec<Evidence>,
}

// ============================================================================
// 3. LDAP FILTER SUBSET VÀ PAGED ENUMERATION
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LdapFilter {
    Present(String),
    Equality { attribute: String, value: String },
    And(Vec<LdapFilter>),
    Or(Vec<LdapFilter>),
    Not(Box<LdapFilter>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LdapError {
    Empty,
    InvalidSyntax(usize),
    InvalidAttribute(String),
    InvalidEscape(usize),
    UnsupportedSubstring,
    TrailingCharacters(usize),
    EmptyPageSize,
    UnknownBase,
}

impl fmt::Display for LdapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LDAP subset không hợp lệ: {self:?}")
    }
}

impl Error for LdapError {}

impl LdapFilter {
    pub fn parse(input: &str) -> Result<Self, LdapError> {
        if input.is_empty() {
            return Err(LdapError::Empty);
        }
        let mut parser = FilterParser {
            bytes: input.as_bytes(),
            cursor: 0,
        };
        let filter = parser.filter()?;
        if parser.cursor != parser.bytes.len() {
            return Err(LdapError::TrailingCharacters(parser.cursor));
        }
        Ok(filter)
    }

    /// Matching tổng hợp: tên attribute và giá trị ASCII case-insensitive.
    /// Schema LDAP thật quyết định matching rule; không mang giả định này ra mọi directory.
    pub fn matches(&self, attributes: &BTreeMap<String, Vec<String>>) -> bool {
        let values = |attribute: &str| {
            attributes
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(attribute))
                .map(|(_, values)| values.as_slice())
                .unwrap_or(&[])
        };
        match self {
            Self::Present(attribute) => !values(attribute).is_empty(),
            Self::Equality { attribute, value } => values(attribute)
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(value)),
            Self::And(children) => children.iter().all(|child| child.matches(attributes)),
            Self::Or(children) => children.iter().any(|child| child.matches(attributes)),
            Self::Not(child) => !child.matches(attributes),
        }
    }
}

struct FilterParser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl FilterParser<'_> {
    fn filter(&mut self) -> Result<LdapFilter, LdapError> {
        self.expect(b'(')?;
        match self.peek() {
            Some(b'&') | Some(b'|') => {
                let is_and = self.peek() == Some(b'&');
                self.cursor += 1;
                let mut children = Vec::new();
                while self.peek() == Some(b'(') {
                    children.push(self.filter()?);
                }
                if children.is_empty() {
                    return Err(LdapError::InvalidSyntax(self.cursor));
                }
                self.expect(b')')?;
                Ok(if is_and {
                    LdapFilter::And(children)
                } else {
                    LdapFilter::Or(children)
                })
            }
            Some(b'!') => {
                self.cursor += 1;
                let child = self.filter()?;
                self.expect(b')')?;
                Ok(LdapFilter::Not(Box::new(child)))
            }
            _ => self.item(),
        }
    }

    fn item(&mut self) -> Result<LdapFilter, LdapError> {
        let start = self.cursor;
        let mut raw = Vec::new();
        while let Some(byte) = self.peek() {
            if byte == b')' {
                break;
            }
            if byte == b'(' {
                return Err(LdapError::InvalidSyntax(self.cursor));
            }
            if byte == b'\\' {
                raw.push(byte);
                self.cursor += 1;
                for _ in 0..2 {
                    let digit = self.peek().ok_or(LdapError::InvalidEscape(self.cursor))?;
                    if !digit.is_ascii_hexdigit() {
                        return Err(LdapError::InvalidEscape(self.cursor));
                    }
                    raw.push(digit);
                    self.cursor += 1;
                }
            } else {
                raw.push(byte);
                self.cursor += 1;
            }
        }
        if self.peek() != Some(b')') || raw.is_empty() {
            return Err(LdapError::InvalidSyntax(start));
        }
        self.cursor += 1;
        let equals = raw
            .iter()
            .position(|byte| *byte == b'=')
            .ok_or(LdapError::InvalidSyntax(start))?;
        let attribute =
            std::str::from_utf8(&raw[..equals]).map_err(|_| LdapError::InvalidSyntax(start))?;
        if attribute.is_empty()
            || !attribute
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b';')
        {
            return Err(LdapError::InvalidAttribute(attribute.to_string()));
        }
        let encoded = &raw[equals + 1..];
        if encoded == b"*" {
            return Ok(LdapFilter::Present(attribute.to_ascii_lowercase()));
        }
        if encoded.contains(&b'*') {
            return Err(LdapError::UnsupportedSubstring);
        }
        let value = decode_ldap_value(encoded, start + equals + 1)?;
        Ok(LdapFilter::Equality {
            attribute: attribute.to_ascii_lowercase(),
            value,
        })
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.cursor).copied()
    }

    fn expect(&mut self, expected: u8) -> Result<(), LdapError> {
        if self.peek() != Some(expected) {
            return Err(LdapError::InvalidSyntax(self.cursor));
        }
        self.cursor += 1;
        Ok(())
    }
}

fn decode_ldap_value(bytes: &[u8], offset: usize) -> Result<String, LdapError> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            if cursor + 2 >= bytes.len() {
                return Err(LdapError::InvalidEscape(offset + cursor));
            }
            let hex = std::str::from_utf8(&bytes[cursor + 1..cursor + 3])
                .map_err(|_| LdapError::InvalidEscape(offset + cursor))?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| LdapError::InvalidEscape(offset + cursor))?;
            if byte == 0 {
                return Err(LdapError::InvalidEscape(offset + cursor));
            }
            decoded.push(byte);
            cursor += 3;
        } else {
            decoded.push(bytes[cursor]);
            cursor += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| LdapError::InvalidSyntax(offset))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    Base,
    OneLevel,
    Subtree,
}

pub struct PagedEnumeration<'a> {
    items: Vec<&'a AdObject>,
    page_size: usize,
    offset: usize,
}

impl<'a> Iterator for PagedEnumeration<'a> {
    type Item = Vec<&'a AdObject>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.items.len() {
            return None;
        }
        let end = (self.offset + self.page_size).min(self.items.len());
        let page = self.items[self.offset..end].to_vec();
        self.offset = end;
        Some(page)
    }
}

// ============================================================================
// 4. SNAPSHOT VÀ VALIDATION
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryError {
    InvalidObjectId(String),
    InvalidDistinguishedName(String),
    DuplicateObject(ObjectId),
    DuplicateDistinguishedName(String),
    MissingReference {
        context: &'static str,
        from: ObjectId,
        to: ObjectId,
    },
    InvalidEvidence,
    InvalidLdap(LdapError),
}

impl fmt::Display for DirectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "snapshot AD tổng hợp không hợp lệ: {self:?}")
    }
}

impl Error for DirectoryError {}

impl From<LdapError> for DirectoryError {
    fn from(value: LdapError) -> Self {
        Self::InvalidLdap(value)
    }
}

#[derive(Debug, Clone)]
pub struct DirectorySnapshot {
    pub objects: Vec<AdObject>,
    pub memberships: Vec<GroupMembership>,
    pub trusts: Vec<Trust>,
    pub gpos: Vec<Gpo>,
    pub acls: Vec<AclEntry>,
    pub blocked_inheritance_ous: BTreeSet<DistinguishedName>,
}

impl DirectorySnapshot {
    pub fn validated(mut self) -> Result<Self, DirectoryError> {
        self.objects.sort_by(|left, right| left.id.cmp(&right.id));
        self.memberships
            .sort_by(|left, right| (&left.member, &left.group).cmp(&(&right.member, &right.group)));
        self.acls.sort_by(|left, right| {
            (&left.principal, &left.target, left.right).cmp(&(
                &right.principal,
                &right.target,
                right.right,
            ))
        });
        self.gpos.sort_by(|left, right| left.id.cmp(&right.id));
        let mut ids = BTreeSet::new();
        let mut dns = BTreeSet::new();
        for object in &self.objects {
            if object.name.trim().is_empty() || !ids.insert(object.id.clone()) {
                return Err(DirectoryError::DuplicateObject(object.id.clone()));
            }
            if !dns.insert(object.dn.to_string().to_ascii_lowercase()) {
                return Err(DirectoryError::DuplicateDistinguishedName(
                    object.dn.to_string(),
                ));
            }
        }
        for object in &self.objects {
            self.require_reference("object owner", &object.id, &object.owner)?;
        }
        for membership in &self.memberships {
            if !membership.evidence.is_valid() {
                return Err(DirectoryError::InvalidEvidence);
            }
            self.require_reference("membership member", &membership.group, &membership.member)?;
            self.require_reference("membership group", &membership.member, &membership.group)?;
            if self.object(&membership.group).map(|object| object.kind) != Some(ObjectKind::Group) {
                return Err(DirectoryError::MissingReference {
                    context: "membership target is not a group",
                    from: membership.member.clone(),
                    to: membership.group.clone(),
                });
            }
        }
        for acl in &self.acls {
            if !acl.evidence.is_valid() {
                return Err(DirectoryError::InvalidEvidence);
            }
            self.require_reference("ACL principal", &acl.target, &acl.principal)?;
            self.require_reference("ACL target", &acl.principal, &acl.target)?;
        }
        for gpo in &self.gpos {
            if gpo.name.trim().is_empty() {
                return Err(DirectoryError::InvalidEvidence);
            }
            for principal in &gpo.security_filter {
                self.require_reference("GPO security filter", &gpo.id, principal)?;
            }
        }
        for trust in &self.trusts {
            if !trust.evidence.is_valid()
                || trust.source_domain.is_empty()
                || trust.target_domain.is_empty()
                || trust
                    .source_domain
                    .eq_ignore_ascii_case(&trust.target_domain)
            {
                return Err(DirectoryError::InvalidEvidence);
            }
        }
        Ok(self)
    }

    fn require_reference(
        &self,
        context: &'static str,
        from: &ObjectId,
        to: &ObjectId,
    ) -> Result<(), DirectoryError> {
        if self.object(to).is_none() {
            return Err(DirectoryError::MissingReference {
                context,
                from: from.clone(),
                to: to.clone(),
            });
        }
        Ok(())
    }

    pub fn object(&self, id: &ObjectId) -> Option<&AdObject> {
        self.objects
            .binary_search_by(|object| object.id.cmp(id))
            .ok()
            .map(|index| &self.objects[index])
    }

    pub fn search(
        &self,
        base: &DistinguishedName,
        scope: SearchScope,
        filter: &LdapFilter,
    ) -> Result<Vec<&AdObject>, DirectoryError> {
        if !self
            .objects
            .iter()
            .any(|object| object.dn.eq_ignore_ascii_case(base))
        {
            return Err(LdapError::UnknownBase.into());
        }
        Ok(self
            .objects
            .iter()
            .filter(|object| match scope {
                SearchScope::Base => object.dn.eq_ignore_ascii_case(base),
                SearchScope::OneLevel => object
                    .dn
                    .parent()
                    .as_ref()
                    .is_some_and(|parent| parent.eq_ignore_ascii_case(base)),
                SearchScope::Subtree => object.dn.within(base),
            })
            .filter(|object| filter.matches(&object.attributes(self)))
            .collect())
    }

    pub fn paged_search(
        &self,
        base: &DistinguishedName,
        scope: SearchScope,
        filter: &LdapFilter,
        page_size: usize,
    ) -> Result<PagedEnumeration<'_>, DirectoryError> {
        if page_size == 0 {
            return Err(LdapError::EmptyPageSize.into());
        }
        Ok(PagedEnumeration {
            items: self.search(base, scope, filter)?,
            page_size,
            offset: 0,
        })
    }

    /// Trả mọi group trực tiếp và gián tiếp; visited chặn chu trình nesting.
    pub fn effective_groups(&self, member: &ObjectId) -> BTreeSet<ObjectId> {
        let mut adjacency: BTreeMap<&ObjectId, Vec<&ObjectId>> = BTreeMap::new();
        for membership in &self.memberships {
            adjacency
                .entry(&membership.member)
                .or_default()
                .push(&membership.group);
        }
        let mut visited = BTreeSet::from([member.clone()]);
        let mut stack = vec![member.clone()];
        let mut groups = BTreeSet::new();
        while let Some(current) = stack.pop() {
            if let Some(parents) = adjacency.get(&current) {
                for group in parents {
                    if visited.insert((*group).clone()) {
                        groups.insert((*group).clone());
                        stack.push((*group).clone());
                    }
                }
            }
        }
        groups.remove(member);
        groups
    }

    pub fn is_member_of(&self, member: &ObjectId, group: &ObjectId) -> bool {
        self.effective_groups(member).contains(group)
    }

    /// Deny thắng allow trong subset; thiếu ACE là Unknown, không phải Allow.
    pub fn access_decision(
        &self,
        principal: &ObjectId,
        target: &ObjectId,
        right: AccessRight,
    ) -> AccessDecision {
        let mut principals = self.effective_groups(principal);
        principals.insert(principal.clone());
        let matching = self
            .acls
            .iter()
            .filter(|acl| {
                acl.target == *target && acl.right == right && principals.contains(&acl.principal)
            })
            .collect::<Vec<_>>();
        let denies = matching
            .iter()
            .filter(|acl| acl.effect == AceEffect::Deny)
            .map(|acl| acl.evidence.clone())
            .collect::<Vec<_>>();
        if !denies.is_empty() {
            return AccessDecision {
                status: AccessStatus::Denied,
                evidence: denies,
            };
        }
        let allows = matching
            .iter()
            .filter(|acl| acl.effect == AceEffect::Allow)
            .map(|acl| acl.evidence.clone())
            .collect::<Vec<_>>();
        AccessDecision {
            status: if allows.is_empty() {
                AccessStatus::Unknown
            } else {
                AccessStatus::Allowed
            },
            evidence: allows,
        }
    }

    pub fn spn_inventory(&self) -> Vec<SpnRecord> {
        self.objects
            .iter()
            .flat_map(|object| {
                object.spns.iter().map(|spn| SpnRecord {
                    object_id: object.id.clone(),
                    account: object.name.clone(),
                    spn: spn.clone(),
                })
            })
            .collect()
    }

    /// GPO hiệu lực trên snapshot: link/enabled/filter + block inheritance/enforced.
    pub fn effective_gpos(&self, object: &ObjectId) -> Vec<&Gpo> {
        let Some(object) = self.object(object) else {
            return Vec::new();
        };
        let Some(location) = object.dn.parent() else {
            return Vec::new();
        };
        self.gpos
            .iter()
            .filter(|gpo| {
                gpo.security_filter.is_empty() || gpo.security_filter.contains(&object.id)
            })
            .filter(|gpo| {
                gpo.links.iter().any(|link| {
                    if !link.enabled || !location.within(&link.ou) {
                        return false;
                    }
                    let blocked = self.blocked_inheritance_ous.iter().any(|ou| {
                        location.within(ou) && ou.components.len() > link.ou.components.len()
                    });
                    !blocked || link.enforced
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpnRecord {
    pub object_id: ObjectId,
    pub account: String,
    pub spn: String,
}

// ============================================================================
// 5. TYPED GRAPH VÀ DFS CÓ PROVENANCE
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdNode {
    pub id: ObjectId,
    pub kind: ObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeKind {
    MemberOf,
    Owns,
    GenericAll,
    WriteDacl,
    WriteOwner,
    ForceChangePassword,
}

impl EdgeKind {
    fn from_right(right: AccessRight) -> Option<Self> {
        match right {
            AccessRight::GenericAll => Some(Self::GenericAll),
            AccessRight::WriteDacl => Some(Self::WriteDacl),
            AccessRight::WriteOwner => Some(Self::WriteOwner),
            AccessRight::ForceChangePassword => Some(Self::ForceChangePassword),
            AccessRight::GenericRead
            | AccessRight::ReplicationGetChanges
            | AccessRight::ReplicationGetChangesAll => None,
        }
    }

    fn access_right(self) -> Option<AccessRight> {
        match self {
            Self::GenericAll => Some(AccessRight::GenericAll),
            Self::WriteDacl => Some(AccessRight::WriteDacl),
            Self::WriteOwner => Some(AccessRight::WriteOwner),
            Self::ForceChangePassword => Some(AccessRight::ForceChangePassword),
            Self::MemberOf | Self::Owns => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdEdge {
    pub from: ObjectId,
    pub to: ObjectId,
    pub kind: EdgeKind,
    pub evidence: Evidence,
}

#[derive(Debug, Clone)]
pub struct AdGraph {
    pub nodes: BTreeMap<ObjectId, AdNode>,
    pub edges: Vec<AdEdge>,
    /// Giữ deny gốc để path qua group không lách được deny của starting principal.
    acl_entries: Vec<AclEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackPath {
    pub start: ObjectId,
    pub goal: ObjectId,
    pub edges: Vec<AdEdge>,
}

impl AttackPath {
    pub fn validate(&self, graph: &AdGraph) -> bool {
        if !graph.nodes.contains_key(&self.start) || !graph.nodes.contains_key(&self.goal) {
            return false;
        }
        let principals = graph.effective_principals(&self.start);
        let mut current = &self.start;
        for edge in &self.edges {
            if edge.from != *current
                || !graph.nodes.contains_key(&edge.from)
                || !graph.nodes.contains_key(&edge.to)
                || !edge.evidence.is_valid()
                || !graph.edges.contains(edge)
                || !graph.acl_edge_permitted(&principals, edge)
            {
                return false;
            }
            current = &edge.to;
        }
        current == &self.goal
            && (self.start == self.goal || !self.edges.is_empty())
            && (self.start != self.goal || self.edges.is_empty())
    }

    pub fn explain(&self) -> String {
        if self.edges.is_empty() {
            return format!("{} đã là goal; không cần edge.", self.start);
        }
        self.edges
            .iter()
            .map(|edge| {
                format!(
                    "{} --{:?}--> {} [nguồn: {}]",
                    edge.from, edge.kind, edge.to, edge.evidence.source
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl AdGraph {
    pub fn from_snapshot(snapshot: &DirectorySnapshot) -> Result<Self, DirectoryError> {
        // Không để graph tự chấp nhận reference "ma" từ snapshot chưa validate.
        let snapshot = snapshot.clone().validated()?;
        let mut nodes = BTreeMap::new();
        let mut edges = Vec::new();
        for object in &snapshot.objects {
            nodes.insert(
                object.id.clone(),
                AdNode {
                    id: object.id.clone(),
                    kind: object.kind,
                },
            );
            if object.owner != object.id {
                edges.push(AdEdge {
                    from: object.owner.clone(),
                    to: object.id.clone(),
                    kind: EdgeKind::Owns,
                    evidence: Evidence::new("synthetic-owner-attribute", &object.dn.to_string()),
                });
            }
        }
        for membership in &snapshot.memberships {
            edges.push(AdEdge {
                from: membership.member.clone(),
                to: membership.group.clone(),
                kind: EdgeKind::MemberOf,
                evidence: membership.evidence.clone(),
            });
        }
        for acl in &snapshot.acls {
            let Some(kind) = EdgeKind::from_right(acl.right) else {
                continue;
            };
            if snapshot
                .access_decision(&acl.principal, &acl.target, acl.right)
                .status
                == AccessStatus::Allowed
            {
                edges.push(AdEdge {
                    from: acl.principal.clone(),
                    to: acl.target.clone(),
                    kind,
                    evidence: acl.evidence.clone(),
                });
            }
        }
        edges.sort_by(|left, right| {
            (&left.from, &left.to, left.kind).cmp(&(&right.from, &right.to, right.kind))
        });
        edges.dedup();
        Ok(Self {
            nodes,
            edges,
            acl_entries: snapshot.acls.clone(),
        })
    }

    fn effective_principals(&self, start: &ObjectId) -> BTreeSet<ObjectId> {
        let mut principals = BTreeSet::from([start.clone()]);
        let mut stack = vec![start.clone()];
        while let Some(current) = stack.pop() {
            for group in self
                .edges
                .iter()
                .filter(|edge| {
                    edge.kind == EdgeKind::MemberOf
                        && edge.from == current
                        && self.nodes.contains_key(&edge.to)
                })
                .map(|edge| &edge.to)
            {
                if principals.insert(group.clone()) {
                    stack.push(group.clone());
                }
            }
        }
        principals
    }

    fn acl_edge_permitted(&self, principals: &BTreeSet<ObjectId>, edge: &AdEdge) -> bool {
        let Some(right) = edge.kind.access_right() else {
            return true;
        };
        !self.acl_entries.iter().any(|acl| {
            acl.effect == AceEffect::Deny
                && acl.target == edge.to
                && acl.right == right
                && principals.contains(&acl.principal)
        })
    }

    /// DFS bằng stack; mỗi candidate mang visited riêng để chu trình không gây treo
    /// và một nhánh dài không che mất nhánh ngắn hơn trong giới hạn depth.
    pub fn find_path(
        &self,
        start: &ObjectId,
        goal: &ObjectId,
        max_depth: usize,
    ) -> Option<AttackPath> {
        if !self.nodes.contains_key(start) || !self.nodes.contains_key(goal) {
            return None;
        }
        if start == goal {
            return Some(AttackPath {
                start: start.clone(),
                goal: goal.clone(),
                edges: Vec::new(),
            });
        }
        if max_depth == 0 {
            return None;
        }
        let principals = self.effective_principals(start);
        let mut stack = vec![(start.clone(), Vec::new(), BTreeSet::from([start.clone()]))];
        while let Some((current, path, visited)) = stack.pop() {
            if path.len() == max_depth {
                continue;
            }
            let mut outgoing = self
                .edges
                .iter()
                .filter(|edge| {
                    edge.from == current
                        && self.nodes.contains_key(&edge.from)
                        && self.nodes.contains_key(&edge.to)
                        && !visited.contains(&edge.to)
                        && self.acl_edge_permitted(&principals, edge)
                })
                .collect::<Vec<_>>();
            outgoing.sort_by(|left, right| (&right.to, right.kind).cmp(&(&left.to, left.kind)));
            for edge in outgoing {
                let mut next_path = path.clone();
                next_path.push(edge.clone());
                if edge.to == *goal {
                    return Some(AttackPath {
                        start: start.clone(),
                        goal: goal.clone(),
                        edges: next_path,
                    });
                }
                let mut next_visited = visited.clone();
                next_visited.insert(edge.to.clone());
                stack.push((edge.to.clone(), next_path, next_visited));
            }
        }
        None
    }

    pub fn explain_no_path(&self, start: &ObjectId, goal: &ObjectId, max_depth: usize) -> String {
        if self.find_path(start, goal, max_depth).is_some() {
            return "Có path trong giới hạn depth; hãy kiểm tra từng precondition.".to_string();
        }
        format!(
            "Không có typed path {start} → {goal} trong depth {max_depth}. Nguyên nhân có thể là thiếu edge, depth quá nhỏ hoặc dữ liệu chưa biết; trust/OU không tự tạo quyền."
        )
    }
}

// ============================================================================
// 6. KERBEROS OFFLINE: MESSAGE TYPES, KHÔNG PHẢI CRYPTOGRAPHY
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionType {
    DesCbcCrc,
    Aes256CtsHmacSha1,
    Rc4Hmac,
    Rc4HmacExp,
}

impl EncryptionType {
    pub fn protocol_id(self) -> i32 {
        match self {
            Self::DesCbcCrc => 1,
            Self::Aes256CtsHmacSha1 => 18,
            Self::Rc4Hmac => 23,
            Self::Rc4HmacExp => 24,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketKind {
    Tgt,
    Service,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticTicket {
    pub kind: TicketKind,
    pub realm: String,
    pub client: ObjectId,
    pub service: String,
    pub spn: Option<String>,
    pub etype: EncryptionType,
    /// Nhãn phiên tổng hợp, không phải key, password, hash hay ticket bytes.
    pub session_label: String,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreAuthenticationData {
    pub client: ObjectId,
    /// Minh họa trạng thái đã xác thực trên fixture, không phải mật khẩu.
    pub synthetic_proof: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsReq {
    pub client: ObjectId,
    pub realm: String,
    pub pre_authentication: Option<PreAuthenticationData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsRep {
    pub tgt: SyntheticTicket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authenticator {
    pub client: ObjectId,
    pub realm: String,
    pub synthetic_timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApReq {
    pub ticket: SyntheticTicket,
    pub authenticator: Authenticator,
}

/// PA-TGS-REQ chứa AP-REQ (TGT + authenticator), không chứa AS-REP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaTgsReq {
    pub ap_req: ApReq,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TgsReq {
    pub pa_tgs_req: PaTgsReq,
    pub requested_spn: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TgsRep {
    pub service_ticket: SyntheticTicket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KerberosError {
    UnknownClient(ObjectId),
    UnknownServicePrincipal(String),
    RealmMismatch,
    PreAuthenticationRequired,
    InvalidPreAuthentication,
    TicketKindMismatch {
        expected: TicketKind,
        actual: TicketKind,
    },
    ClientMismatch,
}

impl fmt::Display for KerberosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Kerberos exchange tổng hợp bị từ chối: {self:?}")
    }
}

impl Error for KerberosError {}

#[derive(Debug, Clone)]
pub struct MockKerberosKdc {
    pub realm: String,
    clients: BTreeSet<ObjectId>,
    services: BTreeMap<String, ObjectId>,
}

impl MockKerberosKdc {
    pub fn from_snapshot(snapshot: &DirectorySnapshot) -> Self {
        let clients = snapshot
            .objects
            .iter()
            .filter(|object| object.kind == ObjectKind::User)
            .map(|object| object.id.clone())
            .collect();
        let services = snapshot
            .objects
            .iter()
            .flat_map(|object| {
                object
                    .spns
                    .iter()
                    .map(move |spn| (spn.to_ascii_lowercase(), object.id.clone()))
            })
            .collect();
        Self {
            realm: "LAB.EXAMPLE".to_string(),
            clients,
            services,
        }
    }

    pub fn as_exchange(&self, request: &AsReq) -> Result<AsRep, KerberosError> {
        if !request.realm.eq_ignore_ascii_case(&self.realm) {
            return Err(KerberosError::RealmMismatch);
        }
        if !self.clients.contains(&request.client) {
            return Err(KerberosError::UnknownClient(request.client.clone()));
        }
        let proof = request
            .pre_authentication
            .as_ref()
            .ok_or(KerberosError::PreAuthenticationRequired)?;
        if proof.client != request.client || proof.synthetic_proof != "SYNTHETIC-PREAUTH-OK" {
            return Err(KerberosError::InvalidPreAuthentication);
        }
        Ok(AsRep {
            tgt: SyntheticTicket {
                kind: TicketKind::Tgt,
                realm: self.realm.clone(),
                client: request.client.clone(),
                service: format!("krbtgt/{}", self.realm),
                spn: None,
                etype: EncryptionType::Aes256CtsHmacSha1,
                session_label: format!("SYNTHETIC-TGT-SESSION-{}", request.client),
                evidence: Evidence::new("synthetic-kdc", "AS-REQ → AS-REP; pre-auth validated"),
            },
        })
    }

    pub fn tgs_exchange(&self, request: &TgsReq) -> Result<TgsRep, KerberosError> {
        let ap_req = &request.pa_tgs_req.ap_req;
        let tgt = &ap_req.ticket;
        if tgt.kind != TicketKind::Tgt {
            return Err(KerberosError::TicketKindMismatch {
                expected: TicketKind::Tgt,
                actual: tgt.kind,
            });
        }
        if !tgt.realm.eq_ignore_ascii_case(&self.realm)
            || !ap_req.authenticator.realm.eq_ignore_ascii_case(&self.realm)
        {
            return Err(KerberosError::RealmMismatch);
        }
        if !self.clients.contains(&tgt.client) {
            return Err(KerberosError::UnknownClient(tgt.client.clone()));
        }
        if tgt.client != ap_req.authenticator.client {
            return Err(KerberosError::ClientMismatch);
        }
        let service = self
            .services
            .get(&request.requested_spn.to_ascii_lowercase())
            .ok_or_else(|| KerberosError::UnknownServicePrincipal(request.requested_spn.clone()))?;
        Ok(TgsRep {
            service_ticket: SyntheticTicket {
                kind: TicketKind::Service,
                realm: self.realm.clone(),
                client: tgt.client.clone(),
                service: service.to_string(),
                spn: Some(request.requested_spn.clone()),
                etype: EncryptionType::Aes256CtsHmacSha1,
                session_label: format!("SYNTHETIC-SERVICE-SESSION-{}", request.requested_spn),
                evidence: Evidence::new(
                    "synthetic-kdc",
                    "PA-TGS-REQ/AP-REQ → TGS-REP; SPN resolved from snapshot",
                ),
            },
        })
    }

    pub fn request_service_ticket(
        &self,
        client: &ObjectId,
        spn: &str,
    ) -> Result<TgsRep, KerberosError> {
        let as_rep = self.as_exchange(&AsReq {
            client: client.clone(),
            realm: self.realm.clone(),
            pre_authentication: Some(PreAuthenticationData {
                client: client.clone(),
                synthetic_proof: "SYNTHETIC-PREAUTH-OK".to_string(),
            }),
        })?;
        self.tgs_exchange(&TgsReq {
            pa_tgs_req: PaTgsReq {
                ap_req: ApReq {
                    ticket: as_rep.tgt,
                    authenticator: Authenticator {
                        client: client.clone(),
                        realm: self.realm.clone(),
                        synthetic_timestamp: 1_700_000_000,
                    },
                },
            },
            requested_spn: spn.to_string(),
        })
    }
}

// ============================================================================
// 7. FIXTURE TỔNG HỢP
// ============================================================================

fn id(value: &str) -> ObjectId {
    ObjectId::new(value).expect("fixture ObjectId hợp lệ")
}

fn dn(value: &str) -> DistinguishedName {
    DistinguishedName::parse(value).expect("fixture DN hợp lệ")
}

fn evidence(detail: &str) -> Evidence {
    Evidence::new("synthetic-directory-export", detail)
}

fn object(suffix: &str, kind: ObjectKind, name: &str, ou: &str, spns: &[&str]) -> AdObject {
    let domain_dn = "DC=lab,DC=example";
    let object_dn = if ou.is_empty() {
        format!("CN={name},{domain_dn}")
    } else {
        format!("CN={name},OU={ou},{domain_dn}")
    };
    AdObject {
        id: id(&format!(
            "S-1-5-21-1000000001-1000000002-1000000003-{suffix}"
        )),
        kind,
        name: name.to_string(),
        dn: dn(&object_dn),
        owner: id("S-1-5-21-1000000001-1000000002-1000000003-512"),
        guid: Some(format!("00000000-0000-4000-8000-{suffix:0>12}")),
        group_scope: (kind == ObjectKind::Group).then_some(GroupScope::Global),
        spns: spns.iter().map(|spn| (*spn).to_string()).collect(),
        site: Some("SYNTHETIC-SITE-A".to_string()),
    }
}

#[derive(Debug, Clone)]
pub struct MockADMapper {
    pub snapshot: DirectorySnapshot,
    pub graph: AdGraph,
}

impl MockADMapper {
    pub fn standard() -> Self {
        let admin = object("512", ObjectKind::Group, "Domain Admins", "Groups", &[]);
        let alice = object("1101", ObjectKind::User, "alice", "Users", &[]);
        let bob = object("1102", ObjectKind::User, "bob", "Users", &[]);
        let helpdesk = object("2101", ObjectKind::Group, "Helpdesk", "Groups", &[]);
        let operations = object("2102", ObjectKind::Group, "IT Operations", "Groups", &[]);
        let cycle_a = object("2103", ObjectKind::Group, "Cycle A", "Groups", &[]);
        let cycle_b = object("2104", ObjectKind::Group, "Cycle B", "Groups", &[]);
        let workstation = object("3101", ObjectKind::Computer, "WS01", "Workstations", &[]);
        let service = object(
            "4101",
            ObjectKind::ServiceAccount,
            "svc-web",
            "Services",
            &["HTTP/web.lab.example", "HTTP/web"],
        );
        let domain = AdObject::new(
            id("S-1-5-21-1000000001-1000000002-1000000003"),
            ObjectKind::Domain,
            "LAB",
            dn("DC=lab,DC=example"),
            admin.id.clone(),
        );
        let child_domain_id = id("S-1-5-21-2000000001-2000000002-2000000003");
        let child_domain = AdObject::new(
            child_domain_id.clone(),
            ObjectKind::Domain,
            "CHILD",
            dn("DC=child,DC=lab,DC=example"),
            child_domain_id,
        );
        let other_tree_id = id("S-1-5-21-3000000001-3000000002-3000000003");
        let other_tree = AdObject::new(
            other_tree_id.clone(),
            ObjectKind::Domain,
            "OTHER",
            dn("DC=other,DC=example"),
            other_tree_id,
        );
        let users_ou = AdObject::new(
            id("OU-00000001"),
            ObjectKind::OrganizationalUnit,
            "Users",
            dn("OU=Users,DC=lab,DC=example"),
            admin.id.clone(),
        );
        let groups_ou = AdObject::new(
            id("OU-00000002"),
            ObjectKind::OrganizationalUnit,
            "Groups",
            dn("OU=Groups,DC=lab,DC=example"),
            admin.id.clone(),
        );
        let workstations_ou = AdObject::new(
            id("OU-00000003"),
            ObjectKind::OrganizationalUnit,
            "Workstations",
            dn("OU=Workstations,DC=lab,DC=example"),
            admin.id.clone(),
        );
        let services_ou = AdObject::new(
            id("OU-00000004"),
            ObjectKind::OrganizationalUnit,
            "Services",
            dn("OU=Services,DC=lab,DC=example"),
            admin.id.clone(),
        );
        let dc = AdObject {
            site: Some("SYNTHETIC-SITE-A".to_string()),
            ..object(
                "1000",
                ObjectKind::DomainController,
                "DC01",
                "Workstations",
                &[],
            )
        };
        let objects = vec![
            domain.clone(),
            child_domain,
            other_tree,
            users_ou,
            groups_ou,
            workstations_ou,
            services_ou,
            admin.clone(),
            alice.clone(),
            bob.clone(),
            helpdesk.clone(),
            operations.clone(),
            cycle_a.clone(),
            cycle_b.clone(),
            workstation.clone(),
            service.clone(),
            dc,
        ];
        let membership = |member: &AdObject, group: &AdObject| GroupMembership {
            member: member.id.clone(),
            group: group.id.clone(),
            evidence: evidence("synthetic direct memberOf"),
        };
        let memberships = vec![
            membership(&alice, &helpdesk),
            membership(&helpdesk, &operations),
            membership(&bob, &cycle_a),
            membership(&cycle_a, &cycle_b),
            membership(&cycle_b, &cycle_a),
        ];
        let acl = |principal: &AdObject,
                   target: &AdObject,
                   right: AccessRight,
                   effect: AceEffect| AclEntry {
            principal: principal.id.clone(),
            target: target.id.clone(),
            right,
            effect,
            inherited: false,
            evidence: evidence("synthetic explicit ACE"),
        };
        let inherited_acl = |principal: &AdObject,
                             target: &AdObject,
                             right: AccessRight,
                             effect: AceEffect| AclEntry {
            inherited: true,
            evidence: evidence("synthetic ACE inherited from Workstations OU"),
            ..acl(principal, target, right, effect)
        };
        let acls = vec![
            inherited_acl(
                &operations,
                &workstation,
                AccessRight::GenericAll,
                AceEffect::Allow,
            ),
            acl(&cycle_b, &admin, AccessRight::GenericAll, AceEffect::Allow),
            acl(
                &helpdesk,
                &service,
                AccessRight::ForceChangePassword,
                AceEffect::Deny,
            ),
            acl(
                &admin,
                &domain,
                AccessRight::ReplicationGetChangesAll,
                AceEffect::Allow,
            ),
        ];
        let baseline = Gpo {
            id: id("GPO-00000001"),
            name: "Synthetic Workstation Baseline".to_string(),
            links: vec![GpoLink {
                ou: dn("OU=Workstations,DC=lab,DC=example"),
                enabled: true,
                enforced: false,
            }],
            security_filter: vec![workstation.id.clone()],
        };
        let enforced = Gpo {
            id: id("GPO-00000002"),
            name: "Synthetic Enforced Audit Policy".to_string(),
            links: vec![GpoLink {
                ou: dn("DC=lab,DC=example"),
                enabled: true,
                enforced: true,
            }],
            security_filter: Vec::new(),
        };
        let snapshot = DirectorySnapshot {
            objects,
            memberships,
            trusts: vec![
                Trust {
                    source_domain: "lab.example".to_string(),
                    target_domain: "partner.example".to_string(),
                    direction: TrustDirection::Outbound,
                    transitive: false,
                    evidence: evidence("synthetic outbound external trust record"),
                },
                Trust {
                    source_domain: "lab.example".to_string(),
                    target_domain: "child.lab.example".to_string(),
                    direction: TrustDirection::Bidirectional,
                    transitive: true,
                    evidence: evidence("synthetic parent-child trust record"),
                },
                Trust {
                    source_domain: "lab.example".to_string(),
                    target_domain: "other.example".to_string(),
                    direction: TrustDirection::Bidirectional,
                    transitive: true,
                    evidence: evidence("synthetic forest tree-root trust record"),
                },
                Trust {
                    source_domain: "lab.example".to_string(),
                    target_domain: "vendor.example".to_string(),
                    direction: TrustDirection::Inbound,
                    transitive: false,
                    evidence: evidence("synthetic inbound external trust record"),
                },
            ],
            gpos: vec![baseline, enforced],
            acls,
            blocked_inheritance_ous: BTreeSet::new(),
        }
        .validated()
        .expect("fixture AD phải hợp lệ");
        let graph = AdGraph::from_snapshot(&snapshot).expect("graph từ snapshot hợp lệ");
        Self { snapshot, graph }
    }

    pub fn find_path(
        &self,
        start: &ObjectId,
        goal: &ObjectId,
        max_depth: usize,
    ) -> Option<AttackPath> {
        self.graph.find_path(start, goal, max_depth)
    }

    pub fn explain_no_path(&self, start: &ObjectId, goal: &ObjectId, max_depth: usize) -> String {
        self.graph.explain_no_path(start, goal, max_depth)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let lab = MockADMapper::standard();
    let alice = id("S-1-5-21-1000000001-1000000002-1000000003-1101");
    let operations = id("S-1-5-21-1000000001-1000000002-1000000003-2102");
    let workstation = id("S-1-5-21-1000000001-1000000002-1000000003-3101");
    let filter = LdapFilter::parse("(&(objectClass=user)(servicePrincipalName=*))")?;
    let service_accounts =
        lab.snapshot
            .search(&dn("DC=lab,DC=example"), SearchScope::Subtree, &filter)?;
    println!("MockADMapper — toàn bộ directory đều tổng hợp");
    println!("objects: {}", lab.snapshot.objects.len());
    println!(
        "alice effective groups: {}",
        lab.snapshot.effective_groups(&alice).len()
    );
    println!("service accounts with SPN: {}", service_accounts.len());
    println!("SPN inventory rows: {}", lab.snapshot.spn_inventory().len());
    if let Some(path) = lab.find_path(&alice, &workstation, 4) {
        println!(
            "alice → WS01: {} edges\n{}",
            path.edges.len(),
            path.explain()
        );
    }
    println!(
        "alice ∈ IT Operations: {}",
        lab.snapshot.is_member_of(&alice, &operations)
    );
    println!(
        "trust permits partner → lab authentication: {}",
        lab.snapshot.trusts[0].permits_authentication("partner.example", "lab.example")
    );
    println!("trust implies admin: false (không có quy tắc suy diễn như vậy)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALICE: &str = "S-1-5-21-1000000001-1000000002-1000000003-1101";
    const BOB: &str = "S-1-5-21-1000000001-1000000002-1000000003-1102";
    const ADMIN: &str = "S-1-5-21-1000000001-1000000002-1000000003-512";
    const HELPDESK: &str = "S-1-5-21-1000000001-1000000002-1000000003-2101";
    const OPERATIONS: &str = "S-1-5-21-1000000001-1000000002-1000000003-2102";
    const CYCLE_A: &str = "S-1-5-21-1000000001-1000000002-1000000003-2103";
    const CYCLE_B: &str = "S-1-5-21-1000000001-1000000002-1000000003-2104";
    const WS01: &str = "S-1-5-21-1000000001-1000000002-1000000003-3101";
    const SERVICE: &str = "S-1-5-21-1000000001-1000000002-1000000003-4101";

    #[test]
    fn object_id_rejects_empty_space_control_and_long_values() {
        for invalid in ["", "has space", "line\nfeed", "é"] {
            assert!(ObjectId::new(invalid).is_err());
        }
        assert!(ObjectId::new(&"A".repeat(129)).is_err());
        assert!(ObjectId::new("SYNTHETIC-1").is_ok());
    }

    #[test]
    fn dn_parses_components_and_normalizes_domain_case() {
        let parsed = dn("CN=Alice,OU=Users,DC=LAB,DC=Example");
        assert_eq!(parsed.value(DnKey::CommonName), Some("Alice"));
        assert_eq!(parsed.value(DnKey::OrganizationalUnit), Some("Users"));
        assert_eq!(parsed.domain(), "lab.example");
        assert_eq!(parsed.to_string(), "CN=Alice,OU=Users,DC=LAB,DC=Example");
    }

    #[test]
    fn dn_rejects_missing_dc_bad_order_empty_and_trailing_input() {
        for invalid in [
            "",
            "CN=Alice",
            "DC=lab,OU=Users",
            "CN=Alice,CN=Bob,DC=lab",
            "CN=,DC=lab",
            "CN=Alice,DC=lab,",
            "X=Alice,DC=lab",
            " CN=Alice,DC=lab",
            "CN=A\\,B,DC=lab",
        ] {
            assert!(
                DistinguishedName::parse(invalid).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn dn_suffix_and_parent_scopes_are_exact() {
        let child = dn("CN=WS01,OU=Workstations,DC=lab,DC=example");
        let parent = dn("OU=Workstations,DC=lab,DC=example");
        assert_eq!(child.parent(), Some(parent.clone()));
        assert!(child.within(&parent));
        assert!(!parent.within(&child));
        assert!(!child.within(&dn("OU=Users,DC=lab,DC=example")));
    }

    #[test]
    fn fixture_has_unique_ids_and_no_ghost_references() {
        let lab = MockADMapper::standard();
        let ids = lab
            .snapshot
            .objects
            .iter()
            .map(|object| &object.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), lab.snapshot.objects.len());
        for domain in ["lab.example", "child.lab.example", "other.example"] {
            assert!(
                lab.snapshot
                    .objects
                    .iter()
                    .any(|object| object.kind == ObjectKind::Domain
                        && object.dn.domain() == domain)
            );
        }
        assert!(lab.snapshot.acls.iter().all(|acl| {
            lab.snapshot.object(&acl.principal).is_some()
                && lab.snapshot.object(&acl.target).is_some()
        }));
        assert!(lab.snapshot.acls.iter().any(|acl| acl.inherited));
        assert!(
            lab.snapshot
                .acls
                .iter()
                .any(|acl| { acl.effect == AceEffect::Deny && !acl.inherited })
        );
        assert!(
            lab.snapshot
                .acls
                .iter()
                .any(|acl| { acl.right == AccessRight::ReplicationGetChangesAll })
        );
    }

    #[test]
    fn duplicate_object_id_is_rejected() {
        let mut snapshot = MockADMapper::standard().snapshot;
        let duplicate = snapshot.objects[0].clone();
        snapshot.objects.push(duplicate);
        assert!(matches!(
            snapshot.validated(),
            Err(DirectoryError::DuplicateObject(_))
        ));
    }

    #[test]
    fn missing_membership_reference_is_rejected_not_created() {
        let mut snapshot = MockADMapper::standard().snapshot;
        snapshot.memberships.push(GroupMembership {
            member: id("S-1-5-21-1000000001-1000000002-1000000003-9999"),
            group: id(HELPDESK),
            evidence: evidence("missing synthetic user"),
        });
        assert!(matches!(
            snapshot.validated(),
            Err(DirectoryError::MissingReference { .. })
        ));
    }

    #[test]
    fn membership_target_must_be_a_group() {
        let mut snapshot = MockADMapper::standard().snapshot;
        snapshot.memberships.push(GroupMembership {
            member: id(ALICE),
            group: id(WS01),
            evidence: evidence("invalid group target"),
        });
        assert!(snapshot.validated().is_err());
    }

    #[test]
    fn effective_group_nesting_is_transitive() {
        let lab = MockADMapper::standard();
        assert!(lab.snapshot.is_member_of(&id(ALICE), &id(HELPDESK)));
        assert!(lab.snapshot.is_member_of(&id(ALICE), &id(OPERATIONS)));
        assert!(!lab.snapshot.is_member_of(&id(ALICE), &id(ADMIN)));
    }

    #[test]
    fn group_cycle_terminates_and_does_not_add_self() {
        let lab = MockADMapper::standard();
        let groups = lab.snapshot.effective_groups(&id(CYCLE_A));
        assert!(groups.contains(&id(CYCLE_B)));
        assert!(!groups.contains(&id(CYCLE_A)));
        assert!(
            lab.snapshot
                .effective_groups(&id(BOB))
                .contains(&id(CYCLE_A))
        );
    }

    #[test]
    fn present_filter_finds_service_accounts() {
        let lab = MockADMapper::standard();
        let filter =
            LdapFilter::parse("(&(objectClass=user)(servicePrincipalName=*))").expect("filter");
        let found = lab
            .snapshot
            .search(&dn("DC=lab,DC=example"), SearchScope::Subtree, &filter)
            .expect("search");
        assert_eq!(
            found
                .iter()
                .map(|object| object.name.as_str())
                .collect::<Vec<_>>(),
            vec!["svc-web"]
        );
    }

    #[test]
    fn boolean_not_or_and_hex_escape_work() {
        let filter = LdapFilter::parse(
            "(&(|(sAMAccountName=alice)(sAMAccountName=bob))(!(sAMAccountName=\\62ob)))",
        )
        .expect("filter");
        let lab = MockADMapper::standard();
        let found = lab
            .snapshot
            .search(
                &dn("OU=Users,DC=lab,DC=example"),
                SearchScope::OneLevel,
                &filter,
            )
            .expect("search");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "alice");
    }

    #[test]
    fn ldap_parser_rejects_bad_parentheses_and_trailing_input() {
        for invalid in [
            "",
            "objectClass=user",
            "(objectClass=user",
            "(&(objectClass=user))extra",
            "(&)",
            "(!)",
            "(objectClass=user))",
        ] {
            assert!(LdapFilter::parse(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn ldap_parser_rejects_bad_attributes_escapes_and_substrings() {
        for invalid in [
            "(=user)",
            "(bad attr=user)",
            "(name=alice*)",
            "(name=\\zz)",
            "(name=\\0)",
            "(name=a(b)",
        ] {
            assert!(LdapFilter::parse(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn base_one_level_and_subtree_scopes_differ() {
        let lab = MockADMapper::standard();
        let base = dn("ou=users,dc=LAB,dc=Example");
        let all = LdapFilter::parse("(objectClass=*)").expect("filter");
        assert_eq!(
            lab.snapshot
                .search(&base, SearchScope::Base, &all)
                .expect("base")
                .len(),
            1
        );
        assert_eq!(
            lab.snapshot
                .search(&base, SearchScope::OneLevel, &all)
                .expect("one")
                .len(),
            2
        );
        assert_eq!(
            lab.snapshot
                .search(&base, SearchScope::Subtree, &all)
                .expect("subtree")
                .len(),
            3
        );
    }

    #[test]
    fn unknown_search_base_is_an_error() {
        let lab = MockADMapper::standard();
        let result = lab.snapshot.search(
            &dn("OU=Missing,DC=lab,DC=example"),
            SearchScope::Subtree,
            &LdapFilter::Present("objectClass".to_string()),
        );
        assert!(matches!(
            result,
            Err(DirectoryError::InvalidLdap(LdapError::UnknownBase))
        ));
    }

    #[test]
    fn paged_enumeration_is_sorted_and_has_exact_final_page() {
        let lab = MockADMapper::standard();
        let pages = lab
            .snapshot
            .paged_search(
                &dn("DC=lab,DC=example"),
                SearchScope::Subtree,
                &LdapFilter::Present("objectClass".to_string()),
                5,
            )
            .expect("pages")
            .collect::<Vec<_>>();
        let lengths = pages.iter().map(Vec::len).collect::<Vec<_>>();
        // 16 object thuộc lab/child; other.example là tree root ngoài search base này.
        assert_eq!(lengths, vec![5, 5, 5, 1]);
        let ids = pages
            .into_iter()
            .flatten()
            .map(|object| &object.id)
            .collect::<Vec<_>>();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn zero_page_size_is_rejected() {
        let lab = MockADMapper::standard();
        assert!(matches!(
            lab.snapshot.paged_search(
                &dn("DC=lab,DC=example"),
                SearchScope::Subtree,
                &LdapFilter::Present("objectClass".to_string()),
                0
            ),
            Err(DirectoryError::InvalidLdap(LdapError::EmptyPageSize))
        ));
    }

    #[test]
    fn spn_inventory_has_one_row_per_spn() {
        let lab = MockADMapper::standard();
        let inventory = lab.snapshot.spn_inventory();
        assert_eq!(inventory.len(), 2);
        assert!(
            inventory
                .iter()
                .all(|record| record.object_id == id(SERVICE))
        );
        assert_eq!(inventory[0].spn, "HTTP/web.lab.example");
    }

    #[test]
    fn missing_acl_data_is_unknown_not_allowed() {
        let lab = MockADMapper::standard();
        assert_eq!(
            lab.snapshot
                .access_decision(&id(ALICE), &id(ADMIN), AccessRight::WriteDacl)
                .status,
            AccessStatus::Unknown
        );
    }

    #[test]
    fn inherited_group_allow_grants_right() {
        let lab = MockADMapper::standard();
        let decision = lab
            .snapshot
            .access_decision(&id(ALICE), &id(WS01), AccessRight::GenericAll);
        assert_eq!(decision.status, AccessStatus::Allowed);
        assert!(!decision.evidence.is_empty());
        assert!(
            lab.snapshot
                .acls
                .iter()
                .any(|acl| acl.principal == id(OPERATIONS)
                    && acl.target == id(WS01)
                    && acl.inherited
                    && acl.evidence.detail.contains("inherited"))
        );
    }

    #[test]
    fn explicit_deny_wins_over_group_allow() {
        let mut lab = MockADMapper::standard();
        lab.snapshot.acls.push(AclEntry {
            principal: id(OPERATIONS),
            target: id(SERVICE),
            right: AccessRight::ForceChangePassword,
            effect: AceEffect::Allow,
            inherited: false,
            evidence: evidence("synthetic allow to test deny precedence"),
        });
        lab.snapshot = lab.snapshot.validated().expect("valid ACL fixture");
        assert_eq!(
            lab.snapshot
                .access_decision(&id(ALICE), &id(SERVICE), AccessRight::ForceChangePassword)
                .status,
            AccessStatus::Denied
        );
    }

    #[test]
    fn graph_path_cannot_bypass_explicit_deny_through_another_group() {
        let mut lab = MockADMapper::standard();
        lab.snapshot.acls.push(AclEntry {
            principal: id(OPERATIONS),
            target: id(SERVICE),
            right: AccessRight::ForceChangePassword,
            effect: AceEffect::Allow,
            inherited: false,
            evidence: evidence("synthetic allow that must not bypass deny"),
        });
        lab.snapshot = lab.snapshot.validated().expect("valid ACL fixture");
        lab.graph = AdGraph::from_snapshot(&lab.snapshot).expect("valid graph");
        assert!(lab.graph.edges.iter().any(|edge| {
            edge.from == id(OPERATIONS)
                && edge.to == id(SERVICE)
                && edge.kind == EdgeKind::ForceChangePassword
        }));
        assert!(lab.find_path(&id(ALICE), &id(SERVICE), 5).is_none());
        assert!(lab.find_path(&id(OPERATIONS), &id(SERVICE), 1).is_some());
    }

    #[test]
    fn ownership_does_not_fabricate_unrelated_acl_rights() {
        let lab = MockADMapper::standard();
        assert_eq!(
            lab.snapshot
                .access_decision(&id(ADMIN), &id(SERVICE), AccessRight::GenericRead)
                .status,
            AccessStatus::Unknown
        );
        assert!(lab.graph.edges.iter().any(|edge| edge.from == id(ADMIN)
            && edge.to == id(SERVICE)
            && edge.kind == EdgeKind::Owns));
    }

    #[test]
    fn trust_direction_is_explicit_and_does_not_imply_admin() {
        let lab = MockADMapper::standard();
        let outbound = &lab.snapshot.trusts[0];
        assert!(outbound.permits_authentication("partner.example", "lab.example"));
        assert!(!outbound.permits_authentication("lab.example", "partner.example"));
        assert!(!outbound.permits_authentication("other.example", "lab.example"));
        assert!(!outbound.transitive);

        let child = lab
            .snapshot
            .trusts
            .iter()
            .find(|trust| trust.target_domain == "child.lab.example")
            .expect("parent-child trust");
        assert_eq!(child.direction, TrustDirection::Bidirectional);
        assert!(child.transitive);
        assert!(child.permits_authentication("child.lab.example", "lab.example"));
        assert!(child.permits_authentication("lab.example", "child.lab.example"));

        let inbound = lab
            .snapshot
            .trusts
            .iter()
            .find(|trust| trust.direction == TrustDirection::Inbound)
            .expect("inbound trust");
        assert!(inbound.permits_authentication("lab.example", "vendor.example"));
        assert!(!inbound.permits_authentication("vendor.example", "lab.example"));
        assert!(
            !lab.graph
                .edges
                .iter()
                .any(|edge| edge.evidence.detail.contains("trust"))
        );
    }

    #[test]
    fn gpo_security_filter_and_link_are_enforced() {
        let lab = MockADMapper::standard();
        let workstation_gpos = lab.snapshot.effective_gpos(&id(WS01));
        assert_eq!(workstation_gpos.len(), 2);
        assert_eq!(lab.snapshot.effective_gpos(&id(ALICE)).len(), 1);
        assert!(lab.snapshot.effective_gpos(&id("missing")).is_empty());
    }

    #[test]
    fn blocked_inheritance_preserves_direct_and_enforced_links() {
        let mut lab = MockADMapper::standard();
        lab.snapshot.gpos.push(Gpo {
            id: id("GPO-00000003"),
            name: "Synthetic Normal Domain Policy".to_string(),
            links: vec![GpoLink {
                ou: dn("DC=lab,DC=example"),
                enabled: true,
                enforced: false,
            }],
            security_filter: Vec::new(),
        });
        lab.snapshot
            .blocked_inheritance_ous
            .insert(dn("OU=Workstations,DC=lab,DC=example"));
        let names = lab
            .snapshot
            .effective_gpos(&id(WS01))
            .iter()
            .map(|gpo| gpo.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "Synthetic Workstation Baseline",
                "Synthetic Enforced Audit Policy"
            ]
        );
    }

    #[test]
    fn disabled_gpo_link_is_not_applied() {
        let mut lab = MockADMapper::standard();
        lab.snapshot.gpos[0].links[0].enabled = false;
        assert_eq!(lab.snapshot.effective_gpos(&id(WS01)).len(), 1);
    }

    #[test]
    fn dfs_returns_real_ordered_edges_with_provenance() {
        let lab = MockADMapper::standard();
        let path = lab.find_path(&id(ALICE), &id(WS01), 4).expect("path");
        assert_eq!(path.edges.len(), 3);
        assert_eq!(path.edges[0].kind, EdgeKind::MemberOf);
        assert_eq!(path.edges[2].kind, EdgeKind::GenericAll);
        assert!(path.validate(&lab.graph));
        assert!(path.explain().contains("synthetic-directory-export"));
    }

    #[test]
    fn dfs_handles_group_cycle_without_hanging() {
        let lab = MockADMapper::standard();
        let path = lab.find_path(&id(BOB), &id(ADMIN), 5).expect("cyclic path");
        assert!(path.validate(&lab.graph));
        assert_eq!(
            path.edges.last().map(|edge| edge.kind),
            Some(EdgeKind::GenericAll)
        );
        assert!(lab.find_path(&id(WS01), &id(CYCLE_A), 10).is_none());
    }

    #[test]
    fn depth_zero_and_short_depth_are_enforced() {
        let lab = MockADMapper::standard();
        assert_eq!(
            lab.find_path(&id(ALICE), &id(ALICE), 0)
                .expect("identity path")
                .edges
                .len(),
            0
        );
        assert!(lab.find_path(&id(ALICE), &id(WS01), 0).is_none());
        assert!(lab.find_path(&id(ALICE), &id(WS01), 2).is_none());
        assert!(lab.find_path(&id(ALICE), &id(WS01), 3).is_some());
        assert!(
            lab.explain_no_path(&id(ALICE), &id(WS01), 2)
                .contains("depth 2")
        );
    }

    #[test]
    fn path_validation_rejects_fabricated_or_disconnected_edges() {
        let lab = MockADMapper::standard();
        let real = lab.find_path(&id(ALICE), &id(WS01), 4).expect("path");
        let mut fake = real.clone();
        fake.edges[0].evidence.source.clear();
        assert!(!fake.validate(&lab.graph));
        let mut disconnected = real;
        disconnected.edges.remove(1);
        assert!(!disconnected.validate(&lab.graph));

        let ghost = id("S-1-5-21-1000000001-1000000002-1000000003-9999");
        let mut invalid_snapshot = lab.snapshot.clone();
        invalid_snapshot.memberships.push(GroupMembership {
            member: id(ALICE),
            group: ghost.clone(),
            evidence: evidence("ghost membership must be rejected"),
        });
        invalid_snapshot.acls.push(AclEntry {
            principal: ghost.clone(),
            target: id(SERVICE),
            right: AccessRight::GenericAll,
            effect: AceEffect::Allow,
            inherited: false,
            evidence: evidence("ghost ACL must be rejected"),
        });
        assert!(matches!(
            AdGraph::from_snapshot(&invalid_snapshot),
            Err(DirectoryError::MissingReference { .. })
        ));

        // Public fields cũng không được phép mở đường qua node không tồn tại.
        let mut graph = lab.graph.clone();
        let membership = AdEdge {
            from: id(ALICE),
            to: ghost.clone(),
            kind: EdgeKind::MemberOf,
            evidence: evidence("injected ghost membership"),
        };
        let access = AdEdge {
            from: ghost,
            to: id(SERVICE),
            kind: EdgeKind::GenericAll,
            evidence: evidence("injected ghost ACE"),
        };
        graph.edges.extend([membership.clone(), access.clone()]);
        let ghost_path = AttackPath {
            start: id(ALICE),
            goal: id(SERVICE),
            edges: vec![membership, access],
        };
        assert!(!ghost_path.validate(&graph));
        assert!(graph.find_path(&id(ALICE), &id(SERVICE), 3).is_none());
    }

    #[test]
    fn unknown_goal_does_not_panic() {
        let lab = MockADMapper::standard();
        assert!(lab.find_path(&id(ALICE), &id("missing"), 4).is_none());
        assert!(lab.find_path(&id("missing"), &id(WS01), 4).is_none());
        let ghost_identity = AttackPath {
            start: id("missing"),
            goal: id("missing"),
            edges: Vec::new(),
        };
        assert!(!ghost_identity.validate(&lab.graph));
    }

    #[test]
    fn invalid_evidence_is_rejected() {
        let mut snapshot = MockADMapper::standard().snapshot;
        snapshot.memberships[0].evidence.source.clear();
        assert_eq!(
            snapshot.validated().err(),
            Some(DirectoryError::InvalidEvidence)
        );
    }

    #[test]
    fn kerberos_etypes_match_protocol_values() {
        assert_eq!(EncryptionType::DesCbcCrc.protocol_id(), 1);
        assert_eq!(EncryptionType::Aes256CtsHmacSha1.protocol_id(), 18);
        assert_eq!(EncryptionType::Rc4Hmac.protocol_id(), 23);
        assert_eq!(EncryptionType::Rc4HmacExp.protocol_id(), 24);
    }

    #[test]
    fn as_exchange_requires_valid_synthetic_preauthentication() {
        let lab = MockADMapper::standard();
        let kdc = MockKerberosKdc::from_snapshot(&lab.snapshot);
        let missing = kdc.as_exchange(&AsReq {
            client: id(ALICE),
            realm: "LAB.EXAMPLE".to_string(),
            pre_authentication: None,
        });
        assert_eq!(missing, Err(KerberosError::PreAuthenticationRequired));
        let reply = kdc
            .request_service_ticket(&id(ALICE), "HTTP/web.lab.example")
            .expect("valid synthetic exchange");
        assert_eq!(reply.service_ticket.kind, TicketKind::Service);
    }

    #[test]
    fn pa_tgs_req_contains_ap_req_and_returns_service_ticket_for_spn() {
        let lab = MockADMapper::standard();
        let kdc = MockKerberosKdc::from_snapshot(&lab.snapshot);
        let reply = kdc
            .request_service_ticket(&id(ALICE), "http/WEB.lab.example")
            .expect("SPN matching is ASCII case-insensitive");
        assert_eq!(reply.service_ticket.kind, TicketKind::Service);
        assert_eq!(reply.service_ticket.client, id(ALICE));
        assert_eq!(reply.service_ticket.service, id(SERVICE).to_string());
        assert!(reply.service_ticket.spn.is_some());
        assert!(reply.service_ticket.session_label.starts_with("SYNTHETIC-"));
    }

    #[test]
    fn tgs_exchange_rejects_unknown_spn_without_issuing_ticket() {
        let lab = MockADMapper::standard();
        let kdc = MockKerberosKdc::from_snapshot(&lab.snapshot);
        let error = kdc
            .request_service_ticket(&id(ALICE), "HTTP/missing.lab.example")
            .expect_err("unknown SPN must fail");
        assert_eq!(
            error,
            KerberosError::UnknownServicePrincipal("HTTP/missing.lab.example".to_string())
        );
    }

    #[test]
    fn tgs_exchange_rejects_service_ticket_used_as_tgt() {
        let lab = MockADMapper::standard();
        let kdc = MockKerberosKdc::from_snapshot(&lab.snapshot);
        let service = kdc
            .request_service_ticket(&id(ALICE), "HTTP/web")
            .expect("service ticket")
            .service_ticket;
        let result = kdc.tgs_exchange(&TgsReq {
            pa_tgs_req: PaTgsReq {
                ap_req: ApReq {
                    ticket: service,
                    authenticator: Authenticator {
                        client: id(ALICE),
                        realm: "LAB.EXAMPLE".to_string(),
                        synthetic_timestamp: 1_700_000_000,
                    },
                },
            },
            requested_spn: "HTTP/web".to_string(),
        });
        assert_eq!(
            result,
            Err(KerberosError::TicketKindMismatch {
                expected: TicketKind::Tgt,
                actual: TicketKind::Service,
            })
        );
    }

    #[test]
    fn ap_req_authenticator_must_match_tgt_client() {
        let lab = MockADMapper::standard();
        let kdc = MockKerberosKdc::from_snapshot(&lab.snapshot);
        let tgt = kdc
            .as_exchange(&AsReq {
                client: id(ALICE),
                realm: "LAB.EXAMPLE".to_string(),
                pre_authentication: Some(PreAuthenticationData {
                    client: id(ALICE),
                    synthetic_proof: "SYNTHETIC-PREAUTH-OK".to_string(),
                }),
            })
            .expect("TGT")
            .tgt;
        let result = kdc.tgs_exchange(&TgsReq {
            pa_tgs_req: PaTgsReq {
                ap_req: ApReq {
                    ticket: tgt,
                    authenticator: Authenticator {
                        client: id(BOB),
                        realm: "LAB.EXAMPLE".to_string(),
                        synthetic_timestamp: 1_700_000_000,
                    },
                },
            },
            requested_spn: "HTTP/web".to_string(),
        });
        assert_eq!(result, Err(KerberosError::ClientMismatch));
    }

    #[test]
    fn fixture_and_demo_queries_are_deterministic() {
        let first = MockADMapper::standard();
        let second = MockADMapper::standard();
        assert_eq!(
            first.snapshot.spn_inventory(),
            second.snapshot.spn_inventory()
        );
        assert_eq!(first.graph.edges, second.graph.edges);
        assert_eq!(
            first.find_path(&id(BOB), &id(ADMIN), 5),
            second.find_path(&id(BOB), &id(ADMIN), 5)
        );
    }
}
