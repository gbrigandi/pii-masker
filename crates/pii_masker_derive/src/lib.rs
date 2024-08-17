#![allow(unused_imports)]
#![allow(dead_code)]

use pii_masker_inspect::DerivePIIMaskArgs;

extern crate proc_macro;

use darling::{ast, util::Ignored, FromDeriveInput, FromField};
use syn::{parse_macro_input, Data::Struct, DataStruct, DeriveInput, Field, Ident};

use proc_macro::TokenStream;

#[proc_macro_derive(PIIMask, attributes(pii_mask))]
pub fn pii_mask(input: TokenStream) -> TokenStream {
    let original_struct = parse_macro_input!(input as DeriveInput);

    let DeriveInput { data, .. } = original_struct.clone();

    if let Struct(_) = data {

        let _ = match DerivePIIMaskArgs::from_derive_input(&original_struct) {
            Ok(v) => v,
            Err(e) => {
                return TokenStream::from(e.write_errors());
            }
        };

    }

    TokenStream::new()
}

