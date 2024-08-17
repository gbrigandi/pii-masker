extern crate proc_macro;

use std::path::Path;

use ast_grep_config::{DeserializeEnv, SerializableRuleCore};
use ast_grep_core::{language::TSLanguage, Language};
use pii_masker_inspect::DerivePIIMaskArgs;
use serde::Deserialize;

use crate::masker::{Annotation, MaskerMatch, PIIMaskable};
use pii_masker_pii::{MResult, MaskerError};

use darling::FromDeriveInput;
use syn::{parse_str, DeriveInput};

#[derive(Clone, Deserialize, PartialEq, Eq)]
pub enum Rust {
    Rs,
}

impl Language for Rust {
    fn from_path<P: AsRef<Path>>(_path: P) -> Option<Self> {
        Some(Rust::Rs)
    }

    fn get_ts_language(&self) -> TSLanguage {
        tree_sitter_rust::language().into()
    }
}

impl<L: Language> PIIMaskable<L> for Rust {
    fn discover_struct_annotations(
        language: &L,
        source: &str,
    ) -> MResult<Vec<crate::masker::Annotation>> {
        let mut annotations = vec![];
        let env = DeserializeEnv::new(language.clone());
        let ser_rule = ast_grep_config::from_str::<SerializableRuleCore>(
            r#"
{
  "rule": {
    "pattern": "$A",
    "kind": "attribute_item",
    "regex": "PIIMask",
    "precedes": {
      "pattern": "$B",
      "kind": "struct_item",
    }
  }
}"#,
        );
        if let Ok(ser_rule) = ser_rule {
            if let Ok(matcher) = ser_rule.get_matcher(env) {
                let grep = language.ast_grep(source);
                let matches = grep.root().find_all(&matcher);
                for nm in matches {
                    let nm2 = MaskerMatch::from(nm);
                    if let Some(attribute_item) = nm2.env.get("A") {
                        if let Some(struct_item) = nm2.env.get("B") {
                            let attr_and_struct =
                                format!("{} {}", attribute_item.text, struct_item.text);
                            let input =
                                parse_str::<DeriveInput>(&attr_and_struct).expect("should parse");
                            let parsed_args = DerivePIIMaskArgs::from_derive_input(&input)
                                .expect("should convert");

                            let struct_name = parsed_args.ident.to_string();

                            for field in parsed_args.data.take_struct().unwrap().fields {
                                let annotation = Annotation::new(
                                    struct_name.clone(),
                                    field.ident.unwrap().to_string(),
                                    field.faker.to_string(),
                                    field.format,
                                );
                                annotations.push(annotation);
                            }
                        }
                    }
                }
            } else {
                return Err(MaskerError::RuleParseError(
                    "Failed to produce matcher".to_string(),
                ));
            }
        } else {
            return Err(MaskerError::RuleParseError(
                "Failed to parse rule".to_string(),
            ));
        }

        Ok(annotations)
    }

    fn make_struct_annotations_matcher(
        language: L,
    ) -> MResult<ast_grep_config::RuleCore<L>> {
        let env = DeserializeEnv::new(language);
        let ser_rule = ast_grep_config::from_str::<SerializableRuleCore>(
            r#"
rule:
  any:
    - kind: attribute_item
      regex: ^#\[pii_mask
      has:
        kind: attribute
        has:
          kind: identifier
          precedes:
            kind: token_tree
            has:
              kind: identifier
              pattern: $FAKER
      precedes:
        kind: field_declaration
        has:
          kind: field_identifier
          pattern: $FIELD
        inside:
          kind: field_declaration_list
          inside:
            kind: struct_item
            has:
              kind: type_identifier
              pattern: $STRUCT
"#,
        );
        if let Ok(ser_rule) = ser_rule {
            if let Ok(matcher) = ser_rule.get_matcher(env) {
                Ok(matcher)
            } else {
                Err(MaskerError::RuleParseError(
                    "Failed to produce matcher".to_string(),
                ))
            }
        } else {
            Err(MaskerError::RuleParseError(
                "Failed to parse rule".to_string(),
            ))
        }
    }

