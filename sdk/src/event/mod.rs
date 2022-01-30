mod builder;
use std::convert::Infallible;

pub use builder::*;
mod typed;
use cap_sdk_core::transaction::{DetailValue, Event, IndefiniteEvent};
pub use typed::*;

/// Allows a type to be used as a source for an [`IndefiniteEvent`].
///
/// [`IntoEvent`] is implemented for `Vec<(String, DetailValue)>`.
///
/// The type can specify an `operation` and how to turn itself into
/// `Vec<(String, DetailValue)>`, which is how Cap stores event metadata.
///
/// If you are implementing `IntoEvent` on an enum, you should override
/// the default implementation of [`IntoEvent::operation`] with the enum
/// variant, and write the variant's information with [`IntoEvent::details`].
///
/// You must not use to implement a tagged enum, that is non-standard
/// and may increase complexity of querying data for your contract. For examples
/// of how to handle enums, see the `cap-standards` crate source.
pub trait IntoEvent {
    /// The type of operation being executed
    fn operation(&self) -> &'static str {
        ""
    }

    fn details(&self) -> Vec<(String, DetailValue)>;
}

impl IntoEvent for Vec<(String, DetailValue)> {
    fn details(&self) -> Vec<(String, DetailValue)> {
        self.clone()
    }
}

/// Allows a type to be decoded from an [`Event`][crate::Event] or [`IndefiniteEvent`].
pub trait TryFromEvent: Sized {
    type Error;

    fn try_from_event(event: impl MaybeIndefinite) -> Result<Self, Self::Error>;
}

impl TryFromEvent for Vec<(String, DetailValue)> {
    type Error = Infallible;

    fn try_from_event(event: impl MaybeIndefinite) -> Result<Self, Self::Error> {
        let event = event.as_indefinite();

        Ok(event.details)
    }
}

pub trait MaybeIndefinite {
    fn time(&self) -> Option<u64> {
        None
    }

    fn as_indefinite(self) -> IndefiniteEvent;
}

impl MaybeIndefinite for IndefiniteEvent {
    fn as_indefinite(self) -> IndefiniteEvent {
        self
    }
}

impl MaybeIndefinite for Event {
    fn as_indefinite(self) -> IndefiniteEvent {
        self.into()
    }

    fn time(&self) -> Option<u64> {
        Some(self.time)
    }
}
