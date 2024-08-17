use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::str::FromStr;

use ast_grep_config::{
    DeserializeEnv, GlobalRules, RuleConfig, RuleCore, SerializableRuleConfig,
    SerializableRuleCore, Severity,
};
use ast_grep_core::meta_var::{MetaVarEnv, MetaVariable};
use ast_grep_core::{AstGrep, Language, StrDoc};
use ast_grep_core::{Node as SgNode, NodeMatch as SgNodeMatch};
use pii_masker_pii::MResult;
use regex::Regex;
use serde::{Deserialize, Serialize};

use pii_masker_pii::similarity::Category;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    fields: Vec<FieldConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FieldConfig {
    regex_pattern: String,
    source_code_path: String,
    fixture_file_path: String,
    field_time: String,
}

#[derive(Debug)]
pub struct Annotation {
    struct_name: String,
    field: String,
    faker: String,
    #[allow(dead_code)]
    format: Option<String>,
}

impl Annotation {
    pub fn new(struct_name: String, field: String, faker: String, format: Option<String>) -> Self {
        Self {
            struct_name,
            field,
            faker,
            format,
        }
    }
}

pub struct Expectation<'a, L: Language> {
    struct_name: String,
    field: String,
    value: String,
    nm: NodeMatch<'a, L>,
    #[allow(dead_code)]
    grep: &'a AstGrep<StrDoc<L>>,
    rule_config: RuleConfig<L>,
    serializable_rule_config: SerializableRuleConfig<L>,
}

impl<'a, L: Language> Expectation<'a, L> {
    pub fn new(
        struct_name: String,
        field: String,
        value: String,
        grep: &'a AstGrep<StrDoc<L>>,
        nm: NodeMatch<'a, L>,
        rule_config: RuleConfig<L>,
        serializable_rule_config: SerializableRuleConfig<L>,
    ) -> Self {
        Self {
            struct_name,
            field,
            value,
            grep,
            nm,
            rule_config,
            serializable_rule_config,
        }
    }
}

type Node<'a, L> = SgNode<'a, StrDoc<L>>;
type NodeMatch<'a, L> = SgNodeMatch<'a, StrDoc<L>>;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct MaskerNode {
    pub text: String,
    pub range: (usize, usize, usize, usize),
}

impl<L: Language> From<Node<'_, L>> for MaskerNode {
    fn from(nm: Node<'_, L>) -> Self {
        let start = nm.start_pos();
        let end = nm.end_pos();
        Self {
            text: nm.text().to_string(),
            range: (start.0, start.1, end.0, end.1),
        }
    }
}

pub(crate) struct MaskerMatch<'a, L: Language> {
    #[allow(dead_code)]
    pub node: Node<'a, L>,
    pub env: BTreeMap<String, MaskerNode>,
    #[allow(dead_code)]
    pub nm: NodeMatch<'a, L>,
}

impl<L: Language> MaskerMatch<'_, L> {
    fn env_to_map(env: MetaVarEnv<'_, StrDoc<L>>) -> BTreeMap<String, MaskerNode> {
        let mut map = BTreeMap::new();
        for id in env.get_matched_variables() {
            match id {
                MetaVariable::Capture(name, _) => {
                    if let Some(node) = env.get_match(&name) {
                        map.insert(name, MaskerNode::from(node.clone()));
                    } else if let Some(bytes) = env.get_transformed(&name) {
                        let node = MaskerNode {
                            text: String::from_utf8_lossy(bytes).to_string(),
                            range: (0, 0, 0, 0),
                        };
                        map.insert(name, node);
                    }
                }
                MetaVariable::MultiCapture(name) => {
                    let nodes = env.get_multiple_matches(&name);
                    let (Some(first), Some(last)) = (nodes.first(), nodes.last()) else {
                        continue;
                    };
                    let start = first.start_pos();
                    let end = last.end_pos();

                    let text = nodes.iter().map(|n| n.text()).collect();
                    let node = MaskerNode {
                        text,
                        range: (start.0, start.1, end.0, end.1),
                    };
                    map.insert(name, node);
                }
                // ignore anonymous
                _ => continue,
            }
        }
        map
    }
}

impl<'a, L: Language> From<NodeMatch<'a, L>> for MaskerMatch<'a, L> {
    fn from(nm: NodeMatch<'a, L>) -> MaskerMatch<'a, L> {
        let node = nm.get_node().clone();
        let node = Node::from(node);
        let env = nm.get_env();
        let env = Self::env_to_map((*env).clone());
        Self { node, env, nm }
    }
}

