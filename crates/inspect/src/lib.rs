#![allow(unused_imports)]
#![allow(dead_code)]

extern crate proc_macro;

use std::{fmt::Display, str::FromStr};

use darling::{ast, util::Ignored, FromDeriveInput, FromField, FromMeta};
use syn::{parse_macro_input, Data::Struct, DataStruct, DeriveInput, Field, Ident};

use proc_macro::TokenStream;

#[derive(FromField, Clone, Debug)]
#[darling(attributes(pii_mask))]
pub struct PIIMaskDeriveField {
    pub ident: Option<Ident>,
    #[darling(default)]
    pub faker: MaskType,
    pub format: Option<String>,
}

#[derive(FromDeriveInput, Clone, Debug)]
#[darling(attributes(pii_mask), supports(struct_named))]
pub struct DerivePIIMaskArgs {
    pub ident: Ident,
    pub data: ast::Data<Ignored, PIIMaskDeriveField>,
}

#[derive(FromMeta, PartialEq, Clone, Debug)]
#[darling(default)]
pub enum MaskType {
    Ssn,
    FirstName,
    LastName,
    Email,
    Address,
    City,
    PhoneNumber,
    CreditCard,
    ZipCode,
    PositiveDecimal,
    Inferred,
}

impl Display for MaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaskType::Ssn => write!(f, "ssn"),
            MaskType::FirstName => write!(f, "first_name"),
            MaskType::LastName => write!(f, "last_name"),
            MaskType::Email => write!(f, "email"),
            MaskType::Address => write!(f, "address"),
            MaskType::City => write!(f, "city"),
            MaskType::PhoneNumber => write!(f, "phone_number"),
            MaskType::CreditCard => write!(f, "credit_card"),
            MaskType::ZipCode => write!(f, "zip_code"),
            MaskType::PositiveDecimal => write!(f, "positive_decimal"),
            MaskType::Inferred => write!(f, "inferred"),
        }
    }
}

/*
impl FromStr for MaskType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ssn" => Ok(MaskType::SSN),
            "first_name" => Ok(MaskType::FirstName),
            "last_name" => Ok(MaskType::LastName),
            "email" => Ok(MaskType::Email),
            "address" => Ok(MaskType::Address),
            "city" => Ok(MaskType::City),
            "phone_number" => Ok(MaskType::PhoneNumber),
            "credit_card" => Ok(MaskType::CreditCard),
            "zip_code" => Ok(MaskType::ZipCode),
            "positive_decimal" => Ok(MaskType::PositiveDecimal),
            "inferred" => Ok(MaskType::Inferred),
            _ => Err(format!("Invalid category: {}", s)),
        }
    }
}*/

impl Default for MaskType {
    fn default() -> Self {
        MaskType::Inferred
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_pii_mask_derive_field() {
        let input = r#"
            #[derive(PIIMask)]
            struct Foo {
              #[pii_mask(faker = "first_name")]
              name: String,
            }
        "#;
        let field: DeriveInput = syn::parse_str(input).unwrap();
        let parsed_args = DerivePIIMaskArgs::from_derive_input(&field).expect("should convert");
        println!("{:?}", parsed_args);

        //
        //
        //
        //assert_eq!(field.faker, MaskType::FirstName);
    }

    #[test]
    fn test_pii_mask_derive_args() {
        let input = r#"
            #[derive(PIIMask)]
            struct Department {
                #[pii_mask(faker = "first_name")]
                name: String,
                courses: Vec<Course>,
            }
        "#;
        let derive_args: DeriveInput = syn::parse_str(input).unwrap();
        assert_eq!(derive_args.ident, "Department");
    }
}
