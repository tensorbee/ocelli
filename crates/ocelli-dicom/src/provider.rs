//! Ordered DICOM metadata providers.

use std::fmt;

use dicom_core::{DicomValue, Tag};

use crate::{MetadataElement, MetadataError, ParsedDicom};

/// Stable caller-selected identity for one provider registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProviderId(&'static str);

impl ProviderId {
    /// Construct a stable provider identity.
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    /// The stable provider identity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// One lookup request shared by every provider function.
#[derive(Debug, Clone, Copy)]
pub struct MetadataRequest<'a> {
    object: &'a ParsedDicom,
    tag: Tag,
}

impl<'a> MetadataRequest<'a> {
    /// Construct an exact-tag request over one parsed object.
    #[must_use]
    pub const fn new(object: &'a ParsedDicom, tag: Tag) -> Self {
        Self { object, tag }
    }

    /// The parsed object being queried.
    #[must_use]
    pub const fn object(self) -> &'a ParsedDicom {
        self.object
    }

    /// The exact requested tag.
    #[must_use]
    pub const fn tag(self) -> Tag {
        self.tag
    }
}

/// Function-pointer metadata provider signature.
pub type ProviderFn =
    for<'a> fn(MetadataRequest<'a>) -> Result<Option<MetadataElement>, MetadataError>;

/// A provider registration failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProviderRegistryError {
    /// A stable identity may be registered once.
    #[error("metadata provider identity is already registered")]
    DuplicateProviderId(ProviderId),
}

#[derive(Debug, Clone, Copy)]
struct ProviderRegistration {
    id: ProviderId,
    function: ProviderFn,
}

/// An ordered registry whose first present answer wins.
#[derive(Debug, Default)]
pub struct ProviderRegistry {
    providers: Vec<ProviderRegistration>,
}

impl ProviderRegistry {
    /// Construct an empty caller-owned registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Append one provider to the precedence order.
    ///
    /// # Errors
    ///
    /// Returns a duplicate error for a repeated stable identity. Function
    /// addresses do not define identity because Rust gives them no reliable
    /// comparison semantics across code-generation units.
    pub fn register(
        &mut self,
        id: ProviderId,
        function: ProviderFn,
    ) -> Result<(), ProviderRegistryError> {
        if self.providers.iter().any(|provider| provider.id == id) {
            return Err(ProviderRegistryError::DuplicateProviderId(id));
        }
        self.providers.push(ProviderRegistration { id, function });
        Ok(())
    }

    /// Ask providers in registration order and return the first present value.
    ///
    /// A present empty element is an answer and stops lookup.
    ///
    /// # Errors
    ///
    /// Returns the structural projection error from the answering provider.
    pub fn lookup(
        &self,
        request: MetadataRequest<'_>,
    ) -> Result<Option<ProviderAnswer>, MetadataError> {
        for provider in &self.providers {
            if let Some(element) = (provider.function)(request)? {
                return Ok(Some(ProviderAnswer {
                    provider: provider.id,
                    element,
                }));
            }
        }
        Ok(None)
    }

    /// Number of registered providers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Whether no providers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

/// One present lookup result with the identity that supplied it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderAnswer {
    provider: ProviderId,
    element: MetadataElement,
}

impl ProviderAnswer {
    /// Identity of the provider that answered.
    #[must_use]
    pub const fn provider(&self) -> ProviderId {
        self.provider
    }

    /// The projected element.
    #[must_use]
    pub const fn element(&self) -> &MetadataElement {
        &self.element
    }

    /// Consume the answer and return the element.
    #[must_use]
    pub fn into_element(self) -> MetadataElement {
        self.element
    }
}

/// Read the requested tag from the main data set.
///
/// # Errors
///
/// Returns [`MetadataError::UnsupportedPixelFragments`] when the requested
/// element is encapsulated Pixel Data.
pub fn data_set_provider(
    request: MetadataRequest<'_>,
) -> Result<Option<MetadataElement>, MetadataError> {
    let Some(element) = request.object().object().get(request.tag()) else {
        return Ok(None);
    };
    MetadataElement::from_dicom_element(request.tag(), element).map(Some)
}

/// Read the requested tag from File Meta Information.
///
/// # Errors
///
/// Returns a structural projection error if a future file-meta representation
/// introduces a non-primitive value.
pub fn file_meta_provider(
    request: MetadataRequest<'_>,
) -> Result<Option<MetadataElement>, MetadataError> {
    let tag = request.tag();
    let Some(element) = request
        .object()
        .object()
        .meta()
        .to_element_iter()
        .find(|element| element.header().tag == tag)
    else {
        return Ok(None);
    };

    match element.value() {
        DicomValue::Primitive(value) => {
            Ok(Some(MetadataElement::from_primitive(element.vr(), value)))
        }
        DicomValue::Sequence(_) | DicomValue::PixelSequence(_) => {
            Err(MetadataError::UnsupportedPixelFragments(tag))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ProviderId, ProviderRegistry};

    #[test]
    fn empty_registry_has_no_implicit_default() {
        let registry = ProviderRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn provider_identity_is_stable_text() {
        assert_eq!(ProviderId::new("data-set").as_str(), "data-set");
    }
}
