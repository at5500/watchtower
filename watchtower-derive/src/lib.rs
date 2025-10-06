//! Derive macros for Watchtower event system
//!
//! This crate provides procedural macros to simplify event creation from structs.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for automatically generating Event conversion for structs
///
/// # Attributes
///
/// - `#[event(type = "event.type")]` - Sets the event type (required)
/// - `#[event(type = "event.type", source = "service-name")]` - Sets event type and source
///
/// # Example
///
/// ```rust
/// use watchtower_derive::Event;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Event, Serialize, Deserialize)]
/// #[event(type = "user.created")]
/// struct UserCreated {
///     user_id: u64,
///     email: String,
///     name: String,
/// }
///
/// // Now you can convert directly to Event:
/// let user_event = UserCreated {
///     user_id: 123,
///     email: "user@example.com".to_string(),
///     name: "John Doe".to_string(),
/// };
///
/// let event: watchtower_core::Event = user_event.into();
/// ```
///
/// # With metadata
///
/// ```rust
/// use watchtower_derive::Event;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Event, Serialize, Deserialize)]
/// #[event(type = "order.created", source = "order-service")]
/// struct OrderCreated {
///     order_id: String,
///     amount: f64,
/// }
/// ```
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Extract event type from attributes
    let (event_type, source) = extract_event_attributes(&input);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Generate Into<Event> implementation
    let expanded = if let Some(source_val) = source {
        quote! {
            impl #impl_generics ::core::convert::Into<watchtower_core::Event> for #name #ty_generics #where_clause {
                fn into(self) -> watchtower_core::Event {
                    let payload = serde_json::to_value(&self)
                        .expect("Failed to serialize event payload");

                    let metadata = watchtower_core::EventMetadata::new(#event_type)
                        .with_source(#source_val);

                    watchtower_core::Event {
                        metadata,
                        payload,
                    }
                }
            }

            impl #impl_generics #name #ty_generics #where_clause {
                /// Create an Event from this struct
                pub fn to_event(self) -> watchtower_core::Event {
                    self.into()
                }

                /// Get the event type for this struct
                pub fn event_type() -> &'static str {
                    #event_type
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics ::core::convert::Into<watchtower_core::Event> for #name #ty_generics #where_clause {
                fn into(self) -> watchtower_core::Event {
                    let payload = serde_json::to_value(&self)
                        .expect("Failed to serialize event payload");

                    watchtower_core::Event::new(#event_type, payload)
                }
            }

            impl #impl_generics #name #ty_generics #where_clause {
                /// Create an Event from this struct
                pub fn to_event(self) -> watchtower_core::Event {
                    self.into()
                }

                /// Get the event type for this struct
                pub fn event_type() -> &'static str {
                    #event_type
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn extract_event_attributes(input: &DeriveInput) -> (String, Option<String>) {
    let mut event_type = None;
    let mut source = None;

    for attr in &input.attrs {
        if attr.path().is_ident("event") {
            // Parse as Meta::List
            attr.parse_nested_meta(|meta| {
                // Check for "type"
                if meta.path.is_ident("type") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    event_type = Some(s.value());
                    Ok(())
                }
                // Check for "source"
                else if meta.path.is_ident("source") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    source = Some(s.value());
                    Ok(())
                } else {
                    Err(meta.error("unrecognized event attribute"))
                }
            }).expect("Failed to parse event attributes");
        }
    }

    let event_type = event_type.expect("Missing #[event(type = \"...\")] attribute");
    (event_type, source)
}

/// Derive macro for event handlers
///
/// Automatically implements handler registration for structs
///
/// # Example
///
/// ```rust
/// use watchtower_derive::EventHandler;
///
/// #[derive(EventHandler)]
/// #[handler(event = "user.created")]
/// struct UserCreatedHandler;
///
/// impl UserCreatedHandler {
///     async fn handle(&self, event: watchtower_core::Event) -> Result<(), watchtower_core::WatchtowerError> {
///         // Handle the event
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_derive(EventHandler, attributes(handler))]
pub fn derive_event_handler(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    // For now, just generate a marker trait
    // Can be extended later for automatic handler registration
    let expanded = quote! {
        impl #name {
            /// Get handler metadata
            pub fn handler_info() -> watchtower_core::HandlerInfo {
                watchtower_core::HandlerInfo {
                    name: stringify!(#name),
                }
            }
        }
    };

    TokenStream::from(expanded)
}