pub trait PIIMaskable<L: Language> {
    fn mask_tests(
        language: L,
        source: &str,
        fixture: &str,
        category_pool_size: usize,
    ) -> MResult<(String, String)> {
        let word_pool = pii_masker_pii::similarity::generate_fake_words_pool(category_pool_size);
        let mut new_source = source.to_string();
        let mut new_fixture = fixture.to_string();
        if let Ok(annotations) = Self::discover_struct_annotations(&language, source) {
            let grep = language.ast_grep(source);
            if let Ok(expectations) = Self::discover_expectations(language, &grep) {
                for expectation in expectations {
                    if let Some(faker) = Self::lookup_faker_for_field(
                        &expectation.struct_name,
                        &expectation.field,
                        &annotations,
                    ) {
                        let fake_value = pii_masker_pii::similarity::sample_similar_word_for_category(
                            &expectation.value,
                            Category::from_str(faker).unwrap(),
                            &word_pool,
                            1,
                        );
                        let mut fake_value_processed =
                            fake_value.iter().next().unwrap().to_string();

                        // if the faked word's length is less than the oringinal one path with
                        // whitespaces
                        // if it's longer, truncate it
                        let expectation_value_len = expectation.value.len();
                        match fake_value_processed.len().cmp(&expectation_value_len) {
                            Ordering::Less => {
                                let whitespaces = " "
                                    .repeat(expectation.value.len() - fake_value_processed.len());
                                fake_value_processed =
                                    format!("{}{}", fake_value_processed, whitespaces);
                            }
                            Ordering::Greater => {
                                fake_value_processed =
                                    fake_value_processed[..expectation.value.len()].to_string();
                            }
                            Ordering::Equal => {}
                        }

                        if fake_value_processed.len() > expectation.value.len() {
                            fake_value_processed =
                                fake_value_processed[..expectation.value.len()].to_string();
                        }

                        let matcher = &expectation.rule_config.matcher;

                        let globals = GlobalRules::default();
                        let mut mutable_serializable_rule_config =
                            expectation.serializable_rule_config.clone();
                        //
                        // Note: if the fake value contains only numeric characters, Serde will not
                        // return the correct type back. By wrapping it in quotes, we can ensure that
                        // the type is correctly inferred.
                        let fake_value_with_quotes = format!("\"{}\"", fake_value_processed);
                        mutable_serializable_rule_config.fix =
                            Some(ast_grep_config::from_str(&fake_value_with_quotes).unwrap());
                        let rule_config = RuleConfig::try_from(
                            mutable_serializable_rule_config.clone(),
                            &globals,
                        )
                        .unwrap();
                        let fixer = &rule_config.get_fixer().unwrap().unwrap();
                        let edit = expectation.nm.make_edit(matcher, fixer);
                        let mut start = 0;
                        let mut new_content = Vec::<char>::new();
                        let src: Vec<_> = new_source.chars().collect();
                        let inserted_text = String::from_utf8(edit.inserted_text).unwrap();
                        let inserted_text = format!("\"{}\"", inserted_text);
                        let inserted_text = inserted_text.chars();
                        new_content.extend(&src[start..edit.position]);
                        new_content.extend(inserted_text);
                        start = edit.position + edit.deleted_length;
                        new_content.extend(&src[start..]);
                        new_source = new_content.iter().collect::<String>();

                        if let Ok(re) = Regex::new(&expectation.value) {
                            new_fixture = re
                                .replace_all(&new_fixture, fake_value_processed)
                                .to_string();
                        }
                    }
                }
            }
        }

        Ok((new_source, new_fixture))
    }

    fn discover_struct_annotations(language: &L, source: &str) -> MResult<Vec<Annotation>> {
        let mut annotations = vec![];
        if let Ok(matcher) = Self::make_struct_annotations_matcher(language.clone()) {
            let grep = language.ast_grep(source);
            let matches = grep.root().find_all(&matcher);
            for nm in matches {
                let nm2 = MaskerMatch::from(nm);
                // check whether the STRUCT, FIELD and MASKER attributes are not null
                if let Some(struct_name) = nm2.env.get("STRUCT") {
                    if let Some(field) = nm2.env.get("FIELD") {
                        if let Some(attribute) = nm2.env.get("FAKER") {
                            let annotation = Annotation::new(
                                struct_name.text.clone(),
                                field.text.clone(),
                                attribute.text.clone(),
                                None,
                            );
                            annotations.push(annotation);
                        }
                    }
                }
            }
        }
        Ok(annotations)
    }

    fn discover_expectations(
        language: L,
        grep: &AstGrep<StrDoc<L>>,
    ) -> MResult<Vec<Expectation<L>>> {
        let mut expectations = vec![];
        if let Ok(serializable_rule_config) = Self::make_expectations_discovery_matcher(language) {
            let env = DeserializeEnv::new(serializable_rule_config.language.clone());
            let globals = GlobalRules::default();
            let rule_config =
                RuleConfig::try_from(serializable_rule_config.clone(), &globals).unwrap();
            let matcher = rule_config.core.get_matcher(env).unwrap();
            let matches = grep.root().find_all(matcher);
            for nm in matches {
                let nm2 = MaskerMatch::from(nm.clone());
                if let Some(struct_name) = nm2.env.get("STRUCT") {
                    if let Some(field) = nm2.env.get("FIELD") {
                        if let Some(value) = nm2.env.get("VALUE") {
                            let rule_config =
                                RuleConfig::try_from(serializable_rule_config.clone(), &globals)
                                    .unwrap();
                            let value = value.text.replace('"', "");
                            let expectation = Expectation::new(
                                struct_name.text.clone(),
                                field.text.clone(),
                                value,
                                grep,
                                nm.clone(),
                                rule_config,
                                serializable_rule_config.clone(),
                            );
                            expectations.push(expectation);
                        }
                    }
                }
            }
        }
        Ok(expectations)
    }

    fn make_struct_annotations_matcher(language: L) -> MResult<RuleCore<L>>;
    fn make_expectations_discovery_matcher(language: L) -> MResult<SerializableRuleConfig<L>>;
    fn lookup_faker_for_field<'a>(
        struct_name: &str,
        field: &str,
        anns: &'a Vec<Annotation>,
    ) -> Option<&'a str> {
        for ann in anns {
            if ann.struct_name == struct_name && ann.field == field {
                return Some(&ann.faker);
            }
        }
        None
    }
    fn rule_config(language: L, core: SerializableRuleCore) -> SerializableRuleConfig<L> {
        SerializableRuleConfig {
            core,
            id: "".into(),
            language,
            rewriters: None,
            message: "".into(),
            note: None,
            severity: Severity::Hint,
            files: None,
            ignores: None,
            url: None,
            metadata: None,
        }
    }
}