    fn make_expectations_discovery_matcher(
        language: L,
    ) -> MResult<ast_grep_config::SerializableRuleConfig<L>> {
        let ser_rule = ast_grep_config::from_str::<SerializableRuleCore>(
            r#"
rule:
  any: 
    - any:
        - kind: string_literal
        - kind: integer_literal
      pattern: $VALUE
      inside:
        any:
        - kind: arguments
          inside:
            any:
              - kind: call_expression
                inside:
                  kind: field_initializer
                  has:
                    kind: field_identifier
                    pattern: $FIELD
                  inside:
                    kind: field_initializer_list
                    follows:
                      kind: type_identifier
                      pattern: $STRUCT
                      inside:
                        kind: let_declaration
                        stopBy: end
                        inside:
                          kind: block
                          inside: 
                            kind: function_item
                            regex: test_
                            stopBy: end
        - kind: field_initializer
          has:
            kind: field_identifier
            pattern: $FIELD
          inside:
            kind: field_initializer_list
            follows:
              kind: type_identifier
              pattern: $STRUCT
              inside:
                kind: let_declaration
                stopBy: end
                inside:
                  kind: block
                  inside: 
                    kind: function_item
                    regex: test_
                    stopBy: end
        - kind: field_initializer
          has:
            kind: field_identifier
            pattern: $FIELD
          inside:
            kind: field_initializer_list
            follows:
              kind: type_identifier
              pattern: $STRUCT
              inside:
                kind: let_declaration
                stopBy: end
                inside:
                  kind: block
                  inside: 
                    kind: function_item
                    regex: test_
                    stopBy: end
        - kind: field_expression
          inside:
            kind: call_expression
            inside:
              kind: field_initializer
              has:
                kind: field_identifier
                pattern: $FIELD
              inside:
                kind: field_initializer_list
                follows:
                  kind: type_identifier
                  pattern: $STRUCT
                  inside:
                    kind: let_declaration
                    stopBy: end
                    inside:
                      kind: block
                      inside: 
                        kind: function_item
                        regex: test_
                        stopBy: end
"#,
        );

        if let Ok(ser_rule) = ser_rule {
            Ok(Self::rule_config(language, ser_rule))
        } else {
            Err(MaskerError::RuleParseError(
                "Failed to parse rule".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_literal_writing_to_env() {
        let env = DeserializeEnv::new(Rust::Rs);
        let ser_rule: SerializableRuleCore = ast_grep_config::from_str(
            r#"
            {
  "rule": {
    "pattern": "$A",
    "kind": "string_literal",
    "inside": {
      "kind": "token_tree",
      "inside": {
        "kind": "token_tree",
      }
    }
  }
}"#,
        )
        .expect("should deser");
        let matcher = ser_rule.get_matcher(env).expect("should parse");
        let grep = Rust::Rs.ast_grep("assert_eq!(find_user(\"John\"), User { first_name = \"John\", last_name = \"Voight\", address = Address { street = \"Sesame St\" } })");
        let nm = grep.root().find(&matcher).expect("should match");
        let env = nm.get_env();
        let matched = env.get_match("A").expect("should match C").text();
        assert_eq!(matched, "\"John\"");
    }

    #[test]
    fn test_rust_multiple_literal_writing_to_env() {
        let env = DeserializeEnv::new(Rust::Rs);
        let ser_rule: SerializableRuleCore = ast_grep_config::from_str(
            r#"
            {
  "rule": {
    "pattern": "$A",
    "kind": "identifier",
    "inside": {
      "kind": "token_tree",
      "inside": {
        "kind": "token_tree",
      }
    }
  }
}"#,
        )
        .expect("should deser");
        let matcher = ser_rule.get_matcher(env).expect("should parse");
        let grep = Rust::Rs.ast_grep("assert_eq!(find_user(\"John\"), User { first_name = \"John\", last_name = \"Voight\", address = Address { street = \"Sesame St\" } })");
        let matches = grep.root().find_all(&matcher);
        for nm in matches {
            let env = nm.get_env();
            env.get_match("A").expect("should match A").text();
        }
    }

    #[test]
    fn test_mask_tests() {
        let source = r#"
#[derive(Debug,PIIMask)]
struct Student {
    #[pii_mask(faker="first_name")]
    first_name: String,
    #[pii_mask(faker="last_name")]
    last_name: String,
    #[pii_mask(faker="ssn")]
    ssn: String,
    #[pii_mask(faker="inferred")]
    mobile: String
}

#[cfg(test)]
mod tests {
  user super::*;

  #[test]
  fn test_lookup_student() {
    let expected_student = Student {
        first_name: "John",
        last_name: "Doe",
        ssn: "123-45-6789",
        mobile: "310-444-2211"
    };

    assert_eq!(find_student(100), expected_student);

  }
}
"#;
        let fixture = r#"
student:
  first_name: John
  last_name: Doe
  ssn: 123-45-6789
  mobile: 310-444-2211
"#;

        let masked = Rust::mask_tests(Rust::Rs, source, fixture, 10000);
        assert_eq!(masked.is_ok(), true);
        assert_eq!(masked.as_ref().unwrap().0.contains("John"), false);
        assert_eq!(masked.as_ref().unwrap().0.contains("Doe"), false);
        assert_eq!(masked.as_ref().unwrap().0.contains("123-45-6789"), false);
        assert_eq!(masked.as_ref().unwrap().0.contains("310-444-2211"), false);
    }
}
